use strata_ast::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum TokKind {
    // trivia / eof / error
    Eof,
    /// Error token (e.g., token limit exceeded)
    Error(String),
    // punctuation
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Colon,
    ColonColon, // :: for namespaced paths (ADT support)
    Semicolon,
    Arrow,    // -> for function return types
    FatArrow, // => for pattern matching (ADT support)
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
    KwMatch,  // match keyword (ADT support)
    KwEnum,   // enum keyword (ADT support)
    KwStruct, // struct keyword (ADT support)
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
