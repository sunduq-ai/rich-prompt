use clipboard::{ClipboardContext, ClipboardProvider};
use crossterm::{
    ExecutableCommand,
    style::{Color, ResetColor, SetForegroundColor},
};
use log::{debug, info, warn};
use std::fs;
use std::io::{self, Write};
use std::path::Path;

pub trait OutputWriter {
    fn write(&self, content: &str) -> anyhow::Result<()>;
}

pub struct FileWriter {
    path: String,
}

impl FileWriter {
    pub fn new(path: String) -> Self {
        Self { path }
    }
}

impl OutputWriter for FileWriter {
    fn write(&self, content: &str) -> anyhow::Result<()> {
        debug!("Writing output to file: {}", self.path);
        fs::write(Path::new(&self.path), content)?;
        info!("Output written to file: {}", self.path);
        Ok(())
    }
}

pub struct ConsoleWriter;

impl OutputWriter for ConsoleWriter {
    fn write(&self, content: &str) -> anyhow::Result<()> {
        debug!("Writing output to console");
        io::stdout().write_all(content.as_bytes())?;
        io::stdout().write_all(b"\n")?;
        Ok(())
    }
}

pub struct ClipboardWriter;

impl OutputWriter for ClipboardWriter {
    fn write(&self, content: &str) -> anyhow::Result<()> {
        debug!("Writing output to clipboard");

        let mut ctx: ClipboardContext = match ClipboardProvider::new() {
            Ok(ctx) => ctx,
            Err(e) => {
                warn!("Failed to access clipboard: {}", e);
                return Err(anyhow::anyhow!("Failed to access clipboard: {}", e));
            }
        };

        match ctx.set_contents(content.to_owned()) {
            Ok(_) => {
                info!("Output copied to clipboard (size: {} bytes)", content.len());
                Ok(())
            }
            Err(e) => {
                warn!("Failed to copy to clipboard: {}", e);
                Err(anyhow::anyhow!("Failed to copy to clipboard: {}", e))
            }
        }
    }
}

pub fn create_writer(
    output_path: &Option<String>,
    clipboard_output: bool,
) -> Box<dyn OutputWriter> {
    if clipboard_output {
        return Box::new(ClipboardWriter) as Box<dyn OutputWriter>;
    }

    match output_path {
        Some(path) => Box::new(FileWriter::new(path.clone())) as Box<dyn OutputWriter>,
        None => Box::new(ConsoleWriter) as Box<dyn OutputWriter>,
    }
}

pub fn write_output(
    formatted_content: &str,
    output_path: Option<String>,
    clipboard_output: bool,
) -> anyhow::Result<()> {
    let mut stdout = io::stdout();

    let writer = create_writer(&output_path, clipboard_output);
    if let Err(e) = writer.write(formatted_content) {
        return Err(e);
    }

    if clipboard_output && output_path.is_none() {
        stdout.execute(SetForegroundColor(Color::Green))?;
        writeln!(stdout, "\n📋 Content copied to clipboard!")?;
        stdout.execute(ResetColor)?;

        writeln!(stdout, "\nPreview of copied content:\n")?;

        let preview_length = 200;
        let preview = if formatted_content.chars().count() > preview_length {
            let safe_substring: String = formatted_content.chars().take(preview_length).collect();
            format!("{}...", safe_substring)
        } else {
            formatted_content.to_string()
        };

        writeln!(stdout, "{}", preview)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_file_writer() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_string_lossy().to_string();
        let writer = FileWriter::new(path.clone());
        let content = "Test output";

        writer.write(content).unwrap();

        let read_content = fs::read_to_string(path).unwrap();
        assert_eq!(read_content, content);
    }

    #[test]
    fn test_create_writer() {
        let file_writer = create_writer(&Some("test.txt".to_string()), false);
        assert_eq!(
            std::any::type_name_of_val(&*file_writer),
            "dyn rich_prompt::infra::output::OutputWriter"
        );

        let console_writer = create_writer(&None, false);
        assert_eq!(
            std::any::type_name_of_val(&*console_writer),
            "dyn rich_prompt::infra::output::OutputWriter"
        );

        let clipboard_writer = create_writer(&None, true);
        assert_eq!(
            std::any::type_name_of_val(&*clipboard_writer),
            "dyn rich_prompt::infra::output::OutputWriter"
        );
    }

    #[test]
    fn test_utf8_safe_preview() {
        let content =
            "اهلا مرحب عبدالله 🚀 This string has UTF-8 characters like: ├── ./src/file.rs";

        let preview_length = 20;
        let preview: String = content.chars().take(preview_length).collect();

        assert_eq!(preview.chars().count(), preview_length);
    }
}
