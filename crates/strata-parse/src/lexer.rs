use crate::token::{Tok, TokKind};
use strata_ast::span::Span;

pub struct Lexer<'a> {
    src: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            src: src.as_bytes(),
            pos: 0,
        }
    }

    fn bump(&mut self) -> Option<u8> {
        if self.pos >= self.src.len() {
            None
        } else {
            let b = self.src[self.pos];
            self.pos += 1;
            Some(b)
        }
    }

    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }
    fn peek2(&self) -> Option<u8> {
        self.src.get(self.pos + 1).copied()
    }

    fn span(&self, start: usize) -> Span {
        Span {
            start: start as u32,
            end: self.pos as u32,
        }
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            while matches!(self.peek(), Some(b) if (b as char).is_whitespace()) {
                self.bump();
            }
            // line comment: //
            if self.peek() == Some(b'/') && self.peek2() == Some(b'/') {
                self.bump();
                self.bump();
                while let Some(b) = self.peek() {
                    if b == b'\n' {
                        break;
                    }
                    self.bump();
                }
                continue;
            }
            break;
        }
    }

    pub fn next_tok(&mut self) -> Tok {
        self.skip_ws_and_comments();
        let start = self.pos;
        let Some(b) = self.bump() else {
            return Tok {
                kind: TokKind::Eof,
                span: Span {
                    start: self.pos as u32,
                    end: self.pos as u32,
                },
            };
        };
        let c = b as char;

        // 2-char operators first
        if c == '&' && self.peek() == Some(b'&') {
            self.bump();
            return Tok {
                kind: TokKind::AndAnd,
                span: self.span(start),
            };
        }
        if c == '|' && self.peek() == Some(b'|') {
            self.bump();
            return Tok {
                kind: TokKind::OrOr,
                span: self.span(start),
            };
        }
        if c == '=' && self.peek() == Some(b'=') {
            self.bump();
            return Tok {
                kind: TokKind::EqEq,
                span: self.span(start),
            };
        }
        if c == '!' && self.peek() == Some(b'=') {
            self.bump();
            return Tok {
                kind: TokKind::BangEq,
                span: self.span(start),
            };
        }
        if c == '<' && self.peek() == Some(b'=') {
            self.bump();
            return Tok {
                kind: TokKind::Le,
                span: self.span(start),
            };
        }
        if c == '>' && self.peek() == Some(b'=') {
            self.bump();
            return Tok {
                kind: TokKind::Ge,
                span: self.span(start),
            };
        }
        // Arrow: ->
        if c == '-' && self.peek() == Some(b'>') {
            self.bump();
            return Tok {
                kind: TokKind::Arrow,
                span: self.span(start),
            };
        }

        // 1-char punctuation/operators
        let single = match c {
            '(' => Some(TokKind::LParen),
            ')' => Some(TokKind::RParen),
            '{' => Some(TokKind::LBrace),
            '}' => Some(TokKind::RBrace),
            ',' => Some(TokKind::Comma),
            ':' => Some(TokKind::Colon),
            ';' => Some(TokKind::Semicolon),
            '+' => Some(TokKind::Plus),
            '-' => Some(TokKind::Minus),
            '*' => Some(TokKind::Star),
            '/' => Some(TokKind::Slash),
            '=' => Some(TokKind::Eq),
            '<' => Some(TokKind::Lt),
            '>' => Some(TokKind::Gt),
            '!' => Some(TokKind::Bang), // <-- single '!'
            _ => None,
        };
        if let Some(k) = single {
            return Tok {
                kind: k,
                span: self.span(start),
            };
        }

        // string
        if c == '"' {
            let mut s = String::new();
            while let Some(b) = self.peek() {
                self.bump();
                let ch = b as char;
                if ch == '"' {
                    break;
                }
                if ch == '\\' {
                    let Some(esc) = self.bump().map(|x| x as char) else {
                        break;
                    };
                    let real = match esc {
                        'n' => '\n',
                        't' => '\t',
                        '"' => '"',
                        '\\' => '\\',
                        _ => esc,
                    };
                    s.push(real);
                } else {
                    s.push(ch);
                }
            }
            return Tok {
                kind: TokKind::Str(s),
                span: self.span(start),
            };
        }

        // number (int/float)
        if c.is_ascii_digit() {
            let mut s = String::from(c);
            let mut dot = false;
            while let Some(p) = self.peek() {
                let ch = p as char;
                if ch.is_ascii_digit() {
                    s.push(ch);
                    self.bump();
                } else if ch == '.' && !dot {
                    dot = true;
                    s.push('.');
                    self.bump();
                } else {
                    break;
                }
            }
            if dot {
                return Tok {
                    kind: TokKind::Float(s.parse().unwrap()),
                    span: self.span(start),
                };
            } else {
                return Tok {
                    kind: TokKind::Int(s.parse().unwrap()),
                    span: self.span(start),
                };
            }
        }

        // ident / keywords
        if c.is_ascii_alphabetic() || c == '_' {
            let mut s = String::from(c);
            while let Some(p) = self.peek() {
                let ch = p as char;
                if ch.is_ascii_alphanumeric() || ch == '_' {
                    s.push(ch);
                    self.bump();
                } else {
                    break;
                }
            }
            let kind = match s.as_str() {
                "let" => TokKind::KwLet,
                "fn" => TokKind::KwFn,
                "true" => TokKind::KwTrue,
                "false" => TokKind::KwFalse,
                "nil" => TokKind::KwNil,
                "if" => TokKind::KwIf,
                "else" => TokKind::KwElse,
                "while" => TokKind::KwWhile,
                "return" => TokKind::KwReturn,
                "mut" => TokKind::KwMut,
                _ => TokKind::Ident(s),
            };
            return Tok {
                kind,
                span: self.span(start),
            };
        }

        // fallback
        Tok {
            kind: TokKind::Eof,
            span: self.span(start),
        }
    }
}
