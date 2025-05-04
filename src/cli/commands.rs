use crate::core::context_generator::{build_context_output, format_output};
use crate::core::file_selector::select_files;
use crate::domain::models::ContextConfig;
use crate::infra::file_system::{generate_file_map, list_code_files, read_file_contents};
use crate::infra::logger::setup_logger;
use crate::infra::output::write_output;
use clap::{Parser, Subcommand};
use log::{debug, info};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "rich-prompt")]
#[command(about = "Flatten files into LLM context block", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
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

        #[arg(long)]
        prompt: Option<String>,
    },
}

pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    setup_logger(cli.verbose)?;

    match cli.command {
        Commands::Generate {
            path,
            ext,
            exclude,
            output,
            auto,
            prompt,
        } => {
            info!("Starting generate command");
            debug!(
                "Command parameters: path={}, ext={}, exclude={}, output={:?}, auto={}, prompt={:?}",
                path, ext, exclude, output, auto, prompt
            );

            let extensions: Vec<&str> = ext.split(',').map(str::trim).collect();
            let excludes: Vec<&str> = exclude.split(',').map(str::trim).collect();


            let config = ContextConfig {
                root_path: path.clone(),
                extensions: extensions.iter().map(|&s| s.to_string()).collect(),
                exclude_patterns: excludes.iter().map(|&s| s.to_string()).collect(),
                output_path: output.clone(),
                auto_select: auto,
                user_prompt: prompt,
            };

            generate_context(&config)?;
        }
    }
    Ok(())
}

fn generate_context(config: &ContextConfig) -> anyhow::Result<()> {

    let extensions: Vec<&str> = config.extensions.iter().map(|s| s.as_str()).collect();
    let excludes: Vec<&str> = config.exclude_patterns.iter().map(|s| s.as_str()).collect();


    info!("Scanning for files in {}", config.root_path);
    let available_files = list_code_files(&config.root_path, &extensions, &excludes)?;

    if available_files.is_empty() {
        info!("No files found with the specified extensions");
        return Ok(());
    }

    info!("Generating file map");
    let file_map = generate_file_map(&config.root_path, &excludes)?;

    info!("Selecting files");
    let selected_files = select_files(
        available_files,
        |path: &PathBuf| read_file_contents(&path),
        config.auto_select,
    )?;

    info!("Building context output");
    let output = build_context_output(selected_files, file_map, config.user_prompt.clone());
    let formatted_output = format_output(&output);

    info!("Writing output");
    write_output(&output, &formatted_output, config.output_path.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_cli_parsing() {
        let cli = Cli::try_parse_from(&[
            "rich-prompt",
            "generate",
            "--path",
            "./src",
            "--ext",
            ".rs",
            "--exclude",
            ".git",
            "--auto",
            "--prompt",
            "Test prompt",
        ])
        .unwrap();

        match cli.command {
            Commands::Generate {
                path,
                ext,
                exclude,
                auto,
                prompt,
                ..
            } => {
                assert_eq!(path, "./src");
                assert_eq!(ext, ".rs");
                assert_eq!(exclude, ".git");
                assert!(auto);
                assert_eq!(prompt, Some("Test prompt".to_string()));
            }
        }
    }
}
