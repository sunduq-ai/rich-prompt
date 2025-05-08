use log::{debug, info, warn};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{BufRead, Read};
use std::path::{Path, PathBuf};

fn parse_gitignore(root: &str) -> anyhow::Result<HashSet<String>> {
    let gitignore_path = Path::new(root).join(".gitignore");
    let mut patterns = HashSet::new();

    if gitignore_path.exists() && gitignore_path.is_file() {
        debug!("Parsing .gitignore file at: {}", gitignore_path.display());
        let file = fs::File::open(gitignore_path)?;
        let reader = std::io::BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();

            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                if trimmed.starts_with('!') && trimmed.len() > 1 {
                    patterns.insert(format!("!{}", trimmed[1..].trim()));
                } else {
                    patterns.insert(trimmed.to_string());
                }
            }
        }

        info!("Loaded {} patterns from .gitignore", patterns.len());
    } else {
        debug!("No .gitignore file found at: {}", gitignore_path.display());
    }

    Ok(patterns)
}

fn should_ignore_by_gitignore(
    path: &Path,
    root: &Path,
    gitignore_patterns: &HashSet<String>,
) -> bool {
    if gitignore_patterns.is_empty() {
        return false;
    }

    let rel_path = path.strip_prefix(root).unwrap_or(path);
    let path_str = rel_path.to_string_lossy();
    let is_dir = path.is_dir();

    let mut matched_negated = false;
    let mut should_ignore = false;

    for pattern in gitignore_patterns {
        let is_negated = pattern.starts_with('!');

        if is_negated {
            let negated_pattern = &pattern[1..]; // Remove the '!' prefix
            if matches_gitignore_pattern(path_str.as_ref(), negated_pattern, is_dir) {
                debug!(
                    "Path {} matches negated gitignore pattern: {}",
                    path_str, pattern
                );
                matched_negated = true;
            }
            continue;
        }

        if !matched_negated && matches_gitignore_pattern(path_str.as_ref(), pattern, is_dir) {
            debug!("Path {} matches gitignore pattern: {}", path_str, pattern);
            should_ignore = true;
        }
    }

    if matched_negated {
        return false;
    }

    should_ignore
}

