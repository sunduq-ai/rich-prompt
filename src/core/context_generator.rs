use crate::infra::file_system::{read_file_contents, list_dir_structure, list_code_files};
use crate::infra::output::write_output;

fn count_tokens(content: &str) -> usize {
    content.split_whitespace().count()
}

pub fn generate_context(
    path: &str,
    extensions: &[&str],
    exclude: &[&str],
    output_path: Option<String>,
    auto: bool,
) -> anyhow::Result<()> {
    let files = list_code_files(path, extensions, exclude)?;
    let dir_structure = list_dir_structure(path, exclude)?;

    if files.is_empty() {
        println!("No files found with the specified extensions.");
        return Ok(());
    }

    let selected_files = if auto {
        files
    } else {
        let options = files
            .iter()
            .map(|f| f.to_string_lossy().to_string())
            .collect::<Vec<_>>();
        let selection = dialoguer::MultiSelect::new()
            .with_prompt("Select files to include")
            .items(&options)
            .interact()?;
        selection.into_iter().map(|i| files[i].clone()).collect()
    };

    let mut output = String::new();
    output.push_str("<file_map>\n");
    for (dir, files) in &dir_structure {
        output.push_str(&format!("{}\n", dir));
        for file in files {
            output.push_str(&format!("├── {}\n", file));
        }
    }
    output.push_str("</file_map>\n\n\n");

    output.push_str("<file_contents>");
    let mut total_tokens = 0;

    for file in &selected_files {
        if !file.exists() {
            println!("File does not exist: {}", file.display());
            continue;
        }
        if !file.is_file() {
            println!("Not a file: {}", file.display());
            continue;
        }
        let content = read_file_contents(file)?;
        let tokens = count_tokens(&content);
        total_tokens += tokens;

        output.push_str(&format!(
            "\nFile: {}\n```{}\n{}\n```\n",
            file.display(),
            file.extension().and_then(|e| e.to_str()).unwrap_or(""),
            content
        ));
    }
    output.push_str("</file_contents>");
    output.push_str("\n\n<user_instructions>\n\n</user_instructions>");

    println!("Total tokens: {}", total_tokens);

    write_output(&output, output_path)
}
