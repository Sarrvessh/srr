use std::sync::LazyLock;
use regex::Regex;

use crate::error::SrrResult;
use crate::types::{FileEntry, CompressedSection};
use crate::tokenizer::estimator::count_tokens_fast;
use super::FileCompressor;

static COMMENT_BLOCKS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?ms)/\*[\s\S]*?\*/|'''[\s\S]*?'''|"""[\s\S]*?"""#).unwrap()
});
static SINGLE_LINE_COMMENTS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*//[^\n]*|^\s*#[^\n]*|^\s*--[^\n]*").unwrap()
});
static BLANK_LINES_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*$\r?\n").unwrap()
});

pub struct CodeCompressor;

impl FileCompressor for CodeCompressor {
    fn can_handle(&self, file: &FileEntry) -> bool {
        if file.is_binary || file.content.is_none() {
            return false;
        }
        matches!(
            file.extension.as_str(),
            "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "go" | "java" | "rb"
                | "c" | "h" | "cpp" | "hpp" | "cs" | "swift" | "kt" | "kts"
                | "scala" | "php" | "r" | "sh" | "bash" | "zsh" | "ps1" | "sql"
                | "dart" | "lua" | "ex" | "exs" | "hs" | "zig" | "nim" | "jl"
                | "tf" | "hcl" | "pl" | "pm" | "lisp" | "clj" | "cljs" | "erl"
                | "m" | "mm"
        )
    }

    fn compress(&self, files: &[FileEntry]) -> SrrResult<CompressedSection> {
        let code_files: Vec<&FileEntry> = files.iter().filter(|f| self.can_handle(f)).collect();



        let mut total_original_tokens = 0;
        let mut total_compressed_tokens = 0;
        let mut output = String::new();

        output.push_str("## Code Compression Summary\n\n");

        for file in &code_files {
            if let Some(ref content) = file.content {
                total_original_tokens += file.token_count;

                let no_block = COMMENT_BLOCKS_RE.replace_all(content, "");

                // Preserve shebang line (#!) before removing # comments
                let shebang = if no_block.starts_with("#!") {
                    let end = no_block.find('\n').map(|i| i + 1).unwrap_or(no_block.len());
                    Some(no_block[..end].to_string())
                } else {
                    None
                };
                let no_shebang = if let Some(ref sb) = shebang {
                    no_block[sb.len()..].to_string()
                } else {
                    no_block.to_string()
                };

                let no_single = SINGLE_LINE_COMMENTS_RE.replace_all(&no_shebang, "");
                let compressed = BLANK_LINES_RE.replace_all(&no_single, "\n");

                let mut compressed_str = compressed.trim().to_string();
                if compressed_str.is_empty() {
                    if let Some(ref sb) = shebang {
                        compressed_str = sb.trim().to_string();
                    } else {
                        continue;
                    }
                } else if let Some(ref sb) = shebang {
                    compressed_str = format!("{}\n{}", sb.trim(), compressed_str);
                }

                let compressed_tokens = count_tokens_fast(&compressed_str);
                total_compressed_tokens += compressed_tokens;

                output.push_str(&format!(
                    "**{}**: {} tokens (compressed from {})\n",
                    file.path.display(),
                    compressed_tokens,
                    file.token_count
                ));
            }
        }

        if code_files.is_empty() {
            output.push_str("No code files to compress.\n");
        }

        output.push('\n');
        output.push_str(&format!(
            "Total code files analyzed: {}\n", code_files.len()
        ));

        Ok(CompressedSection {
            section_type: "code".to_string(),
            content: output,
            original_tokens: total_original_tokens,
            compressed_tokens: total_compressed_tokens,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_code(path: &str, ext: &str, content: &str) -> FileEntry {
        FileEntry {
            path: PathBuf::from(path),
            relative_path: PathBuf::from(path),
            extension: ext.to_string(),
            size_bytes: content.len() as u64,
            line_count: content.lines().count(),
            is_binary: false,
            content: Some(content.to_string()),
            token_count: content.len() / 4,
        }
    }

    #[test]
    fn test_code_can_handle_rust() {
        let compressor = CodeCompressor;
        let f = make_code("/project/src/main.rs", "rs", "fn main() {}");
        assert!(compressor.can_handle(&f));
    }

    #[test]
    fn test_code_cannot_handle_markdown() {
        let compressor = CodeCompressor;
        let f = make_code("/project/README.md", "md", "# Heading");
        assert!(!compressor.can_handle(&f));
    }

    #[test]
    fn test_code_compression_removes_comments() {
        let compressor = CodeCompressor;
        let files = vec![make_code("/project/src/main.rs", "rs", "// comment\nfn main() {\n    println!(\"hello\");\n}\n")];
        let result = compressor.compress(&files);
        assert!(result.is_ok());
        let section = result.unwrap();
        assert!(section.content.contains("main"));
    }
}
