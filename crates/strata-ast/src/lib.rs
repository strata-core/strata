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
        pub body: Block,
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

    impl TypeExpr {
        /// Get the span of this type expression
        pub fn span(&self) -> Span {
            match self {
                TypeExpr::Path(_, span) => *span,
                TypeExpr::Arrow { span, .. } => *span,
            }
        }
    }

    /// Statement within a block
    #[derive(Debug, Clone, Serialize)]
    pub enum Stmt {
        /// Local variable binding: `let x = e;` or `let mut x: T = e;`
        Let {
            mutable: bool,
            name: Ident,
            ty: Option<TypeExpr>,
            value: Expr,
            span: Span,
        },
        /// Assignment: `x = e;`
        Assign {
            target: Ident,
            value: Expr,
            span: Span,
        },
        /// Expression statement: `e;`
        Expr { expr: Expr, span: Span },
        /// Return statement: `return e;` or `return;`
        Return { value: Option<Expr>, span: Span },
    }

    /// Block expression: `{ stmt; stmt; expr }`
    #[derive(Debug, Clone, Serialize)]
    pub struct Block {
        /// Statements in the block (with trailing semicolons)
        pub stmts: Vec<Stmt>,
        /// Optional tail expression (no trailing semicolon) - the block's value
        pub tail: Option<Box<Expr>>,
        pub span: Span,
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
        /// Block expression: `{ stmt; stmt; expr }`
        Block(Block),
        /// If expression: `if cond { ... } else { ... }`
        If {
            cond: Box<Expr>,
            then_: Block,
            else_: Option<Box<Expr>>,
            span: Span,
        },
        /// While loop: `while cond { ... }`
        While {
            cond: Box<Expr>,
            body: Block,
            span: Span,
        },
    }

    impl Expr {
        /// Get the span of this expression
        pub fn span(&self) -> Span {
            match self {
                Expr::Lit(_, span) => *span,
                Expr::Var(ident) => ident.span,
                Expr::Unary { span, .. } => *span,
                Expr::Call { span, .. } => *span,
                Expr::Binary { span, .. } => *span,
                Expr::Paren { span, .. } => *span,
                Expr::Block(block) => block.span,
                Expr::If { span, .. } => *span,
                Expr::While { span, .. } => *span,
            }
        }
    }

    #[derive(Debug, Clone, Copy, Serialize)]
    pub enum UnOp {
        Not,
        Neg,
    }

    /// Literal values in the source code
    #[derive(Debug, Clone, Serialize)]
    pub enum Lit {
        Int(i64),
        Float(f64),
        Str(String),
        Bool(bool),
        /// The `nil` literal, which has type `Unit`.
        /// Note: `Nil` is the AST representation; in the type system this becomes `Ty::Const(TyConst::Unit)`.
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
