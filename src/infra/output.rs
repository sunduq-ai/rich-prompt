use crate::domain::models::ContextOutput;
use dialoguer::console::style;
use log::{debug, info};
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

pub fn create_writer(output_path: &Option<String>) -> Box<dyn OutputWriter> {
    match output_path {
        Some(path) => Box::new(FileWriter::new(path.clone())) as Box<dyn OutputWriter>,
        None => Box::new(ConsoleWriter) as Box<dyn OutputWriter>,
    }
}

pub fn write_output(
    output: &ContextOutput,
    formatted_content: &str,
    output_path: Option<String>,
) -> anyhow::Result<()> {
    println!("{} {}", style("⚙️ Total tokens in output:").bold().blue(), output.token_count);

    let writer = create_writer(&output_path);
    writer.write(formatted_content)
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
        let file_writer = create_writer(&Some("test.txt".to_string()));
        assert_eq!(
            std::any::type_name_of_val(&*file_writer),
            "dyn rich_prompt::infra::output::OutputWriter"
        );

        let console_writer = create_writer(&None);
        assert_eq!(
            std::any::type_name_of_val(&*console_writer),
            "dyn rich_prompt::infra::output::OutputWriter"
        );
    }
}
