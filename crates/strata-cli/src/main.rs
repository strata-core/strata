mod eval;

use anyhow::Result;
use clap::{Parser, ValueEnum};
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
        eval::eval_module(&module)?;
        return Ok(());
    }

    match cli.format {
        Format::Pretty => println!("{:#?}", module),
        Format::Json => println!("{}", serde_json::to_string_pretty(&module)?),
    }
    Ok(())
}
