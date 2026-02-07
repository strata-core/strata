use crate::lexer::Lexer;
use crate::token::{Tok, TokKind};
use anyhow::{bail, Result};
use strata_ast::ast::{
    BinOp, Block, EnumDef, Expr, ExternFnDecl, Field, FieldInit, FnDecl, Ident, Item, LetDecl, Lit,
    MatchArm, Module, Param, Pat, PatField, Path, Stmt, StructDef, TypeExpr, UnOp, Variant,
    VariantFields,
};
use strata_ast::span::Span;

/// Maximum nesting depth for blocks, ifs, whiles, and nested expressions.
/// This prevents stack overflow from deeply nested input.
const MAX_NESTING_DEPTH: u32 = 512;

pub fn parse_str(_file: &str, src: &str) -> Result<Module> {
    let mut p = Parser::new(src);
    p.parse_module()
}

struct Parser<'a> {
    lex: Lexer<'a>,
    cur: Tok,
    nxt: Tok,
    /// Current nesting depth for blocks/ifs/whiles/exprs
    depth: u32,
}

impl<'a> Parser<'a> {
    fn new(src: &'a str) -> Self {
        let mut lex = Lexer::new(src);
        let cur = lex.next_tok();
        let nxt = lex.next_tok();
        Self {
            lex,
            cur,
            nxt,
            depth: 0,
        }
    }

    /// Increment depth and check limit
    fn enter_nesting(&mut self) -> Result<()> {
        self.depth += 1;
        if self.depth > MAX_NESTING_DEPTH {
            bail!(
                "maximum nesting depth exceeded (limit: {})",
                MAX_NESTING_DEPTH
            );
        }
        Ok(())
    }

