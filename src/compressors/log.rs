use std::collections::HashMap;
use std::sync::LazyLock;
use regex::Regex;

use crate::error::SrrResult;
use crate::types::{FileEntry, CompressedSection};
use super::FileCompressor;

static TS_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}").unwrap());
static NUM_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b\d+\b").unwrap());
static IP_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}").unwrap());
static UUID_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}").unwrap());
static LEVEL_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\b(ERROR|WARN(?:ING)?|INFO|DEBUG|TRACE|FATAL|CRITICAL)\b").unwrap());

pub struct LogCompressor;

impl FileCompressor for LogCompressor {
    fn can_handle(&self, file: &FileEntry) -> bool {
        if file.is_binary {
            return false;
        }
        let path_lower = file.path.to_string_lossy().to_lowercase();
        path_lower.ends_with(".log") || path_lower.contains("/logs/") || path_lower.contains("\\logs\\")
    }

    fn compress(&self, files: &[FileEntry]) -> SrrResult<CompressedSection> {
        let log_files: Vec<&FileEntry> = files.iter().filter(|f| self.can_handle(f)).collect();

        let mut original_tokens = 0;
        let mut all_messages: HashMap<(String, String), usize> = HashMap::new();

        for file in &log_files {
            if let Some(ref content) = file.content {
                original_tokens += file.token_count;

                for line in content.lines() {
                    let level = LEVEL_RE.find(line)
                        .map(|m| m.as_str().to_uppercase())
                        .unwrap_or_else(|| {
                            if line.contains("ERR") || line.contains("FAIL") { "ERROR".to_string() }
                            else { "INFO".to_string() }
                        });

                    let template = TS_RE.replace_all(line, "{timestamp}");
                    let template = IP_RE.replace_all(&template, "{ip}");
                    let template = UUID_RE.replace_all(&template, "{uuid}");
                    let template = NUM_RE.replace_all(&template, "{n}");

                    let template = template.trim().to_string();
                    if template.is_empty() { continue; }

                    *all_messages.entry((level, template)).or_insert(0) += 1;
                }
            }
        }

        let mut entries: Vec<_> = all_messages.into_iter().collect();
        entries.sort_by_key(|b| std::cmp::Reverse(b.1));

        let mut output = String::new();
        output.push_str("## Log Compression Summary\n\n");
        output.push_str("| Level | Message | Occurrences |\n");
        output.push_str("|-------|---------|-------------|\n");

        for ((level, template), count) in &entries {
            let truncated = if template.len() > 100 {
                format!("{}...", &template[..97])
            } else {
                template.clone()
            };
            output.push_str(&format!("| {} | `{}` | {} |\n", level, truncated, count));
        }

        if entries.is_empty() {
            output.push_str("No log entries found.\n");
        }

        let compressed_tokens = crate::tokenizer::estimator::count_tokens_fast(&output);

        Ok(CompressedSection {
            section_type: "log".to_string(),
            content: output,
            original_tokens,
            compressed_tokens,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_log(path: &str, content: &str) -> FileEntry {
        FileEntry {
            path: PathBuf::from(path),
            relative_path: PathBuf::from(path),
            extension: "log".to_string(),
            size_bytes: content.len() as u64,
            line_count: content.lines().count(),
            is_binary: false,
            content: Some(content.to_string()),
            token_count: content.len() / 4,
        }
    }

    #[test]
    fn test_log_can_handle() {
        let compressor = LogCompressor;
        let f = make_log("/logs/app.log", "test");
        assert!(compressor.can_handle(&f));
    }

    #[test]
    fn test_log_compression_basic() {
        let compressor = LogCompressor;
        let files = vec![make_log("/logs/app.log", "2024-01-01 INFO Application started")];
        let result = compressor.compress(&files);
        assert!(result.is_ok());
        let section = result.unwrap();
        assert!(section.content.contains("Application started"));
    }

    #[test]
    fn test_log_can_handle_non_log() {
        let compressor = LogCompressor;
        let f = FileEntry {
            path: PathBuf::from("/project/src/main.rs"),
            relative_path: PathBuf::from("/project/src/main.rs"),
            extension: "rs".to_string(),
            size_bytes: 10,
            line_count: 1,
            is_binary: false,
            content: Some("fn main() {}".to_string()),
            token_count: 3,
        };
        assert!(!compressor.can_handle(&f));
    }
}
