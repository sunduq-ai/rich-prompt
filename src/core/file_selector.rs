use crate::domain::models::FileContext;
use dialoguer::MultiSelect;
use log::{debug, info, warn};
use std::path::PathBuf;

pub fn select_files(
    files: Vec<PathBuf>,
    file_reader: impl Fn(&PathBuf) -> anyhow::Result<String>,
    auto: bool,
) -> anyhow::Result<Vec<FileContext>> {
    if files.is_empty() {
        info!("No files to select");
        return Ok(Vec::new());
    }

    debug!("Selecting from {} available files", files.len());
    let selected_paths = if auto {
        info!("Auto-selecting all {} files", files.len());
        files
    } else {
        info!("Entering interactive selection mode");
        let options = files
            .iter()
            .map(|f| f.to_string_lossy().to_string())
            .collect::<Vec<_>>();

        let selection = MultiSelect::new()
            .with_prompt("Select files to include")
            .items(&options)
            .interact()?;

        info!("Selected {} files", selection.len());
        selection.into_iter().map(|i| files[i].clone()).collect()
    };

    let mut selected_files = Vec::new();
    for path in selected_paths {
        debug!("Reading file: {}", path.display());
        match file_reader(&path) {
            Ok(content) => {
                selected_files.push(FileContext { path, content });
            }
            Err(e) => {
                warn!("Error reading file {}: {}", path.display(), e);
            }
        }
    }

    info!("Successfully loaded {} files", selected_files.len());
    Ok(selected_files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct MockFileSystem {
        files: HashMap<PathBuf, String>,
    }

    impl MockFileSystem {
        fn new() -> Self {
            Self {
                files: HashMap::new(),
            }
        }

        fn add_file(&mut self, path: PathBuf, content: String) {
            self.files.insert(path, content);
        }

        fn read_file(&self, path: &PathBuf) -> anyhow::Result<String> {
            match self.files.get(path) {
                Some(content) => Ok(content.clone()),
                None => Err(anyhow::anyhow!("File not found")),
            }
        }
    }

    #[test]
    fn test_select_files_with_auto() {
        let mut mock_fs = MockFileSystem::new();
        mock_fs.add_file(PathBuf::from("file1.rs"), "content1".to_string());
        mock_fs.add_file(PathBuf::from("file2.rs"), "content2".to_string());

        let files = vec![PathBuf::from("file1.rs"), PathBuf::from("file2.rs")];

        let reader = |path: &PathBuf| mock_fs.read_file(path);

        let selected = select_files(files, reader, true).unwrap();

        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0].content, "content1");
        assert_eq!(selected[1].content, "content2");
    }

    #[test]
    fn test_select_files_with_empty_input() {
        let files: Vec<PathBuf> = vec![];
        let reader = |_: &PathBuf| -> anyhow::Result<String> { Ok("".to_string()) };

        let selected = select_files(files, reader, true).unwrap();

        assert_eq!(selected.len(), 0);
    }

    #[test]
    fn test_select_files_with_read_error() {
        let files = vec![PathBuf::from("nonexistent.rs")];
        let reader =
            |_: &PathBuf| -> anyhow::Result<String> { Err(anyhow::anyhow!("File not found")) };

        let selected = select_files(files, reader, true).unwrap();

        assert_eq!(selected.len(), 0);
    }
}
