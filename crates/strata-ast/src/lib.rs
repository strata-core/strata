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
        Struct(StructDef),
        Enum(EnumDef),
    }

    /// Struct definition: `struct Point<T> { x: T, y: T }`
    #[derive(Debug, Clone, Serialize)]
    pub struct StructDef {
        pub name: Ident,
        pub type_params: Vec<Ident>,
        pub fields: Vec<Field>,
        pub span: Span,
    }

    /// Field in a struct: `name: Type`
    #[derive(Debug, Clone, Serialize)]
    pub struct Field {
        pub name: Ident,
        pub ty: TypeExpr,
        pub span: Span,
    }

    /// Enum definition: `enum Option<T> { Some(T), None }`
    #[derive(Debug, Clone, Serialize)]
    pub struct EnumDef {
        pub name: Ident,
        pub type_params: Vec<Ident>,
        pub variants: Vec<Variant>,
        pub span: Span,
    }

    /// Enum variant: `Some(T)` or `None`
    #[derive(Debug, Clone, Serialize)]
    pub struct Variant {
        pub name: Ident,
        pub fields: VariantFields,
        pub span: Span,
    }

    /// Variant field types
    #[derive(Debug, Clone, Serialize)]
    pub enum VariantFields {
        /// Unit variant: `None`
        Unit,
        /// Tuple variant: `Some(T, U)`
        Tuple(Vec<TypeExpr>),
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
        /// Simple or qualified path: `Int`, `Option::Some`
        Path(Vec<Ident>, Span),
        /// Function type: `fn(A, B) -> R`
        Arrow {
            params: Vec<TypeExpr>,
            ret: Box<TypeExpr>,
            span: Span,
        },
        /// Generic type application: `Option<T>`, `Result<T, E>`
        App {
            base: Vec<Ident>,
            args: Vec<TypeExpr>,
            span: Span,
        },
        /// Tuple type: `(A, B, C)`
        Tuple(Vec<TypeExpr>, Span),
    }

    impl TypeExpr {
        /// Get the span of this type expression
        pub fn span(&self) -> Span {
            match self {
                TypeExpr::Path(_, span) => *span,
                TypeExpr::Arrow { span, .. } => *span,
                TypeExpr::App { span, .. } => *span,
                TypeExpr::Tuple(_, span) => *span,
            }
        }
    }

    /// Statement within a block
    #[derive(Debug, Clone, Serialize)]
    pub enum Stmt {
        /// Local variable binding: `let x = e;` or `let (a, b) = e;`
        /// Pattern must be irrefutable (use match for refutable patterns)
        Let {
            mutable: bool,
            pat: Pat,
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

    /// Qualified path: `Option::Some`, `Result::Ok`
    #[derive(Debug, Clone, Serialize)]
    pub struct Path {
        pub segments: Vec<Ident>,
        pub span: Span,
    }

    impl Path {
        /// Create a single-segment path
        pub fn single(ident: Ident) -> Self {
            let span = ident.span;
            Self {
                segments: vec![ident],
                span,
            }
        }

        /// Get the full path as a string (e.g., "Option::Some")
        pub fn as_str(&self) -> String {
            self.segments
                .iter()
                .map(|s| s.text.as_str())
                .collect::<Vec<_>>()
                .join("::")
        }
    }

    /// Pattern for match arms and destructuring
    #[derive(Debug, Clone, Serialize)]
    pub enum Pat {
        /// Wildcard pattern: `_`
        Wildcard(Span),
        /// Variable binding: `x`, `_unused`
        Ident(Ident),
        /// Literal pattern: `0`, `true`, `"hello"`
        Literal(Lit, Span),
        /// Tuple pattern: `(a, b)`
        Tuple(Vec<Pat>, Span),
        /// Struct pattern: `Point { x, y: 0 }`
        Struct {
            path: Path,
            fields: Vec<PatField>,
            span: Span,
        },
        /// Variant pattern: `Option::Some(x)`, `Option::None`
        Variant {
            path: Path,
            fields: Vec<Pat>,
            span: Span,
        },
    }

    impl Pat {
        /// Get the span of this pattern
        pub fn span(&self) -> Span {
            match self {
                Pat::Wildcard(span) => *span,
                Pat::Ident(ident) => ident.span,
                Pat::Literal(_, span) => *span,
                Pat::Tuple(_, span) => *span,
                Pat::Struct { span, .. } => *span,
                Pat::Variant { span, .. } => *span,
            }
        }
    }

    /// Field in a struct pattern: `x` or `x: pat`
    #[derive(Debug, Clone, Serialize)]
    pub struct PatField {
        pub name: Ident,
        pub pat: Pat,
        pub span: Span,
    }

    /// Match arm: `Pattern => body`
    #[derive(Debug, Clone, Serialize)]
    pub struct MatchArm {
        pub pat: Pat,
        pub body: Expr,
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
        /// Match expression: `match expr { pat => body, ... }`
        Match {
            scrutinee: Box<Expr>,
            arms: Vec<MatchArm>,
            span: Span,
        },
        /// Tuple expression: `(a, b, c)`
        Tuple {
            elems: Vec<Expr>,
            span: Span,
        },
        /// Struct construction: `Point { x: 1, y: 2 }`
        StructExpr {
            path: Path,
            fields: Vec<FieldInit>,
            span: Span,
        },
        /// Variant construction: `Option::Some(x)`
        /// (Handled as Call on a path, but explicit for clarity in some cases)
        PathExpr(Path),
    }

    /// Field initialization in struct expression: `x: expr` or `x` (shorthand)
    #[derive(Debug, Clone, Serialize)]
    pub struct FieldInit {
        pub name: Ident,
        pub value: Expr,
        pub span: Span,
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
                Expr::Match { span, .. } => *span,
                Expr::Tuple { span, .. } => *span,
                Expr::StructExpr { span, .. } => *span,
                Expr::PathExpr(path) => path.span,
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
