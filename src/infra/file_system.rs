use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

pub fn list_code_files(
    root: &str,
    extensions: &[&str],
    exclude_patterns: &[&str],
) -> anyhow::Result<Vec<PathBuf>> {
    println!("Listing code files in: {}", root);
    println!("Extensions: {:?}", extensions);
    println!("Exclude patterns: {:?}\n", exclude_patterns);

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
            .map(|e| extensions.iter().any(|ext| ext.trim_start_matches('.') == e))
            .unwrap_or(false);

        let excluded = exclude_patterns
            .iter()
            .any(|pattern| path.to_string_lossy().contains(pattern));

        if ext_matches && !excluded {
            result.push(path.to_path_buf());
        }
    }
    Ok(result)
}

pub fn read_file_contents(path: &Path) -> anyhow::Result<String> {
    if !path.exists() {
        println!("File does not exist: {}", path.display());
        return Ok(String::new());
    }
    if !path.is_file() {
        println!("Not a file: {}", path.display());
        return Ok(String::new());
    }
    if path.metadata()?.len() == 0 {
        println!("File is empty: {}", path.display());
        return Ok(String::new());
    }

    let mut file = fs::File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

pub fn list_dir_structure(
    root: &str,
    exclude_patterns: &[&str],
) -> anyhow::Result<HashMap<String, Vec<String>>> {
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
    Ok(dir_map)
}
