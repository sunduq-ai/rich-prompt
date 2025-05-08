use crate::domain::models::{ContextOutput, FileContext};
use log::{debug, info};

fn count_tokens(content: &str) -> usize {
    content.split_whitespace().count()
}

pub fn build_context_output(
    files: Vec<FileContext>,
    file_map: String,
    user_prompt: Option<String>,
) -> ContextOutput {
    debug!("Building context output from {} files", files.len());
    let mut file_contents = String::new();
    let mut total_tokens = 0;

    for file in &files {
        let tokens = count_tokens(&file.content);
        total_tokens += tokens;

        debug!("Adding file {} with {} tokens", file.path.display(), tokens);
        file_contents.push_str(&format!(
            "\nFile: {}\n```{}\n{}\n```\n",
            file.path.display(),
            file.path.extension().and_then(|e| e.to_str()).unwrap_or(""),
            file.content
        ));
    }

    let user_instructions = match user_prompt {
        Some(prompt) => {
            info!("Including user prompt in context");
            prompt
        }
        None => {
            debug!("No user prompt provided");
            String::new()
        }
    };

    ContextOutput {
        file_map,
        file_contents,
        user_instructions,
        token_count: total_tokens,
    }
}

pub fn format_output(output: &ContextOutput) -> String {
    debug!(
        "Formatting context output with {} tokens",
        output.token_count
    );
    let mut result = String::new();

    result.push_str("<file_map>\n");
    result.push_str(&output.file_map);
    result.push_str("</file_map>\n\n\n");

    result.push_str("<file_contents>");
    result.push_str(&output.file_contents);
    result.push_str("</file_contents>");

    if !output.user_instructions.is_empty() {
        result.push_str("\n\n<user_instructions>\n");
        result.push_str(&output.user_instructions);
        result.push_str("\n</user_instructions>");
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_count_tokens() {
        assert_eq!(count_tokens("hello world"), 2);
        assert_eq!(count_tokens(""), 0);
        assert_eq!(count_tokens("one\ntwo\nthree"), 3);
    }

    #[test]
    fn test_build_context_output() {
        let files = vec![
            FileContext {
                path: PathBuf::from("test/file1.rs"),
                content: "fn test() {}".to_string(),
            },
            FileContext {
                path: PathBuf::from("test/file2.rs"),
                content: "struct Test {}".to_string(),
            },
        ];

        let file_map = "test\n├── test/file1.rs\n├── test/file2.rs\n".to_string();
        let user_prompt = Some("Refactor this code".to_string());

        let output = build_context_output(files, file_map, user_prompt);

        assert_eq!(output.token_count, 6);
        assert_eq!(output.user_instructions, "Refactor this code");
        assert!(output.file_contents.contains("fn test() {}"));
        assert!(output.file_contents.contains("struct Test {}"));
    }

    #[test]
    fn test_format_output() {
        let output = ContextOutput {
            file_map: "dir1\n".to_string(),
            file_contents: "content1\n".to_string(),
            user_instructions: "prompt1".to_string(),
            token_count: 3,
        };

        let formatted = format_output(&output);

        assert!(formatted.contains("<file_map>\ndir1\n</file_map>"));
        assert!(formatted.contains("<file_contents>content1\n</file_contents>"));
        assert!(formatted.contains("<user_instructions>\nprompt1\n</user_instructions>"));
    }
}
