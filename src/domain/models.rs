use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct FileContext {
    pub path: PathBuf,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct ContextConfig {
    pub root_path: String,
    pub extensions: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub output_path: Option<String>,
    pub auto_select: bool,
    pub user_prompt: Option<String>,
    pub exclude_version_control_dir: String,
    pub apply_dot_git_ignore: bool,
    pub clipboard_output: bool,
}

#[derive(Debug)]
pub struct ContextOutput {
    pub file_map: String,
    pub file_contents: String,
    pub user_instructions: String,
    pub token_count: usize,
}
