use anyhow::{bail, Result};
use clap::{Parser, ValueEnum};
use strata_ast::ast::*;
use strata_parse::parse_str;
use strata_types::TypeChecker;

#[derive(ValueEnum, Clone, Debug)]
enum Format {
    Pretty,
    Json,
}

#[derive(Parser, Debug)]
#[command(name = "strata-cli")]
#[command(about = "Strata compiler CLI (Issue 001: Parser & AST)")]
struct Cli {
    /// Evaluate each `let` and print results instead of dumping the AST
    #[arg(long)]
    eval: bool,

    /// Output format when not using --eval
    #[arg(long, value_enum, default_value_t = Format::Pretty)]
    format: Format,

    /// Path to a .strata file
    path: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let src = std::fs::read_to_string(&cli.path)?;
    let module = parse_str(&cli.path, &src)?;

    let mut type_checker = TypeChecker::new();
    if let Err(e) = type_checker.check_module(&module) {
        eprintln!("Type error: {}", e);
        std::process::exit(1);
    }

    if cli.eval {
        eval_module(&module)?;
        return Ok(());
    }

    match cli.format {
        Format::Pretty => println!("{:#?}", module),
        Format::Json => println!("{}", serde_json::to_string_pretty(&module)?),
    }
    Ok(())
}

// ======== Tiny evaluator ========

#[derive(Debug, Clone)]
enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Nil,
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(v) => write!(f, "{v}"),
            Value::Float(v) => write!(f, "{v}"),
            Value::Bool(v) => write!(f, "{v}"),
            Value::Str(s) => write!(f, "\"{s}\""),
            Value::Nil => write!(f, "nil"),
        }
    }
}

fn eval_module(m: &Module) -> Result<()> {
    for item in &m.items {
        match item {
            Item::Let(ld) => {
                let v = eval_expr(&ld.value)?;
                println!("{} = {}", ld.name.text, v);
            }
            Item::Fn(_) => {
                // TODO: Implement function definitions in evaluator (Issue 005 Phase 7)
                // For now, just skip function declarations
            }
        }
    }
    Ok(())
}