    /// Decrement depth
    fn exit_nesting(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    fn bump(&mut self) {
        self.cur = std::mem::replace(&mut self.nxt, self.lex.next_tok());
    }

    /// Check if current token is a lexer error and surface it
    fn check_lex_error(&self) -> Result<()> {
        if let TokKind::Error(msg) = &self.cur.kind {
            bail!("Lexer error at {:?}: {}", self.cur.span, msg);
        }
        Ok(())
    }

    fn at(&self, k: &TokKind) -> bool {
        std::mem::discriminant(&self.cur.kind) == std::mem::discriminant(k)
    }

    fn expect(&mut self, k: TokKind) -> Result<Tok> {
        // Surface lexer errors immediately with proper span
        if let TokKind::Error(msg) = &self.cur.kind {
            bail!("Lexer error at {:?}: {}", self.cur.span, msg);
        }

        if self.at(&k) {
            let t = self.cur.clone();
            self.bump();
            Ok(t)
        } else {
            bail!(
                "expected {:?}, found {:?} at {:?}",
                k,
                self.cur.kind,
                self.cur.span
            )
        }
    }

    // ======= module / items =======

    fn parse_module(&mut self) -> Result<Module> {
        let start = self.cur.span.start;
        let mut items = Vec::new();
        while !matches!(self.cur.kind, TokKind::Eof) {
            // Surface any lexer errors immediately
            self.check_lex_error()?;
            items.push(self.parse_item()?);
        }
        Ok(Module {
            items,
            span: Span {
                start,
                end: self.cur.span.end,
            },
        })
    }

    fn parse_item(&mut self) -> Result<Item> {
        match self.cur.kind {
            TokKind::KwLet => Ok(Item::Let(self.parse_let()?)),
            TokKind::KwExtern => Ok(Item::ExternFn(self.parse_extern_fn()?)),
            TokKind::KwFn => Ok(Item::Fn(self.parse_fn_decl()?)),
            TokKind::KwStruct => Ok(Item::Struct(self.parse_struct_def()?)),
            TokKind::KwEnum => Ok(Item::Enum(self.parse_enum_def()?)),
            _ => bail!("unexpected token at top level: {:?}", self.cur.kind),
        }
    }

    fn parse_ident(&mut self) -> Result<Ident> {
        match &self.cur.kind {
            TokKind::Ident(s) => {
                let span = self.cur.span;
                let id = Ident {
                    text: s.clone(),
                    span,
                };
                self.bump();
                Ok(id)
            }
            _ => bail!("expected identifier, found {:?}", self.cur.kind),
        }
    }

    fn parse_let(&mut self) -> Result<LetDecl> {
        let start = self.cur.span.start;
        self.expect(TokKind::KwLet)?;
        let name = self.parse_ident()?;
        let ty = if matches!(self.cur.kind, TokKind::Colon) {
            self.bump();
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect(TokKind::Eq)?;
        let value = self.parse_expr_bp(0)?;
        let semi = self.expect(TokKind::Semicolon)?;
        Ok(LetDecl {
            name,
            ty,
            value,
            span: Span {
                start,
                end: semi.span.end,
            },
        })
    }

    fn parse_fn_decl(&mut self) -> Result<FnDecl> {
        let start = self.cur.span.start;
        self.expect(TokKind::KwFn)?;
        let name = self.parse_ident()?;

        // Parse parameters: (param1, param2, ...)
        self.expect(TokKind::LParen)?;
        let params = self.parse_params()?;
        self.expect(TokKind::RParen)?;

        // Parse optional return type: -> Type
        let ret_ty = if matches!(self.cur.kind, TokKind::Arrow) {
            self.bump(); // consume ->
            Some(self.parse_type()?)
        } else {
            None
        };

        // Parse optional effect annotation: & { Fs, Net }
        let effects = self.parse_effect_annotation()?;

        // Parse body block
        let body = self.parse_block()?;
        let body_end = body.span.end;

        Ok(FnDecl {
            name,
            params,
            ret_ty,
            effects,
            body,
            span: Span {
                start,
                end: body_end,
            },
        })
    }

    /// Parse an extern function declaration: `extern fn name(params) -> Type & {effects};`
    fn parse_extern_fn(&mut self) -> Result<ExternFnDecl> {
        let start = self.cur.span.start;
        self.expect(TokKind::KwExtern)?;
        self.expect(TokKind::KwFn)?;
        let name = self.parse_ident()?;

        // Parse parameters
        self.expect(TokKind::LParen)?;
        let params = self.parse_params()?;
        self.expect(TokKind::RParen)?;

        // Parse optional return type
        let ret_ty = if matches!(self.cur.kind, TokKind::Arrow) {
            self.bump();
            Some(self.parse_type()?)
        } else {
            None
        };

        // Parse optional effect annotation
        let effects = self.parse_effect_annotation()?;

        // Semicolon-terminated (no body)
        let semi = self.expect(TokKind::Semicolon)?;

        Ok(ExternFnDecl {
            name,
            params,
            ret_ty,
            effects,
            span: Span {
                start,
                end: semi.span.end,
            },
        })
    }

    /// Parse an optional effect annotation: `& { Ident, Ident, ... }`
    ///
    /// Returns `None` if no `&` token is present.
    /// Returns `Some(vec![])` for `& {}` (explicit empty/pure).
    fn parse_effect_annotation(&mut self) -> Result<Option<Vec<Ident>>> {
        if !matches!(self.cur.kind, TokKind::Ampersand) {
            return Ok(None);
        }
        self.bump(); // consume &
        self.expect(TokKind::LBrace)?;

        let mut effects = Vec::new();
        if !matches!(self.cur.kind, TokKind::RBrace) {
            effects.push(self.parse_ident()?);
            while matches!(self.cur.kind, TokKind::Comma) {
                self.bump(); // consume comma
                if matches!(self.cur.kind, TokKind::RBrace) {
                    break; // trailing comma
                }
                effects.push(self.parse_ident()?);
            }
        }

        self.expect(TokKind::RBrace)?;
        Ok(Some(effects))
    }

    /// Parse a struct definition: `struct Name<T, U> { field: Type, ... }`
    fn parse_struct_def(&mut self) -> Result<StructDef> {
        let start = self.cur.span.start;
        self.expect(TokKind::KwStruct)?;
        let name = self.parse_ident()?;

        // Parse optional type parameters: <T, U>
        let type_params = self.parse_type_params()?;

        // Parse fields: { field: Type, ... }
        self.expect(TokKind::LBrace)?;
        let fields = self.parse_struct_fields()?;
        let end_tok = self.expect(TokKind::RBrace)?;

        Ok(StructDef {
            name,
            type_params,
            fields,
            span: Span {
                start,
                end: end_tok.span.end,
            },
        })
    }

    /// Parse optional type parameters: `<T, U>`
    fn parse_type_params(&mut self) -> Result<Vec<Ident>> {
        if !matches!(self.cur.kind, TokKind::Lt) {
            return Ok(vec![]);
        }
        self.bump(); // consume '<'

        let mut params = Vec::new();
        if !matches!(self.cur.kind, TokKind::Gt) {
            params.push(self.parse_ident()?);
            while matches!(self.cur.kind, TokKind::Comma) {
                self.bump();
                params.push(self.parse_ident()?);
            }
        }
        self.expect(TokKind::Gt)?;
        Ok(params)
    }

    /// Parse struct fields: `field: Type, ...`
    fn parse_struct_fields(&mut self) -> Result<Vec<Field>> {
        let mut fields = Vec::new();

        while !matches!(self.cur.kind, TokKind::RBrace) {
            let field_start = self.cur.span.start;
            let name = self.parse_ident()?;
            self.expect(TokKind::Colon)?;
            let ty = self.parse_type()?;
            let ty_end = ty.span().end;

            fields.push(Field {
                name,
                ty,
                span: Span {
                    start: field_start,
                    end: ty_end,
                },
            });

            // Optional trailing comma
            if matches!(self.cur.kind, TokKind::Comma) {
                self.bump();
            } else {
                break;
            }
        }

        Ok(fields)
    }

    /// Parse an enum definition: `enum Name<T> { Variant1, Variant2(T), ... }`
    fn parse_enum_def(&mut self) -> Result<EnumDef> {
        let start = self.cur.span.start;
        self.expect(TokKind::KwEnum)?;
        let name = self.parse_ident()?;

        // Parse optional type parameters
        let type_params = self.parse_type_params()?;

        // Parse variants: { Variant1, Variant2(T), ... }
        self.expect(TokKind::LBrace)?;
        let variants = self.parse_enum_variants()?;
        let end_tok = self.expect(TokKind::RBrace)?;

        Ok(EnumDef {
            name,
            type_params,
            variants,
            span: Span {
                start,
                end: end_tok.span.end,
            },
        })
    }

    /// Parse enum variants: `Variant1, Variant2(T), ...`
    fn parse_enum_variants(&mut self) -> Result<Vec<Variant>> {
        let mut variants = Vec::new();

        while !matches!(self.cur.kind, TokKind::RBrace) {
            let var_start = self.cur.span.start;
            let name = self.parse_ident()?;

            // Check for tuple fields: Variant(T, U)
            let (fields, var_end) = if matches!(self.cur.kind, TokKind::LParen) {
                self.bump(); // consume '('
                let mut tys = Vec::new();
                if !matches!(self.cur.kind, TokKind::RParen) {
                    tys.push(self.parse_type()?);
                    while matches!(self.cur.kind, TokKind::Comma) {
                        self.bump();
                        tys.push(self.parse_type()?);
                    }
                }
                let rparen = self.expect(TokKind::RParen)?;
                (VariantFields::Tuple(tys), rparen.span.end)
            } else {
                (VariantFields::Unit, name.span.end)
            };

            variants.push(Variant {
                name,
                fields,
                span: Span {
                    start: var_start,
                    end: var_end,
                },
            });

            // Optional trailing comma
            if matches!(self.cur.kind, TokKind::Comma) {
                self.bump();
            } else {
                break;
            }
        }

        Ok(variants)
    }

    fn parse_type(&mut self) -> Result<TypeExpr> {
        let start = self.cur.span.start;

        // Check for reference type: &T (but not effect annotation: & {Fs})
        if matches!(self.cur.kind, TokKind::Ampersand) && !matches!(self.nxt.kind, TokKind::LBrace)
        {
            self.bump(); // consume &
            let inner = self.parse_type()?;
            let end = inner.span().end;
            return Ok(TypeExpr::Ref(Box::new(inner), Span { start, end }));
        }

        // Check if it's a function type: fn(T1, T2) -> R
        if matches!(self.cur.kind, TokKind::KwFn) {
            self.bump(); // consume 'fn'
            self.expect(TokKind::LParen)?;

            let mut params = Vec::new();
            if !matches!(self.cur.kind, TokKind::RParen) {
                params.push(self.parse_type()?);
                while matches!(self.cur.kind, TokKind::Comma) {
                    self.bump();
                    params.push(self.parse_type()?);
                }
            }

            self.expect(TokKind::RParen)?;
            self.expect(TokKind::Arrow)?;
            let ret = Box::new(self.parse_type()?);
            let mut end = ret.span().end;

            // Parse optional effect annotation on function type
            let effects = self.parse_effect_annotation()?;
            if let Some(ref effs) = effects {
                if let Some(last) = effs.last() {
                    end = last.span.end;
                }
            }

            return Ok(TypeExpr::Arrow {
                params,
                ret,
                effects,
                span: Span { start, end },
            });
        }

        // Check if it's a tuple type: (A, B, C)
        if matches!(self.cur.kind, TokKind::LParen) {
            self.bump(); // consume '('

            // Empty tuple: ()
            if matches!(self.cur.kind, TokKind::RParen) {
                let end_tok = self.expect(TokKind::RParen)?;
                return Ok(TypeExpr::Tuple(
                    vec![],
                    Span {
                        start,
                        end: end_tok.span.end,
                    },
                ));
            }

            // Parse first element
            let first = self.parse_type()?;

            // Check if it's a single-element parenthesized type or a tuple
            if matches!(self.cur.kind, TokKind::RParen) {
                // Single element in parens - just return the inner type
                // (We don't have 1-tuples)
                self.bump();
                return Ok(first);
            }

            // It's a tuple with multiple elements
            let mut elems = vec![first];
            while matches!(self.cur.kind, TokKind::Comma) {
                self.bump();
                if matches!(self.cur.kind, TokKind::RParen) {
                    break; // trailing comma
                }
                elems.push(self.parse_type()?);
            }
            let end_tok = self.expect(TokKind::RParen)?;

            return Ok(TypeExpr::Tuple(
                elems,
                Span {
                    start,
                    end: end_tok.span.end,
                },
            ));
        }

        // Otherwise, it's a path type (possibly with generic args): Int, Option<T>, Foo::Bar<A, B>
        // Grammar: Ident ('::' Ident)* ('<' Type (',' Type)* '>')?
        let mut segs = vec![self.parse_ident()?];

        // Parse qualified path: Foo::Bar::Baz
        while matches!(self.cur.kind, TokKind::ColonColon) {
            self.bump(); // consume ::
            segs.push(self.parse_ident()?);
        }

        // Check for generic type arguments: <T, U>
        if matches!(self.cur.kind, TokKind::Lt) {
            self.bump(); // consume '<'
            let mut args = Vec::new();
            if !matches!(self.cur.kind, TokKind::Gt) {
                args.push(self.parse_type()?);
                while matches!(self.cur.kind, TokKind::Comma) {
                    self.bump();
                    args.push(self.parse_type()?);
                }
            }
            let end_tok = self.expect(TokKind::Gt)?;

            return Ok(TypeExpr::App {
                base: segs,
                args,
                span: Span {
                    start,
                    end: end_tok.span.end,
                },
            });
        }

        // Simple path without generics
        let last_seg_end = segs.last().map(|s| s.span.end).unwrap_or(start);

        Ok(TypeExpr::Path(
            segs,
            Span {
                start,
                end: last_seg_end,
            },
        ))
    }

    fn parse_params(&mut self) -> Result<Vec<Param>> {
        let mut params = Vec::new();

        // Empty param list: ()
        if matches!(self.cur.kind, TokKind::RParen) {
            return Ok(params);
        }

        // Parse first parameter
        params.push(self.parse_param()?);

        // Parse remaining parameters: , param
        while matches!(self.cur.kind, TokKind::Comma) {
            self.bump(); // consume comma
            params.push(self.parse_param()?);
        }

        Ok(params)
    }

    fn parse_param(&mut self) -> Result<Param> {
        let start = self.cur.span.start;
        let name = self.parse_ident()?;

        // Optional type annotation: : Type
        let (ty, end) = if matches!(self.cur.kind, TokKind::Colon) {
            self.bump(); // consume :
            let type_expr = self.parse_type()?;
            let type_end = type_expr.span().end;
            (Some(type_expr), type_end)
        } else {
            (None, name.span.end)
        };

        Ok(Param {
            name,
            ty,
            span: Span { start, end },
        })
    }

    // ======= match expressions and patterns =======

    /// Parse a match expression: `match expr { pat => body, ... }`
    fn parse_match(&mut self) -> Result<Expr> {
        let start = self.cur.span.start;
        self.expect(TokKind::KwMatch)?;

        let scrutinee = Box::new(self.parse_expr_bp(0)?);

        self.expect(TokKind::LBrace)?;

        let mut arms = Vec::new();
        while !matches!(self.cur.kind, TokKind::RBrace) {
            arms.push(self.parse_match_arm()?);

            // Optional trailing comma
            if matches!(self.cur.kind, TokKind::Comma) {
                self.bump();
            }
        }

        let end_tok = self.expect(TokKind::RBrace)?;

        Ok(Expr::Match {
            scrutinee,
            arms,
            span: Span {
                start,
                end: end_tok.span.end,
            },
        })
    }

    /// Parse a match arm: `pat => body`
    fn parse_match_arm(&mut self) -> Result<MatchArm> {
        let pat = self.parse_pattern()?;
        let pat_start = pat.span().start;

        self.expect(TokKind::FatArrow)?;

        let body = self.parse_expr_bp(0)?;
        let body_end = body.span().end;

        Ok(MatchArm {
            pat,
            body,
            span: Span {
                start: pat_start,
                end: body_end,
            },
        })
    }

    /// Parse a pattern
    fn parse_pattern(&mut self) -> Result<Pat> {
        self.enter_nesting()?;
        let result = self.parse_pattern_inner();
        self.exit_nesting();
        result
    }

    fn parse_pattern_inner(&mut self) -> Result<Pat> {
        let start = self.cur.span.start;

        // Tuple pattern: (a, b)
        if matches!(self.cur.kind, TokKind::LParen) {
            self.bump(); // consume '('

            // Empty tuple: ()
            if matches!(self.cur.kind, TokKind::RParen) {
                let end_tok = self.expect(TokKind::RParen)?;
                return Ok(Pat::Tuple(
                    vec![],
                    Span {
                        start,
                        end: end_tok.span.end,
                    },
                ));
            }

            // Parse first pattern
            let first = self.parse_pattern()?;

            // Check if it's a single-element parenthesized pattern or a tuple
            if matches!(self.cur.kind, TokKind::RParen) {
                // Single element in parens - just return the inner pattern
                self.bump();
                return Ok(first);
            }

            // It's a tuple with multiple elements
            let mut elems = vec![first];
            while matches!(self.cur.kind, TokKind::Comma) {
                self.bump();
                if matches!(self.cur.kind, TokKind::RParen) {
                    break; // trailing comma
                }
                elems.push(self.parse_pattern()?);
            }
            let end_tok = self.expect(TokKind::RParen)?;

            return Ok(Pat::Tuple(
                elems,
                Span {
                    start,
                    end: end_tok.span.end,
                },
            ));
        }

        // Literal patterns: numbers, strings, booleans
        match &self.cur.kind {
            TokKind::Int(v) => {
                let v = *v;
                let span = self.cur.span;
                self.bump();
                return Ok(Pat::Literal(Lit::Int(v), span));
            }
            TokKind::Float(v) => {
                let v = *v;
                let span = self.cur.span;
                self.bump();
                return Ok(Pat::Literal(Lit::Float(v), span));
            }
            TokKind::Str(s) => {
                let s = s.clone();
                let span = self.cur.span;
                self.bump();
                return Ok(Pat::Literal(Lit::Str(s), span));
            }
            TokKind::KwTrue => {
                let span = self.cur.span;
                self.bump();
                return Ok(Pat::Literal(Lit::Bool(true), span));
            }
            TokKind::KwFalse => {
                let span = self.cur.span;
                self.bump();
                return Ok(Pat::Literal(Lit::Bool(false), span));
            }
            TokKind::KwNil => {
                let span = self.cur.span;
                self.bump();
                return Ok(Pat::Literal(Lit::Nil, span));
            }
            _ => {}
        }

        // Identifier patterns: _, x, Foo, Foo::Bar, etc.
        if let TokKind::Ident(s) = &self.cur.kind {
            // Wildcard pattern: _
            if s == "_" {
                let span = self.cur.span;
                self.bump();
                return Ok(Pat::Wildcard(span));
            }

            // Parse path: Foo or Foo::Bar
            let mut segs = vec![self.parse_ident()?];
            while matches!(self.cur.kind, TokKind::ColonColon) {
                self.bump(); // consume ::
                segs.push(self.parse_ident()?);
            }

            let path_end = segs.last().map(|s| s.span.end).unwrap_or(start);
            let path = Path {
                segments: segs.clone(),
                span: Span {
                    start,
                    end: path_end,
                },
            };

            // Check if it's a variant pattern with fields: Option::Some(x)
            if matches!(self.cur.kind, TokKind::LParen) {
                self.bump(); // consume '('

                let mut fields = Vec::new();
                if !matches!(self.cur.kind, TokKind::RParen) {
                    fields.push(self.parse_pattern()?);
                    while matches!(self.cur.kind, TokKind::Comma) {
                        self.bump();
                        if matches!(self.cur.kind, TokKind::RParen) {
                            break; // trailing comma
                        }
                        fields.push(self.parse_pattern()?);
                    }
                }
                let rparen = self.expect(TokKind::RParen)?;

                return Ok(Pat::Variant {
                    path,
                    fields,
                    span: Span {
                        start,
                        end: rparen.span.end,
                    },
                });
            }

            // Check if it's a struct pattern: Point { x, y }
            if matches!(self.cur.kind, TokKind::LBrace) {
                self.bump(); // consume '{'

                let mut fields = Vec::new();
                while !matches!(self.cur.kind, TokKind::RBrace) {
                    let field = self.parse_pat_field()?;
                    fields.push(field);

                    if matches!(self.cur.kind, TokKind::Comma) {
                        self.bump();
                    } else {
                        break;
                    }
                }
                let rbrace = self.expect(TokKind::RBrace)?;

                return Ok(Pat::Struct {
                    path,
                    fields,
                    span: Span {
                        start,
                        end: rbrace.span.end,
                    },
                });
            }

            // If it's a single identifier (not a path), it's a variable binding
            if segs.len() == 1 {
                return Ok(Pat::Ident(segs.into_iter().next().unwrap()));
            }

            // Otherwise it's a unit variant: Option::None
            return Ok(Pat::Variant {
                path,
                fields: vec![],
                span: Span {
                    start,
                    end: path_end,
                },
            });
        }

        bail!("unexpected token in pattern: {:?}", self.cur.kind)
    }

    /// Parse a struct pattern field: `x` or `x: pat`
    fn parse_pat_field(&mut self) -> Result<PatField> {
        let field_start = self.cur.span.start;
        let name = self.parse_ident()?;

        // Check for explicit pattern: x: pat
        let (pat, field_end) = if matches!(self.cur.kind, TokKind::Colon) {
            self.bump(); // consume ':'
            let p = self.parse_pattern()?;
            let end = p.span().end;
            (p, end)
        } else {
            // Shorthand: x means x: x
            let end = name.span.end;
            (Pat::Ident(name.clone()), end)
        };

        Ok(PatField {
            name,
            pat,
            span: Span {
                start: field_start,
                end: field_end,
            },
        })
    }

    /// Parse struct expression: `Point { x: 1, y: 2 }` or `Point { x, y }` (shorthand)
    fn parse_struct_expr(&mut self, path: Path) -> Result<Expr> {
        let start = path.span.start;
        self.expect(TokKind::LBrace)?;

        let mut fields = Vec::new();
        while !matches!(self.cur.kind, TokKind::RBrace) {
            let field_start = self.cur.span.start;
            let name = self.parse_ident()?;

            // Check for explicit value: x: expr
            let (value, field_end) = if matches!(self.cur.kind, TokKind::Colon) {
                self.bump(); // consume ':'
                let expr = self.parse_expr_bp(0)?;
                let end = expr.span().end;
                (expr, end)
            } else {
                // Shorthand: x means x: x
                let end = name.span.end;
                (Expr::Var(name.clone()), end)
            };

            fields.push(FieldInit {
                name,
                value,
                span: Span {
                    start: field_start,
                    end: field_end,
                },
            });

            // Optional comma
            if matches!(self.cur.kind, TokKind::Comma) {
                self.bump();
            } else {
                break;
            }
        }

        let end_tok = self.expect(TokKind::RBrace)?;

        Ok(Expr::StructExpr {
            path,
            fields,
            span: Span {
                start,
                end: end_tok.span.end,
            },
        })
    }

    // ======= blocks and statements =======

    /// Parse a block: `{ stmt* tail? }`
    fn parse_block(&mut self) -> Result<Block> {
        self.enter_nesting()?;
        let result = self.parse_block_inner();
        self.exit_nesting();
        result
    }

    fn parse_block_inner(&mut self) -> Result<Block> {
        let start = self.cur.span.start;
        self.expect(TokKind::LBrace)?;

        let mut stmts = Vec::new();
        let mut tail = None;

        loop {
            // End of block?
            if matches!(self.cur.kind, TokKind::RBrace) {
                break;
            }

            // Try to parse a statement or tail expression
            match self.cur.kind {
                TokKind::KwLet => {
                    stmts.push(self.parse_let_stmt()?);
                }
                TokKind::KwReturn => {
                    stmts.push(self.parse_return_stmt()?);
                }
                _ => {
                    // Parse expression, then determine if it's a statement or tail
                    let expr = self.parse_expr_bp(0)?;
                    let expr_span = Span {
                        start: node_start(&expr),
                        end: node_end(&expr),
                    };

                    if matches!(self.cur.kind, TokKind::Eq) {
                        // Assignment: expr = value;
                        // expr must be a variable
                        let target = match expr {
                            Expr::Var(id) => id,
                            _ => bail!("assignment target must be a variable"),
                        };
                        self.bump(); // consume '='
                        let value = self.parse_expr_bp(0)?;
                        let semi = self.expect(TokKind::Semicolon)?;
                        let span = Span {
                            start: expr_span.start,
                            end: semi.span.end,
                        };
                        stmts.push(Stmt::Assign {
                            target,
                            value,
                            span,
                        });
                    } else if matches!(self.cur.kind, TokKind::Semicolon) {
                        // Expression statement
                        let semi_end = self.cur.span.end;
                        self.bump(); // consume ';'
                        let span = Span {
                            start: expr_span.start,
                            end: semi_end,
                        };
                        stmts.push(Stmt::Expr { expr, span });
                    } else if matches!(self.cur.kind, TokKind::RBrace) {
                        // Tail expression (no semicolon before closing brace)
                        tail = Some(Box::new(expr));
                        break;
                    } else {
                        bail!(
                            "expected ';', '=', or '}}' after expression, found {:?}",
                            self.cur.kind
                        );
                    }
                }
            }
        }

        let end_tok = self.expect(TokKind::RBrace)?;
        Ok(Block {
            stmts,
            tail,
            span: Span {
                start,
                end: end_tok.span.end,
            },
        })
    }

    /// Parse a let statement: `let [mut] pattern [: Type] = expr;`
    /// Supports destructuring patterns like `let (a, b) = expr;`
    fn parse_let_stmt(&mut self) -> Result<Stmt> {
        let start = self.cur.span.start;
        self.expect(TokKind::KwLet)?;

        // Check for `mut` keyword (only valid for simple identifier patterns)
        let mutable = if matches!(self.cur.kind, TokKind::KwMut) {
            self.bump();
            true
        } else {
            false
        };

        // Parse the pattern
        let pat = self.parse_pattern()?;

        // Optional type annotation (only valid for simple identifier patterns)
        let ty = if matches!(self.cur.kind, TokKind::Colon) {
            // Type annotations only allowed for simple identifier patterns
            if !matches!(pat, Pat::Ident(_)) {
                bail!("type annotations not supported for destructuring patterns");
            }
            self.bump();
            Some(self.parse_type()?)
        } else {
            None
        };

        // `mut` only valid for simple identifier patterns
        if mutable && !matches!(pat, Pat::Ident(_)) {
            bail!("`mut` not supported for destructuring patterns");
        }

        self.expect(TokKind::Eq)?;
        let value = self.parse_expr_bp(0)?;
        let semi = self.expect(TokKind::Semicolon)?;

        Ok(Stmt::Let {
            mutable,
            pat,
            ty,
            value,
            span: Span {
                start,
                end: semi.span.end,
            },
        })
    }

    /// Parse a return statement: `return [expr];`
    fn parse_return_stmt(&mut self) -> Result<Stmt> {
        let start = self.cur.span.start;
        self.expect(TokKind::KwReturn)?;

        // Optional return value
        let value = if matches!(self.cur.kind, TokKind::Semicolon) {
            None
        } else {
            Some(self.parse_expr_bp(0)?)
        };

        let semi = self.expect(TokKind::Semicolon)?;

        Ok(Stmt::Return {
            value,
            span: Span {
                start,
                end: semi.span.end,
            },
        })
    }

    /// Parse an if expression: `if cond { } [else { }]` or `if cond { } else if cond2 { } else { }`
    fn parse_if(&mut self) -> Result<Expr> {
        let start = self.cur.span.start;
        self.expect(TokKind::KwIf)?;

        let cond = Box::new(self.parse_expr_bp(0)?);
        let then_ = self.parse_block()?;
        let then_end = then_.span.end;

        // Check for else clause
        let (else_, end) = if matches!(self.cur.kind, TokKind::KwElse) {
            self.bump(); // consume 'else'

            if matches!(self.cur.kind, TokKind::KwIf) {
                // else if: parse as nested if expression (with nesting guard)
                self.enter_nesting()?;
                let else_if = self.parse_if();
                self.exit_nesting();
                let else_if = else_if?;
                let else_end = node_end(&else_if);
                (Some(Box::new(else_if)), else_end)
            } else {
                // else block
                let else_block = self.parse_block()?;
                let else_end = else_block.span.end;
                (Some(Box::new(Expr::Block(else_block))), else_end)
            }
        } else {
            (None, then_end)
        };

        let span = Span { start, end };

        Ok(Expr::If {
            cond,
            then_,
            else_,
            span,
        })
    }

    /// Parse a while loop: `while cond { body }`
    fn parse_while(&mut self) -> Result<Expr> {
        let start = self.cur.span.start;
        self.expect(TokKind::KwWhile)?;

        let cond = Box::new(self.parse_expr_bp(0)?);
        let body = self.parse_block()?;
        let body_end = body.span.end;

        let span = Span {
            start,
            end: body_end,
        };

        Ok(Expr::While { cond, body, span })
    }

    // ======= expressions (Pratt parser) =======
    //
    // Precedence (low -> high):
    //   1:  ||
    //   3:  &&
    //   5:  == !=
    //   7:  < <= > >=
    //   10: + -
    //   20: * /
    // prefix (unary) binds tighter than all infix; we give it rbp = 100

    fn parse_expr_bp(&mut self, min_bp: u8) -> Result<Expr> {
        // prefix: literals, vars, (), unary ! and -
        let mut lhs = self.parse_prefix()?;

        loop {
            let (op, lbp, rbp) = match self.cur.kind {
                // logical
                TokKind::OrOr => (BinOp::Or, 1, 2),
                TokKind::AndAnd => (BinOp::And, 3, 4),
                // equality
                TokKind::EqEq => (BinOp::Eq, 5, 6),
                TokKind::BangEq => (BinOp::Ne, 5, 6),
                // relational
                TokKind::Lt => (BinOp::Lt, 7, 8),
                TokKind::Le => (BinOp::Le, 7, 8),
                TokKind::Gt => (BinOp::Gt, 7, 8),
                TokKind::Ge => (BinOp::Ge, 7, 8),
                // arithmetic
                TokKind::Plus => (BinOp::Add, 10, 11),
                TokKind::Minus => (BinOp::Sub, 10, 11),
                TokKind::Star => (BinOp::Mul, 20, 21),
                TokKind::Slash => (BinOp::Div, 20, 21),
                // call application (tightest)
                TokKind::LParen => {
                    let start = node_start(&lhs);
                    let (args, rparen_end) = self.parse_call_args()?;
                    let span = Span {
                        start,
                        end: rparen_end,
                    };
                    lhs = Expr::Call {
                        callee: Box::new(lhs),
                        args,
                        span,
                    };
                    continue;
                }
                _ => break,
            };

            if lbp < min_bp {
                break;
            }
            self.bump(); // consume operator
            let rhs = self.parse_expr_bp(rbp)?;
            let span = Span {
                start: node_start(&lhs),
                end: node_end(&rhs),
            };
            lhs = Expr::Binary {
                lhs: Box::new(lhs),
                op,
                rhs: Box::new(rhs),
                span,
            };
        }

        Ok(lhs)
    }

    fn parse_prefix(&mut self) -> Result<Expr> {
        // Snapshot current token to avoid borrow issues when bumping
        let tok_kind = self.cur.kind.clone();
        let tok_span = self.cur.span;

        match tok_kind {
            // unary prefix
            TokKind::Bang => {
                self.enter_nesting()?;
                self.bump();
                let result = self.parse_expr_bp(100);
                self.exit_nesting();
                let inner = result?;
                let span = Span {
                    start: tok_span.start,
                    end: node_end(&inner),
                };
                Ok(Expr::Unary {
                    op: UnOp::Not,
                    expr: Box::new(inner),
                    span,
                })
            }
            TokKind::Minus => {
                self.enter_nesting()?;
                self.bump();
                let result = self.parse_expr_bp(100);
                self.exit_nesting();
                let inner = result?;
                let span = Span {
                    start: tok_span.start,
                    end: node_end(&inner),
                };
                Ok(Expr::Unary {
                    op: UnOp::Neg,
                    expr: Box::new(inner),
                    span,
                })
            }

            // Borrow expression: &expr
            TokKind::Ampersand => {
                self.enter_nesting()?;
                self.bump();
                let result = self.parse_expr_bp(100);
                self.exit_nesting();
                let inner = result?;
                let span = Span {
                    start: tok_span.start,
                    end: node_end(&inner),
                };
                Ok(Expr::Borrow(Box::new(inner), span))
            }

            // primaries
            TokKind::Int(v) => {
                self.bump();
                Ok(Expr::Lit(Lit::Int(v), tok_span))
            }
            TokKind::Float(v) => {
                self.bump();
                Ok(Expr::Lit(Lit::Float(v), tok_span))
            }
            TokKind::Str(s) => {
                self.bump();
                Ok(Expr::Lit(Lit::Str(s), tok_span))
            }
            TokKind::KwTrue => {
                self.bump();
                Ok(Expr::Lit(Lit::Bool(true), tok_span))
            }
            TokKind::KwFalse => {
                self.bump();
                Ok(Expr::Lit(Lit::Bool(false), tok_span))
            }
            TokKind::KwNil => {
                self.bump();
                Ok(Expr::Lit(Lit::Nil, tok_span))
            }

            TokKind::Ident(_) => {
                let start = tok_span.start;
                let first_id = self.parse_ident()?;

                // Check for qualified path: Foo::Bar
                if matches!(self.cur.kind, TokKind::ColonColon) {
                    let mut segs = vec![first_id];
                    while matches!(self.cur.kind, TokKind::ColonColon) {
                        self.bump(); // consume ::
                        segs.push(self.parse_ident()?);
                    }

                    let path_end = segs.last().map(|s| s.span.end).unwrap_or(start);
                    let path = Path {
                        segments: segs,
                        span: Span {
                            start,
                            end: path_end,
                        },
                    };

                    // Check if it's a struct construction: Foo::Bar { ... }
                    // Only allow struct expressions for qualified paths to avoid ambiguity
                    // with blocks (e.g., `if x { }` vs `x { field: val }`)
                    if matches!(self.cur.kind, TokKind::LBrace) {
                        return self.parse_struct_expr(path);
                    }

                    // Otherwise it's a path expression (e.g., enum constructor)
                    return Ok(Expr::PathExpr(path));
                }

                // For unqualified identifiers, check for struct expression using heuristic:
                // If the identifier starts with uppercase (type name convention) and
                // is followed by `{`, parse as struct expression.
                // This avoids ambiguity with blocks like `if x { }` since those use
                // lowercase variable names.
                if matches!(self.cur.kind, TokKind::LBrace) {
                    let is_type_name = first_id
                        .text
                        .chars()
                        .next()
                        .map(|c| c.is_ascii_uppercase())
                        .unwrap_or(false);
                    if is_type_name {
                        let path = Path {
                            segments: vec![first_id],
                            span: Span {
                                start,
                                end: start + 1, // Will be updated by parse_struct_expr
                            },
                        };
                        return self.parse_struct_expr(path);
                    }
                }

                // Simple variable
                Ok(Expr::Var(first_id))
            }

            TokKind::LParen => {
                self.enter_nesting()?;
                let result = self.parse_paren_or_tuple(tok_span.start);
                self.exit_nesting();
                result
            }

            // Block expression
            TokKind::LBrace => {
                let block = self.parse_block()?;
                Ok(Expr::Block(block))
            }

            // If expression
            TokKind::KwIf => {
                self.enter_nesting()?;
                let e = self.parse_if();
                self.exit_nesting();
                e
            }

            // While loop
            TokKind::KwWhile => {
                self.enter_nesting()?;
                let e = self.parse_while();
                self.exit_nesting();
                e
            }

            // Match expression
            TokKind::KwMatch => {
                self.enter_nesting()?;
                let e = self.parse_match();
                self.exit_nesting();
                e
            }

            // Lexer error - surface it with proper context
            TokKind::Error(msg) => bail!("Lexer error at {:?}: {}", self.cur.span, msg),

            _ => bail!("unexpected token in expression: {:?}", tok_kind),
        }
    }

    /// Parse parenthesized expression or tuple. Called after '(' is consumed.
    /// Nesting depth is managed by the caller.
    fn parse_paren_or_tuple(&mut self, start: u32) -> Result<Expr> {
        self.bump(); // '('

        // Empty tuple: ()
        if matches!(self.cur.kind, TokKind::RParen) {
            let end_tok = self.expect(TokKind::RParen)?;
            return Ok(Expr::Tuple {
                elems: vec![],
                span: Span {
                    start,
                    end: end_tok.span.end,
                },
            });
        }

        // Parse first expression
        let first = self.parse_expr_bp(0)?;

        // Check if it's a tuple or parenthesized expression
        if matches!(self.cur.kind, TokKind::Comma) {
            // It's a tuple
            let mut elems = vec![first];
            while matches!(self.cur.kind, TokKind::Comma) {
                self.bump();
                if matches!(self.cur.kind, TokKind::RParen) {
                    break; // trailing comma
                }
                elems.push(self.parse_expr_bp(0)?);
            }
            let end_tok = self.expect(TokKind::RParen)?;
            Ok(Expr::Tuple {
                elems,
                span: Span {
                    start,
                    end: end_tok.span.end,
                },
            })
        } else {
            // Parenthesized expression
            let end_tok = self.expect(TokKind::RParen)?;
            Ok(Expr::Paren {
                inner: Box::new(first),
                span: Span {
                    start,
                    end: end_tok.span.end,
                },
            })
        }
    }

    /// Parse call arguments and return (args, closing_paren_span_end)
    fn parse_call_args(&mut self) -> Result<(Vec<Expr>, u32)> {
        self.expect(TokKind::LParen)?; // we are at '('
        let mut args = Vec::new();
        if !matches!(self.cur.kind, TokKind::RParen) {
            loop {
                args.push(self.parse_expr_bp(0)?);
                if matches!(self.cur.kind, TokKind::Comma) {
                    self.bump();
                    continue;
                }
                break;
            }
        }
        let rparen = self.expect(TokKind::RParen)?;
        Ok((args, rparen.span.end))
    }
}

// ======= span helpers =======

fn node_start(e: &Expr) -> u32 {
    match e {
        Expr::Lit(_, sp) => sp.start,
        Expr::Var(id) => id.span.start,
        Expr::Unary { span, .. } => span.start,
        Expr::Call { span, .. } => span.start,
        Expr::Binary { span, .. } => span.start,
        Expr::Paren { span, .. } => span.start,
        Expr::Block(block) => block.span.start,
        Expr::If { span, .. } => span.start,
        Expr::While { span, .. } => span.start,
        Expr::Match { span, .. } => span.start,
        Expr::Tuple { span, .. } => span.start,
        Expr::StructExpr { span, .. } => span.start,
        Expr::PathExpr(path) => path.span.start,
        Expr::Borrow(_, span) => span.start,
    }
}

fn node_end(e: &Expr) -> u32 {
    match e {
        Expr::Lit(_, sp) => sp.end,
        Expr::Var(id) => id.span.end,
        Expr::Unary { span, .. } => span.end,
        Expr::Call { span, .. } => span.end,
        Expr::Binary { span, .. } => span.end,
        Expr::Paren { span, .. } => span.end,
        Expr::Block(block) => block.span.end,
        Expr::If { span, .. } => span.end,
        Expr::While { span, .. } => span.end,
        Expr::Match { span, .. } => span.end,
        Expr::Tuple { span, .. } => span.end,
        Expr::StructExpr { span, .. } => span.end,
        Expr::PathExpr(path) => path.span.end,
        Expr::Borrow(_, span) => span.end,
    }
}
