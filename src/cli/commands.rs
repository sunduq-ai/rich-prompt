use crate::core::context_generator::generate_context;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "rich-prompt")]
#[command(about = "Flatten files into LLM context block", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Generate {
        #[arg(long)]
        path: String,

        #[arg(long, default_value = ".java,.js,.go,.rs,.py,.toml,.yml")]
        ext: String,

        #[arg(long, default_value = ".git,.venv,target")]
        exclude: String,

        #[arg(long)]
        output: Option<String>,

        #[arg(long)]
        auto: bool,
    },
}

pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Generate {
            path,
            ext,
            exclude,
            output,
            auto,
        } => {
            let extensions: Vec<&str> = ext.split(',').map(str::trim).collect();
            let excludes: Vec<&str> = exclude.split(',').map(str::trim).collect();
            generate_context(&path, &extensions, &excludes, output, auto)?;
        }
    }
    Ok(())
}
