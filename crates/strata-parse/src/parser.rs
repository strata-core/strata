use crate::lexer::Lexer;
use crate::token::{Tok, TokKind};
use anyhow::{bail, Result};
use strata_ast::ast::{
    BinOp, Block, Expr, FnDecl, Ident, Item, LetDecl, Lit, Module, Param, Stmt, TypeExpr, UnOp,
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
            TokKind::KwFn => Ok(Item::Fn(self.parse_fn_decl()?)),
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

        // Parse body block
        let body = self.parse_block()?;
        let body_end = body.span.end;

        Ok(FnDecl {
            name,
            params,
            ret_ty,
            body,
            span: Span {
                start,
                end: body_end,
            },
        })
    }

    fn parse_type(&mut self) -> Result<TypeExpr> {
        let start = self.cur.span.start;

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
            let ret_end = ret.span().end;

            return Ok(TypeExpr::Arrow {
                params,
                ret,
                span: Span {
                    start,
                    end: ret_end,
                },
            });
        }

        // Otherwise, it's a simple path type: Int, Bool, Foo::Bar, etc.
        // Grammar: Ident ('::' Ident)*
        let mut segs = vec![self.parse_ident()?];

        // Parse qualified path: Foo::Bar::Baz
        while matches!(self.cur.kind, TokKind::ColonColon) {
            self.bump(); // consume ::
            segs.push(self.parse_ident()?);
        }

        // Use last segment's span end
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

    /// Parse a let statement: `let [mut] name [: Type] = expr;`
    fn parse_let_stmt(&mut self) -> Result<Stmt> {
        let start = self.cur.span.start;
        self.expect(TokKind::KwLet)?;

        // Check for `mut` keyword
        let mutable = if matches!(self.cur.kind, TokKind::KwMut) {
            self.bump();
            true
        } else {
            false
        };

        let name = self.parse_ident()?;

        // Optional type annotation
        let ty = if matches!(self.cur.kind, TokKind::Colon) {
            self.bump();
            Some(self.parse_type()?)
        } else {
            None
        };

        self.expect(TokKind::Eq)?;
        let value = self.parse_expr_bp(0)?;
        let semi = self.expect(TokKind::Semicolon)?;

        Ok(Stmt::Let {
            mutable,
            name,
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
                let inner = self.parse_expr_bp(100)?;
                self.exit_nesting();
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
                let inner = self.parse_expr_bp(100)?;
                self.exit_nesting();
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
                let id = self.parse_ident()?;
                Ok(Expr::Var(id))
            }

            TokKind::LParen => {
                self.enter_nesting()?;
                let start = tok_span.start;
                self.bump(); // '('
                let inner = self.parse_expr_bp(0)?;
                let end_tok = self.expect(TokKind::RParen)?;
                self.exit_nesting();
                Ok(Expr::Paren {
                    inner: Box::new(inner),
                    span: Span {
                        start,
                        end: end_tok.span.end,
                    },
                })
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

            // Lexer error - surface it with proper context
            TokKind::Error(msg) => bail!("Lexer error at {:?}: {}", self.cur.span, msg),

            _ => bail!("unexpected token in expression: {:?}", tok_kind),
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
    }
}
