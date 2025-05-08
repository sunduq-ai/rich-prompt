# 🚀 Rich Prompt

> 🧠 Supercharge your LLM interactions with structured context from your codebase!

A Rust CLI tool that transforms your project files into perfectly formatted context blocks for Large Language Models. Ideal for code reviews, refactoring assistance, and technical discussions with AI.

## ✨ Features

- 📁 **Smart File Selection** - Automatically scan and select files with specific extensions
- 🔍 **Interactive Mode** - Choose files interactively or auto-include all matched files
- 🚫 **Exclusion Patterns** - Easily ignore directories like `.git`, `node_modules`, etc.
- 🏗️ **Structured Output** - Generate well-formatted context blocks optimized for LLMs
- 💬 **Custom Instructions** - Include your specific prompts within the context block
- 📤 **Flexible Output** - Print to console, save to file, or copy to clipboard with a simple flag
- 📋 **Clipboard Support** - Copy generated content directly to your clipboard for easy pasting
- 🔄 **GitIgnore Support** - Respect existing `.gitignore` rules when scanning files

## 📦 Installation

### 📥 From Cargo

```bash
cargo install rich-prompt
```

### 🔧 From Source

```bash
git clone https://github.com/username/rich-prompt.git
cd rich-prompt
cargo install --path .
```

## 🎮 Usage

### 🔰 Basic Usage

```bash
rich-prompt generate --path /path/to/project
```

### ⚙️ Command Line Options

| Option | Description |
|--------|-------------|
| `--path` | 📂 Root directory to scan (required) |
| `--ext` | 📑 File extensions to include (optional, include all files if not specified) |
| `--exclude` | 🚫 Patterns to exclude (optional, exclude none if not specified) |
| `--output` | 💾 File path to save output (optional) |
| `--auto` | 🤖 Skip interactive selection, include all files |
| `--prompt` | 💬 User prompt to include in context block |
| `--exclude-version-control-dir` | 📂 Version control directory to exclude (default: `.git`) |
| `--apply-dot-git-ignore` | 🔍 Whether to apply .gitignore rules (default: `true`) |
| `--clipboard-output` | 📋 Copy the output to the clipboard |
| `--verbose` | 📝 Increase logging verbosity (-v, -vv, -vvv) |

### 🌟 Examples

#### Include all Rust files in a project:

```bash
rich-prompt generate --path ./my-project --ext .rs --exclude target --auto
```

#### Include selected JavaScript and TypeScript files:

```bash
rich-prompt generate --path ./frontend --ext .js,.ts --exclude node_modules --output output.txt
```

#### Include a custom prompt with your file context:

```bash
rich-prompt generate --path ./src --prompt "Optimize this code for performance and reduce memory usage"
```

#### Copy the output directly to clipboard:

```bash
rich-prompt generate --path ./src --auto --clipboard-output
```

## 📋 Output Format

The tool generates output in the following format:

````
<file_map>
# 📂 Directory structure representation
</file_map>

<file_contents>
File: path/to/file.ext
```ext
file content
```
</file_contents>

<user_instructions>
💬 Your custom prompt goes here
</user_instructions>
````

## 🎯 Use Cases

- 🔍 **Code Reviews**: Get AI feedback on your code quality and structure
- 📚 **Documentation**: Generate comprehensive docs with AI assistance
- 🛠️ **Refactoring**: Receive intelligent suggestions for code improvements
- 🧩 **Problem Solving**: Get AI help with complex coding challenges
- 🎓 **Learning**: Analyze and understand project structure with AI explanations

## 🔄 Workflow Integration

Perfect for integrating with:

- 💻 CI/CD pipelines
- 🤖 AI code review bots
- 📊 Documentation generators
- 🧪 Testing frameworks

## 🚦 Logging Levels

Control verbosity with the `--verbose` flag:

- Default: Only errors
- `-v`: Warnings and errors
- `-vv`: Info, warnings, and errors
- `-vvv`: All debug information

## 🛠️ Advanced Configuration

Create a `.rich-prompt.toml` in your home directory to set default options:

```toml
default_extensions = [".rs", ".toml"]
default_excludes = [".git", "target", "node_modules"]
log_level = "info"
```

## 🤝 Contributing

Contributions are welcome! Here's how you can help:

1. 🍴 Fork the repository
2. 🔄 Create a feature branch
3. 💻 Add your changes
4. 🧪 Add tests for your changes
5. 📤 Submit a pull request

Please make sure your code follows our coding standards and includes appropriate tests.

## 📜 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

<p align="center">
  Made with ❤️ by Mohamed Abdelwahed
</p>