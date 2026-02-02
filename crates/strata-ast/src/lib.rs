pub mod span {
    use serde::Serialize;

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
    pub struct Span {
        pub start: u32,
        pub end: u32,
    }
}

pub mod ast {
    use super::span::Span;
    use serde::Serialize;

    #[derive(Debug, Serialize)]
    pub struct Module {
        pub items: Vec<Item>,
        pub span: Span,
    }

    #[derive(Debug, Serialize)]
    pub enum Item {
        Let(LetDecl),
        Fn(FnDecl),
    }

    #[derive(Debug, Serialize)]
    pub struct FnDecl {
        pub name: Ident,
        pub params: Vec<Param>,
        pub ret_ty: Option<TypeExpr>,
        pub body: Expr, // Single expression for now (blocks in Issue 006)
        pub span: Span,
    }

    #[derive(Debug, Serialize)]
    pub struct Param {
        pub name: Ident,
        pub ty: Option<TypeExpr>,
        pub span: Span,
    }

    #[derive(Debug, Serialize)]
    pub struct LetDecl {
        pub name: Ident,
        pub ty: Option<TypeExpr>,
        pub value: Expr,
        pub span: Span,
    }

    #[derive(Debug, Clone, Serialize)]
    pub struct Ident {
        pub text: String,
        pub span: Span,
    }

    #[derive(Debug, Clone, Serialize)]
    pub enum TypeExpr {
        Path(Vec<Ident>, Span),
        Arrow {
            // fn(A, B, C) -> R
            params: Vec<TypeExpr>,
            ret: Box<TypeExpr>,
            span: Span,
        },
    }

    #[derive(Debug, Clone, Serialize)]
    pub enum Expr {
        Lit(Lit, Span),
        Var(Ident),
        Unary {
            op: UnOp,
            expr: Box<Expr>,
            span: Span,
        },
        Call {
            callee: Box<Expr>,
            args: Vec<Expr>,
            span: Span,
        },
        Binary {
            lhs: Box<Expr>,
            op: BinOp,
            rhs: Box<Expr>,
            span: Span,
        },
        Paren {
            inner: Box<Expr>,
            span: Span,
        },
    }

    #[derive(Debug, Clone, Copy, Serialize)]
    pub enum UnOp {
        Not,
        Neg,
    }

    #[derive(Debug, Clone, Serialize)]
    pub enum Lit {
        Int(i64),
        Float(f64),
        Str(String),
        Bool(bool),
        Nil,
    }

    #[derive(Debug, Clone, Copy, Serialize)]
    pub enum BinOp {
        // logical
        Or,
        And,
        // equality
        Eq,
        Ne,
        // relational
        Lt,
        Le,
        Gt,
        Ge,
        // arithmetic
        Add,
        Sub,
        Mul,
        Div,
    }
}