fn eval_expr(e: &Expr) -> Result<Value> {
    match e {
        Expr::Lit(Lit::Int(v), _) => Ok(Value::Int(*v)),
        Expr::Lit(Lit::Float(v), _) => Ok(Value::Float(*v)),
        Expr::Lit(Lit::Bool(b), _) => Ok(Value::Bool(*b)),
        Expr::Lit(Lit::Str(s), _) => Ok(Value::Str(s.clone())),
        Expr::Lit(Lit::Nil, _) => Ok(Value::Nil),

        Expr::Unary { op, expr, .. } => {
            let v = eval_expr(expr)?;
            match (op, v) {
                (UnOp::Not, Value::Bool(b)) => Ok(Value::Bool(!b)),
                (UnOp::Neg, Value::Int(i)) => Ok(Value::Int(-i)),
                (UnOp::Neg, Value::Float(f)) => Ok(Value::Float(-f)),
                (UnOp::Not, _) => bail!("`!` expects Bool"),
                (UnOp::Neg, _) => bail!("unary `-` expects Int or Float"),
            }
        }

        Expr::Binary { lhs, op, rhs, .. } => {
            use BinOp::*;
            match op {
                Add | Sub | Mul | Div => {
                    let (l, r) = (eval_expr(lhs)?, eval_expr(rhs)?);
                    match (l, r, op) {
                        (Value::Int(a), Value::Int(b), Add) => Ok(Value::Int(a + b)),
                        (Value::Int(a), Value::Int(b), Sub) => Ok(Value::Int(a - b)),
                        (Value::Int(a), Value::Int(b), Mul) => Ok(Value::Int(a * b)),
                        (Value::Int(a), Value::Int(b), Div) => Ok(Value::Int(a / b)),

                        (Value::Int(a), Value::Float(b), Add) => Ok(Value::Float((a as f64) + b)),
                        (Value::Int(a), Value::Float(b), Sub) => Ok(Value::Float((a as f64) - b)),
                        (Value::Int(a), Value::Float(b), Mul) => Ok(Value::Float((a as f64) * b)),
                        (Value::Int(a), Value::Float(b), Div) => Ok(Value::Float((a as f64) / b)),

                        (Value::Float(a), Value::Int(b), Add) => Ok(Value::Float(a + (b as f64))),
                        (Value::Float(a), Value::Int(b), Sub) => Ok(Value::Float(a - (b as f64))),
                        (Value::Float(a), Value::Int(b), Mul) => Ok(Value::Float(a * (b as f64))),
                        (Value::Float(a), Value::Int(b), Div) => Ok(Value::Float(a / (b as f64))),

                        (Value::Float(a), Value::Float(b), Add) => Ok(Value::Float(a + b)),
                        (Value::Float(a), Value::Float(b), Sub) => Ok(Value::Float(a - b)),
                        (Value::Float(a), Value::Float(b), Mul) => Ok(Value::Float(a * b)),
                        (Value::Float(a), Value::Float(b), Div) => Ok(Value::Float(a / b)),

                        _ => bail!("arithmetic expects Int/Float"),
                    }
                }

                Lt | Le | Gt | Ge => {
                    let (l, r) = (eval_expr(lhs)?, eval_expr(rhs)?);
                    let res = match (l, r, op) {
                        (Value::Int(a), Value::Int(b), Lt) => a < b,
                        (Value::Int(a), Value::Int(b), Le) => a <= b,
                        (Value::Int(a), Value::Int(b), Gt) => a > b,
                        (Value::Int(a), Value::Int(b), Ge) => a >= b,

                        (Value::Float(a), Value::Float(b), Lt) => a < b,
                        (Value::Float(a), Value::Float(b), Le) => a <= b,
                        (Value::Float(a), Value::Float(b), Gt) => a > b,
                        (Value::Float(a), Value::Float(b), Ge) => a >= b,

                        (Value::Int(a), Value::Float(b), Lt) => (a as f64) < b,
                        (Value::Int(a), Value::Float(b), Le) => (a as f64) <= b,
                        (Value::Int(a), Value::Float(b), Gt) => (a as f64) > b,
                        (Value::Int(a), Value::Float(b), Ge) => (a as f64) >= b,

                        (Value::Float(a), Value::Int(b), Lt) => a < (b as f64),
                        (Value::Float(a), Value::Int(b), Le) => a <= (b as f64),
                        (Value::Float(a), Value::Int(b), Gt) => a > (b as f64),
                        (Value::Float(a), Value::Int(b), Ge) => a >= (b as f64),

                        _ => bail!("relational ops expect numbers"),
                    };
                    Ok(Value::Bool(res))
                }

                Eq | Ne => {
                    let (l, r) = (eval_expr(lhs)?, eval_expr(rhs)?);
                    let eq = match (l, r) {
                        (Value::Int(a), Value::Int(b)) => a == b,
                        (Value::Float(a), Value::Float(b)) => a == b,
                        (Value::Int(a), Value::Float(b)) => (a as f64) == b,
                        (Value::Float(a), Value::Int(b)) => a == (b as f64),
                        (Value::Bool(a), Value::Bool(b)) => a == b,
                        (Value::Str(a), Value::Str(b)) => a == b,
                        (Value::Nil, Value::Nil) => true,
                        _ => false,
                    };
                    Ok(Value::Bool(if matches!(op, Eq) { eq } else { !eq }))
                }

                And | Or => {
                    let l = eval_expr(lhs)?;
                    match (op, l) {
                        (And, Value::Bool(false)) => Ok(Value::Bool(false)),
                        (Or, Value::Bool(true)) => Ok(Value::Bool(true)),
                        (And, Value::Bool(true)) => {
                            let r = eval_expr(rhs)?;
                            match r {
                                Value::Bool(b) => Ok(Value::Bool(b)),
                                _ => bail!("&& expects Bool"),
                            }
                        }
                        (Or, Value::Bool(false)) => {
                            let r = eval_expr(rhs)?;
                            match r {
                                Value::Bool(b) => Ok(Value::Bool(b)),
                                _ => bail!("|| expects Bool"),
                            }
                        }
                        _ => bail!("logical ops expect Bool"),
                    }
                }
            }
        }

        Expr::Var(id) => bail!("unknown variable `{}` (no env yet)", id.text),
        Expr::Call { .. } => bail!("calls not supported in tiny evaluator yet"),
        Expr::Paren { inner, .. } => eval_expr(inner),

        // Phase 4 will implement evaluation for these
        Expr::Block(_) => bail!("block evaluation not implemented yet (Issue 006 Phase 4)"),
        Expr::If { .. } => bail!("if evaluation not implemented yet (Issue 006 Phase 4)"),
        Expr::While { .. } => bail!("while evaluation not implemented yet (Issue 006 Phase 4)"),
    }
}
