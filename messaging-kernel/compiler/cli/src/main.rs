//! CLI for Neo protocol compiler

use clap::{Parser, Subcommand};
use neo_codegen::generate_rust_code;
use neo_parser::parse;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "neo-compiler")]
#[command(about = "Neo protocol compiler")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile a .neo file to Rust code
    Compile {
        /// Input .neo file
        #[arg(short, long)]
        input: PathBuf,
        /// Output directory
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Validate a .neo file
    Validate {
        /// Input .neo file
        #[arg(short, long)]
        input: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Compile { input, output } => {
            compile_file(input, output).await?;
        }
        Commands::Validate { input } => {
            validate_file(input).await?;
        }
    }

    Ok(())
}

async fn compile_file(input: PathBuf, output: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    let source = fs::read_to_string(&input)?;
    let service = parse(&source)?;
    let rust_code = generate_rust_code(&service)?;

    let output_path = output.unwrap_or_else(|| {
        input.with_extension("rs")
    });

    fs::write(&output_path, rust_code)?;
    println!("Generated Rust code: {}", output_path.display());

    Ok(())
}

async fn validate_file(input: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let source = fs::read_to_string(&input)?;
    let _service = parse(&source)?;
    println!("File is valid: {}", input.display());

    Ok(())
}