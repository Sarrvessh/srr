use std::sync::LazyLock;
use regex::Regex;

use crate::error::SrrResult;
use crate::types::{FileEntry, CompressedSection};
use crate::tokenizer::estimator::count_tokens_fast;
use super::FileCompressor;

static HEADING_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)^#{1,6}\s+(.+)$").unwrap());
static CODE_BLOCK_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?ms)```[\s\S]*?```").unwrap());

pub struct DocCompressor;

impl FileCompressor for DocCompressor {
    fn can_handle(&self, file: &FileEntry) -> bool {
        if file.is_binary {
            return false;
        }
        let path_lower = file.path.to_string_lossy().to_lowercase();
        file.extension == "md"
            || file.extension == "markdown"
            || path_lower.contains("/docs/")
            || path_lower.contains("\\docs\\")
            || path_lower.contains("/readme")
            || path_lower.contains("\\readme")
            || path_lower.ends_with("readme.md")
    }

    fn compress(&self, files: &[FileEntry]) -> SrrResult<CompressedSection> {
        let doc_files: Vec<&FileEntry> = files.iter().filter(|f| self.can_handle(f)).collect();

        let mut original_tokens = 0;
        let mut all_sections: Vec<String> = Vec::new();

        for file in &doc_files {
            if let Some(ref content) = file.content {
                original_tokens += file.token_count;
                let clean = CODE_BLOCK_RE.replace_all(content, "[code block]");
                let mut sections: Vec<String> = Vec::new();
                let mut last_pos = 0;
                let mut last_heading = String::new();

                for cap in HEADING_RE.find_iter(&clean) {
                    if !last_heading.is_empty() {
                        let section_text = clean[last_pos..cap.start()].trim();
                        if !section_text.is_empty() {
                            sections.push(format!("**{}:** {}", last_heading, truncate_text(section_text, 200)));
                        }
                    }
                    last_heading = cap.as_str().trim_start_matches('#').trim().to_string();
                    last_pos = cap.end();
                }

                if !last_heading.is_empty() {
                    let section_text = clean[last_pos..].trim();
                    if !section_text.is_empty() {
                        sections.push(format!("**{}:** {}", last_heading, truncate_text(section_text, 200)));
                    }
                }

                if sections.is_empty() {
                    sections.push(truncate_text(clean.trim(), 300));
                }

                let file_header = format!("### {}:\n{}", file.path.display(), sections.join("\n"));
                all_sections.push(file_header);
            }
        }

        let mut output = String::new();
        output.push_str("## Documentation Summary\n\n");

        if doc_files.is_empty() {
            output.push_str("No documentation files found.\n");
        } else {
            output.push_str(&format!("Documents analyzed: {}\n\n", doc_files.len()));
            for section in &all_sections {
                output.push_str(section);
                output.push('\n');
                output.push('\n');
            }
        }

        let compressed_tokens = count_tokens_fast(&output);

        Ok(CompressedSection {
            section_type: "documentation".to_string(),
            content: output,
            original_tokens,
            compressed_tokens,
        })
    }
}

fn truncate_text(text: &str, max_chars: usize) -> String {
    if text.len() <= max_chars {
        return text.to_string();
    }
    let mut result = String::with_capacity(max_chars + 3);
    for c in text.chars() {
        if result.len() + c.len_utf8() > max_chars {
            result.push_str("...");
            break;
        }
        result.push(c);
    }
    result
}
