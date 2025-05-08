use crate::domain::models::ContextOutput;
use clipboard::{ClipboardContext, ClipboardProvider};
use dialoguer::console::style;
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
    output: &ContextOutput,
    formatted_content: &str,
    output_path: Option<String>,
    clipboard_output: bool,
) -> anyhow::Result<()> {
    println!(
        "{} {}",
        style("⚙️ Total tokens in output:").bold().blue(),
        output.token_count
    );

    let writer = create_writer(&output_path, clipboard_output);
    if let Err(e) = writer.write(formatted_content) {
        return Err(e);
    }

    if clipboard_output && output_path.is_none() {
        println!(
            "{}",
            style("📋 Content copied to clipboard!").bold().green()
        );
        println!("{}", style("Preview of copied content:").italic());

        let preview_length = 200;
        let preview = if formatted_content.chars().count() > preview_length {
            let safe_substring: String = formatted_content.chars().take(preview_length).collect();
            format!("{}...", safe_substring)
        } else {
            formatted_content.to_string()
        };

        println!("{}", preview);
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

#[cfg(test)]
mod clipboard_tests {
    use tempfile::NamedTempFile;

    use super::*;
    use crate::domain::models::ContextOutput;

    #[test]
    #[ignore]
    fn test_clipboard_writer() {
        if std::env::var("ENABLE_CLIPBOARD_TESTS").is_err() {
            return;
        }

        let writer = ClipboardWriter;
        let test_content = "Test clipboard content";

        match writer.write(test_content) {
            Ok(_) => {
                println!("Clipboard write test passed");
            }
            Err(e) => {
                panic!("Failed to write to clipboard: {}", e);
            }
        }
    }

    #[test]
    fn test_write_output_with_clipboard() {
        let output = ContextOutput {
            file_map: "test_dir\n".to_string(),
            file_contents: "test_content\n".to_string(),
            user_instructions: "test_prompt".to_string(),
            token_count: 3,
        };

        let formatted_content = "<file_map>\ntest_dir\n</file_map>\n\n<file_contents>test_content\n</file_contents>\n\n<user_instructions>\ntest_prompt\n</user_instructions>";

        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_string_lossy().to_string();

        let result = write_output(&output, formatted_content, Some(path), true);
        assert!(result.is_ok());

        let result = write_output(&output, formatted_content, None, true);
        assert!(result.is_ok());
    }
}