fn matches_gitignore_pattern(path: &str, pattern: &str, is_dir: bool) -> bool {
    if pattern.ends_with('/') && !is_dir {
        return false;
    }

    let clean_pattern = pattern.trim_end_matches('/');

    if !clean_pattern.contains('*') {
        if path == clean_pattern
            || path.starts_with(&format!("{}/", clean_pattern))
            || path.ends_with(&format!("/{}", clean_pattern))
        {
            return true;
        }
    }

    if clean_pattern.contains('*') {
        if clean_pattern.starts_with('*') && clean_pattern.ends_with('*') {
            let inner = clean_pattern.trim_matches('*');
            return path.contains(inner);
        } else if clean_pattern.starts_with('*') {
            let suffix = clean_pattern.trim_start_matches('*');
            return path.ends_with(suffix);
        } else if clean_pattern.ends_with('*') {
            let prefix = clean_pattern.trim_end_matches('*');
            return path.starts_with(prefix);
        } else if clean_pattern.contains('*') {
            let parts: Vec<&str> = clean_pattern.split('*').collect();
            if parts.len() >= 2 {
                return path.starts_with(parts[0])
                    && path.ends_with(parts[parts.len() - 1])
                    && parts[1..parts.len() - 1]
                        .iter()
                        .all(|part| path.contains(part));
            }
        }
    }

    if clean_pattern.starts_with('/') {
        let pattern_without_slash = clean_pattern.trim_start_matches('/');
        return path == pattern_without_slash
            || path.starts_with(&format!("{}/", pattern_without_slash));
    }

    path.contains(clean_pattern)
}

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
            let exclude_match = exclude_patterns.is_empty()
                || !exclude_patterns.iter().any(|pat| path.contains(pat));

            exclude_match
        })
        .filter_map(Result::ok)
    {
        if entry.file_type().is_dir() || entry.file_type().is_symlink() {
            continue;
        }

        let path = entry.path();

        let ext_matches = if extensions.is_empty() {
            true
        } else {
            path.extension()
                .and_then(|e| e.to_str())
                .map(|e| {
                    extensions
                        .iter()
                        .any(|ext| ext.trim_start_matches('.') == e)
                })
                .unwrap_or(false)
        };

        let excluded = !exclude_patterns.is_empty()
            && exclude_patterns
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

pub fn list_code_files_with_gitignore(
    root: &str,
    extensions: &[&str],
    exclude_patterns: &[&str],
    exclude_version_control_dir: &str,
    apply_dot_git_ignore: bool,
) -> anyhow::Result<Vec<PathBuf>> {
    info!("Listing code files in: {} with gitignore support", root);
    debug!("Extensions: {:?}", extensions);
    debug!("Exclude patterns: {:?}", exclude_patterns);
    debug!("Exclude VCS dir: {}", exclude_version_control_dir);
    debug!("Apply .gitignore: {}", apply_dot_git_ignore);

    let mut result = Vec::new();
    let mut all_exclude_patterns = exclude_patterns.to_vec();

    // Add version control directory to exclude patterns
    if !exclude_version_control_dir.is_empty() {
        all_exclude_patterns.push(exclude_version_control_dir);
    }

    // Parse .gitignore if needed
    let gitignore_patterns = if apply_dot_git_ignore {
        parse_gitignore(root)?
    } else {
        HashSet::new()
    };

    let root_path = Path::new(root);

    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            let path = e.path().to_string_lossy();
            let exclude_match = all_exclude_patterns.is_empty()
                || !all_exclude_patterns.iter().any(|pat| path.contains(pat));

            let gitignore_match = if apply_dot_git_ignore {
                !should_ignore_by_gitignore(e.path(), root_path, &gitignore_patterns)
            } else {
                true
            };

            exclude_match && gitignore_match
        })
        .filter_map(Result::ok)
    {
        if entry.file_type().is_dir() || entry.file_type().is_symlink() {
            continue;
        }

        let path = entry.path();

        let ext_matches = if extensions.is_empty() {
            true
        } else {
            path.extension()
                .and_then(|e| e.to_str())
                .map(|e| {
                    extensions
                        .iter()
                        .any(|ext| ext.trim_start_matches('.') == e)
                })
                .unwrap_or(false)
        };

        if ext_matches {
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

pub fn generate_file_map(
    root: &str,
    exclude_patterns: &[&str],
    exclude_version_control_dir: &str,
    apply_dot_git_ignore: bool,
) -> anyhow::Result<String> {
    info!("Generating file map for: {}", root);
    let mut output = String::new();

    let mut all_exclude_patterns = exclude_patterns.to_vec();

    if !exclude_version_control_dir.is_empty() {
        all_exclude_patterns.push(exclude_version_control_dir);
    }

    let gitignore_patterns = if apply_dot_git_ignore {
        parse_gitignore(root)?
    } else {
        HashSet::new()
    };

    let dir_map = list_dir_structure_with_gitignore(
        root,
        &all_exclude_patterns,
        &gitignore_patterns,
        apply_dot_git_ignore,
    )?;

    for (dir, files) in &dir_map {
        output.push_str(&format!("{}\n", dir));
        for file in files {
            output.push_str(&format!("├── {}\n", file));
        }
    }

    debug!("Generated file map with {} directories", dir_map.len());
    Ok(output)
}

pub fn list_dir_structure_with_gitignore(
    root: &str,
    exclude_patterns: &[&str],
    gitignore_patterns: &HashSet<String>,
    apply_dot_git_ignore: bool,
) -> anyhow::Result<HashMap<String, Vec<String>>> {
    debug!(
        "Listing directory structure in: {} with gitignore support",
        root
    );
    let mut dir_map = HashMap::new();
    let root_path = Path::new(root);

    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            let path = e.path().to_string_lossy();
            let exclude_match = exclude_patterns.is_empty()
                || !exclude_patterns.iter().any(|pat| path.contains(pat));

            let gitignore_match = if apply_dot_git_ignore {
                !should_ignore_by_gitignore(e.path(), root_path, gitignore_patterns)
            } else {
                true
            };

            exclude_match && gitignore_match
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

    #[test]
    fn test_parse_gitignore() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_string_lossy().to_string();
        let gitignore_path = temp_dir.path().join(".gitignore");

        {
            let mut file = File::create(&gitignore_path).unwrap();
            writeln!(file, "# Comment line").unwrap();
            writeln!(file, "node_modules/").unwrap();
            writeln!(file, "*.log").unwrap();
            writeln!(file, "build").unwrap();
            writeln!(file, "").unwrap();
            writeln!(file, "/dist").unwrap();
            writeln!(file, "temp*").unwrap();
            writeln!(file, "!important.log").unwrap();
            writeln!(file, "**/coverage").unwrap();
        }

        let patterns = parse_gitignore(&root).unwrap();

        assert_eq!(patterns.len(), 7);
        assert!(patterns.contains("node_modules/"));
        assert!(patterns.contains("*.log"));
        assert!(patterns.contains("build"));
        assert!(patterns.contains("/dist"));
        assert!(patterns.contains("temp*"));
        assert!(patterns.contains("!important.log"));
        assert!(patterns.contains("**/coverage"));
    }

    #[test]
    fn test_matches_gitignore_pattern() {
        assert!(matches_gitignore_pattern("test.log", "*.log", false));
        assert!(matches_gitignore_pattern("logs/test.log", "*.log", false));
        assert!(matches_gitignore_pattern(
            "node_modules/package.json",
            "node_modules/",
            true
        ));
        assert!(!matches_gitignore_pattern(
            "node_modules.txt",
            "node_modules/",
            false
        ));
        assert!(matches_gitignore_pattern("dist/main.js", "/dist", false));
        assert!(matches_gitignore_pattern("temp", "temp*", false));
        assert!(matches_gitignore_pattern("temporary.txt", "temp*", false));
        assert!(matches_gitignore_pattern(
            "src/coverage/report.html",
            "**/coverage",
            false
        ));
        assert!(matches_gitignore_pattern("abc.xyz", "*.xy*", false));
        assert!(matches_gitignore_pattern("a/b/c.txt", "**/c.txt", false));

        assert!(!matches_gitignore_pattern(
            "node_modules.txt",
            "node_modules/",
            false
        ));
        assert!(matches_gitignore_pattern(
            "node_modules",
            "node_modules/",
            true
        ));
    }

    #[test]
    fn test_should_ignore_by_gitignore() {
        let root = Path::new("/test");
        let mut patterns = HashSet::new();

        patterns.insert("node_modules/".to_string());
        patterns.insert("*.log".to_string());
        patterns.insert("build".to_string());
        patterns.insert("/dist".to_string());
        patterns.insert("temp*".to_string());
        patterns.insert("!important.log".to_string());

        assert!(should_ignore_by_gitignore(
            &Path::new("/test/node_modules/file.js"),
            root,
            &patterns
        ));
        assert!(should_ignore_by_gitignore(
            &Path::new("/test/logs/server.log"),
            root,
            &patterns
        ));
        assert!(should_ignore_by_gitignore(
            &Path::new("/test/build/index.js"),
            root,
            &patterns
        ));
        assert!(should_ignore_by_gitignore(
            &Path::new("/test/dist/main.js"),
            root,
            &patterns
        ));
        assert!(should_ignore_by_gitignore(
            &Path::new("/test/temporary.txt"),
            root,
            &patterns
        ));

        assert!(!should_ignore_by_gitignore(
            &Path::new("/test/logs/important.log"),
            root,
            &patterns
        ));

        assert!(!should_ignore_by_gitignore(
            &Path::new("/test/src/index.js"),
            root,
            &patterns
        ));
        assert!(!should_ignore_by_gitignore(
            &Path::new("/test/package.json"),
            root,
            &patterns
        ));
    }
}
