use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use strata_ast::ast::Item;
use strata_parse::parse_str;
use strata_types::TypeChecker;

/// Maximum source file size in bytes (1MB)
const MAX_SOURCE_SIZE: usize = 1_000_000;

#[derive(Parser, Debug)]
#[command(name = "strata")]
#[command(about = "Strata: safe automation with effect types and capability security")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Execute a Strata program
    Run {
        /// Path to .strata source file
        file: String,

        /// Write effect trace to file (large values hashed)
        #[arg(long)]
        trace: Option<String>,

        /// Write replay-capable trace (all values recorded)
        #[arg(long, conflicts_with = "trace")]
        trace_full: Option<String>,
    },

    /// Replay a recorded effect trace
    Replay {
        /// Path to trace JSONL file
        trace_path: String,

        /// Path to .strata source file (omit for trace summary)
        file: Option<String>,
    },

    /// Parse a source file and dump the AST
    Parse {
        /// Path to .strata source file
        file: String,

        /// Output format
        #[arg(long, value_enum, default_value_t = Format::Pretty)]
        format: Format,
    },
}

#[derive(ValueEnum, Clone, Debug)]
enum Format {
    Pretty,
    Json,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            file,
            trace,
            trace_full,
        } => cmd_run(&file, trace, trace_full),

        Commands::Replay { trace_path, file } => cmd_replay(&trace_path, file.as_deref()),

        Commands::Parse { file, format } => cmd_parse(&file, format),
    }
}

fn load_and_typecheck(path: &str) -> Result<strata_ast::ast::Module, Box<dyn std::error::Error>> {
    let src = std::fs::read_to_string(path)?;

    if src.len() > MAX_SOURCE_SIZE {
        eprintln!(
            "Error: source file exceeds {}MB limit ({} bytes)",
            MAX_SOURCE_SIZE / 1_000_000,
            src.len()
        );
        std::process::exit(1);
    }

    let module = parse_str(path, &src)?;

    let mut type_checker = TypeChecker::new();
    if let Err(e) = type_checker.check_module(&module) {
        eprintln!("Type error: {}", e);
        std::process::exit(1);
    }

    Ok(module)
}

fn cmd_run(
    file: &str,
    trace: Option<String>,
    trace_full: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let module = load_and_typecheck(file)?;

    let has_main_params = module
        .items
        .iter()
        .any(|item| matches!(item, Item::Fn(d) if d.name.text == "main" && !d.params.is_empty()));

    let has_main = module
        .items
        .iter()
        .any(|item| matches!(item, Item::Fn(d) if d.name.text == "main"));

    if let Some(trace_path) = trace_full {
        // Replay-capable trace: all values recorded
        let writer: Box<dyn std::io::Write + Send> = Box::new(std::fs::File::create(&trace_path)?);
        let result = strata_cli::eval::run_module_traced_full(&module, writer)?;
        print_result(&result, has_main);
        eprintln!("Trace written to {}", trace_path);
    } else if let Some(trace_path) = trace {
        // Audit trace: large values hashed
        let writer: Box<dyn std::io::Write + Send> = Box::new(std::fs::File::create(&trace_path)?);
        let result = strata_cli::eval::run_module_traced(&module, writer)?;
        print_result(&result, has_main);
        eprintln!("Trace written to {}", trace_path);
    } else if has_main_params {
        // No trace — run with capability injection
        let result = strata_cli::eval::run_module(&module)?;
        print_result(&result, true);
    } else if has_main {
        // No trace — run module with simple main()
        let result = strata_cli::eval::run_module(&module)?;
        print_result(&result, true);
    } else {
        // No main() — eval module (print let bindings)
        strata_cli::eval::eval_module(&module)?;
    }

    Ok(())
}

fn print_result(result: &strata_cli::eval::Value, _has_main: bool) {
    match result {
        strata_cli::eval::Value::Unit => {
            println!("Program completed successfully.");
        }
        other => {
            println!("main() = {}", other);
        }
    }
}

fn cmd_replay(trace_path: &str, file: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let trace_content = std::fs::read_to_string(trace_path)
        .map_err(|e| anyhow::anyhow!("Failed to read trace file '{}': {}", trace_path, e))?;

    match file {
        Some(source_path) => {
            // Replay against source
            let module = load_and_typecheck(source_path)?;
            strata_cli::eval::run_module_replay(&module, &trace_content)?;

            let effect_count = trace_content.lines().filter(|l| !l.is_empty()).count();
            println!("Replay successful: {} effects replayed.", effect_count);
        }
        None => {
            // Print trace summary
            print_trace_summary(&trace_content)?;
        }
    }

    Ok(())
}

fn print_trace_summary(trace_content: &str) -> Result<(), Box<dyn std::error::Error>> {
    let records: Vec<serde_json::Value> = trace_content
        .lines()
        .filter(|l| !l.is_empty())
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?;

    if records.is_empty() {
        println!("Trace: empty (no effects recorded)");
        return Ok(());
    }

    // Extract effect entries (skip header/footer)
    let mut effects = Vec::new();
    let mut header_info = None;
    let mut footer_info = None;

    for record in &records {
        match record.get("record").and_then(|r| r.as_str()) {
            Some("header") => {
                let version = record["schema_version"].as_str().unwrap_or("?");
                let full = record["full_values"].as_bool().unwrap_or(false);
                header_info = Some((version.to_string(), full));
            }
            Some("footer") => {
                let status = record["program_status"].as_str().unwrap_or("?");
                let trace_status = record["trace_status"].as_str().unwrap_or("?");
                footer_info = Some((status.to_string(), trace_status.to_string()));
            }
            Some("effect") | None => {
                // "effect" record or legacy format (no "record" field)
                effects.push(record);
            }
            _ => {}
        }
    }

    if let Some((version, full)) = &header_info {
        println!(
            "Trace schema: v{}, mode: {}",
            version,
            if *full {
                "full (replay-capable)"
            } else {
                "audit (hashed)"
            }
        );
    }

    println!("Trace summary: {} effects", effects.len());
    for entry in &effects {
        let seq = entry["seq"].as_u64().unwrap_or(0);
        let effect = entry["effect"].as_str().unwrap_or("?");
        let op = entry["operation"].as_str().unwrap_or("?");
        let access = entry["capability"]["access"].as_str().unwrap_or("?");
        let status = entry["output"]["status"].as_str().unwrap_or("?");
        let duration = entry["duration_ms"].as_u64().unwrap_or(0);
        println!(
            "  [{}] {}::{}    ({}) - {}, {}ms",
            seq, effect, op, access, status, duration
        );
    }

    if let Some((prog_status, trace_status)) = &footer_info {
        println!("Program: {}, Trace: {}", prog_status, trace_status);
    }

    Ok(())
}

fn cmd_parse(file: &str, format: Format) -> Result<(), Box<dyn std::error::Error>> {
    let src = std::fs::read_to_string(file)?;

    if src.len() > MAX_SOURCE_SIZE {
        eprintln!(
            "Error: source file exceeds {}MB limit ({} bytes)",
            MAX_SOURCE_SIZE / 1_000_000,
            src.len()
        );
        std::process::exit(1);
    }

    let module = parse_str(file, &src)?;

    let mut type_checker = TypeChecker::new();
    if let Err(e) = type_checker.check_module(&module) {
        eprintln!("Type error: {}", e);
        std::process::exit(1);
    }

    match format {
        Format::Pretty => println!("{:#?}", module),
        Format::Json => println!("{}", serde_json::to_string_pretty(&module)?),
    }
    Ok(())
}
