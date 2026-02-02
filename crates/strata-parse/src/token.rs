use strata_ast::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum TokKind {
    // trivia / eof
    Eof,
    // punctuation
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Colon,
    Semicolon,
    Arrow,
    // assignment
    Eq,
    // arithmetic
    Plus,
    Minus,
    Star,
    Slash,
    // equality
    EqEq,
    BangEq,
    // relational
    Lt,
    Le,
    Gt,
    Ge,
    // logical
    AndAnd,
    OrOr,
    // unary
    Bang, // <-- needed for '!'
    // idents / keywords
    Ident(String),
    KwLet,
    KwFn,
    KwTrue,
    KwFalse,
    KwNil,
    KwIf,
    KwElse,
    KwWhile,
    KwReturn,
    KwMut,
    // literals
    Int(i64),
    Float(f64),
    Str(String),
}

#[derive(Debug, Clone)]
pub struct Tok {
    pub kind: TokKind,
    pub span: Span,
}
