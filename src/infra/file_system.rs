use log::{debug, info, warn};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

pub fn list_code_files(
    root: &str,
    extensions: &[&str],
    exclude_patterns: &[&str],
) -> anyhow::Result<Vec<PathBuf>> {
    info!("Listing code files in: {}", root);
    debug!("Extensions: {:?}", extensions);
    debug!("Exclude patterns: {:?}", exclude_patterns);

    let mut result = Vec::new();

    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            let path = e.path().to_string_lossy();
            !exclude_patterns.iter().any(|pat| path.contains(pat))
        })
        .filter_map(Result::ok)
    {
        if entry.file_type().is_dir() || entry.file_type().is_symlink() {
            continue;
        }

        let path = entry.path();

        let ext_matches = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| {
                extensions
                    .iter()
                    .any(|ext| ext.trim_start_matches('.') == e)
            })
            .unwrap_or(false);

        let excluded = exclude_patterns
            .iter()
            .any(|pattern| path.to_string_lossy().contains(pattern));

        if ext_matches && !excluded {
            debug!("Found matching file: {}", path.display());
            result.push(path.to_path_buf());
        }
    }

    info!("Found {} matching files", result.len());
    Ok(result)
}

pub fn read_file_contents(path: &Path) -> anyhow::Result<String> {
    if !path.exists() {
        warn!("File does not exist: {}", path.display());
        return Ok(String::new());
    }
    if !path.is_file() {
        warn!("Not a file: {}", path.display());
        return Ok(String::new());
    }
    if path.metadata()?.len() == 0 {
        debug!("File is empty: {}", path.display());
        return Ok(String::new());
    }

    debug!("Reading file contents: {}", path.display());
    let mut file = fs::File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    debug!("Read {} bytes from file", contents.len());
    Ok(contents)
}

pub fn generate_file_map(root: &str, exclude_patterns: &[&str]) -> anyhow::Result<String> {
    info!("Generating file map for: {}", root);
    let mut output = String::new();
    let dir_map = list_dir_structure(root, exclude_patterns)?;

    for (dir, files) in &dir_map {
        output.push_str(&format!("{}\n", dir));
        for file in files {
            output.push_str(&format!("├── {}\n", file));
        }
    }

    debug!("Generated file map with {} directories", dir_map.len());
    Ok(output)
}

pub fn list_dir_structure(
    root: &str,
    exclude_patterns: &[&str],
) -> anyhow::Result<HashMap<String, Vec<String>>> {
    debug!("Listing directory structure in: {}", root);
    let mut dir_map = HashMap::new();

    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            let path = e.path().to_string_lossy();
            !exclude_patterns.iter().any(|pat| path.contains(pat))
        })
        .filter_map(Result::ok)
    {
        if entry.file_type().is_dir() {
            let path = entry.path().to_string_lossy().to_string();
            dir_map.entry(path).or_insert_with(Vec::new);
        } else if entry.file_type().is_file() {
            let path = entry.path().to_string_lossy().to_string();
            let parent = entry.path().parent().unwrap_or_else(|| Path::new(""));
            dir_map
                .entry(parent.to_string_lossy().to_string())
                .or_insert_with(Vec::new)
                .push(path);
        }
    }

    debug!("Found {} directories in structure", dir_map.len());
    Ok(dir_map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_read_file_contents() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        {
            let mut file = File::create(&file_path).unwrap();
            writeln!(file, "Test content").unwrap();
        }

        let contents = read_file_contents(&file_path).unwrap();
        assert_eq!(contents, "Test content\n");
    }

    #[test]
    fn test_read_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("nonexistent.txt");

        let contents = read_file_contents(&file_path).unwrap();
        assert_eq!(contents, "");
    }
}
