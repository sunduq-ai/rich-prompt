use std::fs;
use std::path::Path;

pub fn write_output(content: &str, output_path: Option<String>) -> anyhow::Result<()> {
    match output_path {
        Some(path) => fs::write(Path::new(&path), content)?,
        None => println!("{}", content),
    }
    Ok(())
}
