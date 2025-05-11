use crate::core::context_generator::{build_context_output, format_output};
use crate::core::file_selector::select_files;
use crate::domain::models::ContextConfig;
use crate::infra::file_system::{
    generate_file_map, list_code_files, list_code_files_with_gitignore, read_file_contents,
};
use crate::infra::logger::{print_welcome_message, setup_logger};
use crate::infra::output::write_output;
use clap::{Parser, Subcommand};
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use log::{debug, info, warn};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Text},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::io;
use std::path::PathBuf;
use std::time::Duration;

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
        #[arg(long, default_value = ".")]
        path: String,

        #[arg(long)]
        ext: Option<String>,

        #[arg(long)]
        exclude: Option<String>,

        #[arg(long)]
        output: Option<String>,

        #[arg(long)]
        auto: bool,

        #[arg(long)]
        prompt: Option<String>,

        #[arg(long, default_value = ".git")]
        exclude_version_control_dir: String,

        #[arg(long, default_value = "true")]
        apply_dot_git_ignore: bool,

        #[arg(long, help = "Copy the output to clipboard (requires X11/Wayland on Linux)")]
        clipboard_output: bool,
    },
}

fn get_prompt_input() -> anyhow::Result<Option<String>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;


    let mut prompt_text = String::new();
    let mut cursor_position = 0;


    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints(
                    [
                        Constraint::Length(1),
                        Constraint::Length(3),
                        Constraint::Length(1),
                    ]
                    .as_ref(),
                )
                .split(f.area());

            let title = Paragraph::new(Span::styled(
                "Enter your prompt instructions",
                Style::default().add_modifier(Modifier::BOLD),
            ));
            f.render_widget(title, chunks[0]);

            let input = Paragraph::new(prompt_text.as_str())
                .style(Style::default().fg(Color::Blue))
                .block(Block::default().borders(Borders::ALL).title("Prompt"));
            f.render_widget(input, chunks[1]);

            f.set_cursor_position((
                chunks[1].x + 1 + cursor_position as u16,
                chunks[1].y + 1,
            ));

            let mut text = Text::default();
            text.extend(vec![
                Span::styled("Press ", Style::default().fg(Color::DarkGray))
            ]);
            let controls = Paragraph::new(text);
            f.render_widget(controls, chunks[2]);
        })?;

        if let Ok(true) = event::poll(Duration::from_millis(100)) {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Enter => {
                        // Submit the prompt
                        break;
                    }
                    KeyCode::Esc => {
                        // Skip providing a prompt
                        prompt_text.clear();
                        break;
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        // Cancel operation
                        prompt_text.clear();
                        break;
                    }
                    KeyCode::Char(c) => {
                        prompt_text.insert(cursor_position, c);
                        cursor_position += 1;
                    }
                    KeyCode::Backspace => {
                        if cursor_position > 0 {
                            prompt_text.remove(cursor_position - 1);
                            cursor_position -= 1;
                        }
                    }
                    KeyCode::Delete => {
                        if cursor_position < prompt_text.len() {
                            prompt_text.remove(cursor_position);
                        }
                    }
                    KeyCode::Left => {
                        if cursor_position > 0 {
                            cursor_position -= 1;
                        }
                    }
                    KeyCode::Right => {
                        if cursor_position < prompt_text.len() {
                            cursor_position += 1;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    // Return the prompt if it's not empty
    if prompt_text.is_empty() {
        Ok(None)
    } else {
        Ok(Some(prompt_text))
    }
}

pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    setup_logger(cli.verbose)?;
    print_welcome_message();

    match cli.command {
        Commands::Generate {
            path,
            ext,
            exclude,
            output,
            auto,
            prompt,
            exclude_version_control_dir,
            apply_dot_git_ignore,
            clipboard_output,
        } => {
            info!("Starting generate command");
            debug!(
                "Command parameters: path={}, ext={:?}, exclude={:?}, output={:?}, auto={}, prompt={:?}, exclude_version_control_dir={}, apply_dot_git_ignore={}, clipboard_output={}",
                path,
                ext,
                exclude,
                output,
                auto,
                prompt,
                exclude_version_control_dir,
                apply_dot_git_ignore,
                clipboard_output
            );

            let extensions: Vec<&str> = match &ext {
                Some(ext_value) => ext_value.split(',').map(str::trim).collect(),
                None => Vec::new(),
            };

            let excludes: Vec<&str> = match &exclude {
                Some(exclude_value) => exclude_value.split(',').map(str::trim).collect(),
                None => Vec::new(),
            };

            let mut config = ContextConfig {
                root_path: path.clone(),
                extensions: extensions.iter().map(|&s| s.to_string()).collect(),
                exclude_patterns: excludes.iter().map(|&s| s.to_string()).collect(),
                output_path: output.clone(),
                auto_select: auto,
                user_prompt: prompt,
                exclude_version_control_dir,
                apply_dot_git_ignore,
                clipboard_output,
            };

            match generate_context(&mut config) {
                Ok(_) => {
                    info!("Context generation completed successfully");
                }
                Err(e) => {
                    if e.to_string().contains("No files selected")
                        || e.to_string().contains("Selection cancelled")
                    {
                        info!("{}", e);
                        info!("Operation cancelled by user");
                        return Ok(());
                    }
                    return Err(e);
                }
            }
        }
    }
    Ok(())
}

fn generate_context(config: &mut ContextConfig) -> anyhow::Result<()> {
    if config.user_prompt.is_none() {
        info!("Asking for user prompt");
        match get_prompt_input()? {
            Some(prompt) => {
                config.user_prompt = Some(prompt);
                info!("Prompt set by user");
            }
            None => {
                info!("Prompt skipped by user");
            }
        }
    }

    let extensions: Vec<&str> = config.extensions.iter().map(|s| s.as_str()).collect();
    let excludes: Vec<&str> = config.exclude_patterns.iter().map(|s| s.as_str()).collect();

    info!("Scanning for files in {}", config.root_path);
    let available_files = if config.apply_dot_git_ignore {
        list_code_files_with_gitignore(
            &config.root_path,
            &extensions,
            &excludes,
            &config.exclude_version_control_dir,
            config.apply_dot_git_ignore,
        )?
    } else {
        list_code_files(&config.root_path, &extensions, &excludes)?
    };

    if available_files.is_empty() {
        warn!("No files found with the specified extensions");
        return Err(anyhow::anyhow!(
            "No files found matching the specified criteria"
        ));
    }

    info!("Generating file map");
    let file_map = generate_file_map(
        &config.root_path,
        &excludes,
        &config.exclude_version_control_dir,
        config.apply_dot_git_ignore,
    )?;

    info!("Selecting files");
    let selected_files = select_files(
        available_files,
        |path: &PathBuf| read_file_contents(&path),
        config.auto_select,
    )?;

    if selected_files.is_empty() {
        warn!("No files were selected");
        return Err(anyhow::anyhow!("No files were selected"));
    }

    info!("Building context output");
    let output = build_context_output(selected_files, file_map, config.user_prompt.clone());
    let formatted_output = format_output(&output);

    info!("Writing output");
    write_output(
        &formatted_output,
        config.output_path.clone(),
        config.clipboard_output,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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
            "--exclude-version-control-dir",
            ".svn",
            "--apply-dot-git-ignore",
            "--clipboard-output",
        ])
        .unwrap();

        match cli.command {
            Commands::Generate {
                path,
                ext,
                exclude,
                auto,
                prompt,
                exclude_version_control_dir,
                apply_dot_git_ignore,
                clipboard_output,
                ..
            } => {
                assert_eq!(path, "./src");
                assert_eq!(ext, Some(".rs".to_string()));
                assert_eq!(exclude, Some(".git".to_string()));
                assert!(auto);
                assert_eq!(prompt, Some("Test prompt".to_string()));
                assert_eq!(exclude_version_control_dir, ".svn");
                assert_eq!(apply_dot_git_ignore, true);
                assert_eq!(clipboard_output, true);
            }
        }
    }

    #[test]
    fn test_cli_parsing_with_optional_args() {
        let cli = Cli::try_parse_from(&[
            "rich-prompt",
            "generate",
            "--path",
            "./src",
            "--auto",
            "--exclude-version-control-dir",
            ".svn",
        ])
        .unwrap();

        match cli.command {
            Commands::Generate {
                path,
                ext,
                exclude,
                auto,
                prompt,
                exclude_version_control_dir,
                apply_dot_git_ignore,
                clipboard_output,
                ..
            } => {
                assert_eq!(path, "./src");
                assert_eq!(ext, None);
                assert_eq!(exclude, None);
                assert!(auto);
                assert_eq!(prompt, None);
                assert_eq!(exclude_version_control_dir, ".svn");
                assert_eq!(apply_dot_git_ignore, true);
                assert_eq!(clipboard_output, false);
            }
        }
    }
}