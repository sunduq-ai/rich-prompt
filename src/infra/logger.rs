use crossterm::{style::{Color, SetForegroundColor, ResetColor}, ExecutableCommand};
use env_logger::Builder;
use log::{debug, info, Level};
use std::io::Write;

pub fn setup_logger(verbosity: u8) -> Result<(), log::SetLoggerError> {
    let level = match verbosity {
        0 => "error",
        1 => "warn",
        2 => "info",
        _ => "debug",
    };
    
    let env = env_logger::Env::default().filter_or("RICH_PROMPT_LOG_LEVEL", level);
    
    Builder::from_env(env)
        .format(|buf, record| {
            let level_color = match record.level() {
                Level::Error => "31", // Red
                Level::Warn => "33",  // Yellow
                Level::Info => "32",  // Green
                Level::Debug => "36", // Cyan
                Level::Trace => "35", // Magenta
            };

            writeln!(
                buf,
                "\x1B[{}m[{}]\x1B[0m [{}] {}",
                level_color,
                record.level(),
                buf.timestamp(),
                record.args()
            )
        })
        .format_timestamp_secs()
        .init();
    Ok(())
}

pub fn print_welcome_message() {
    let mut stdout = std::io::stdout();
    
    writeln!(stdout).unwrap();
    stdout.execute(SetForegroundColor(Color::Cyan)).unwrap();
    writeln!(stdout, "🚀 Rich Prompt v0.3.0").unwrap();
    stdout.execute(ResetColor).unwrap();
    writeln!(stdout, "🧠 Supercharge your LLM interactions with structured context").unwrap();
    writeln!(stdout).unwrap();
    
    debug!("Debug logging enabled");
    info!("Starting Rich Prompt...");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;

    static INIT: Once = Once::new();

    #[test]
    fn test_setup_logger() {
        INIT.call_once(|| {
            assert!(setup_logger(0).is_ok());
        });
    }
}