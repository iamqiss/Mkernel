//! Neo CLI tool

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "neo")]
#[command(about = "Neo messaging kernel CLI")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new service
    New {
        /// Service name
        name: String,
    },
    /// Build a service
    Build {
        /// Service directory
        #[arg(short, long)]
        path: Option<PathBuf>,
    },
    /// Run a service
    Run {
        /// Manifest file
        #[arg(short, long)]
        manifest: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::New { name } => {
            create_new_service(name).await?;
        }
        Commands::Build { path } => {
            build_service(path).await?;
        }
        Commands::Run { manifest } => {
            run_service(manifest).await?;
        }
    }

    Ok(())
}

async fn create_new_service(name: String) -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating new service: {}", name);
    // TODO: Implement service creation
    Ok(())
}

async fn build_service(path: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    println!("Building service at: {:?}", path);
    // TODO: Implement service building
    Ok(())
}

async fn run_service(manifest: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running service with manifest: {:?}", manifest);
    // TODO: Implement service running
    Ok(())
}