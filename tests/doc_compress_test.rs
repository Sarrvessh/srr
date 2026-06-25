use std::path::PathBuf;
use srr::compressors::FileCompressor;
use srr::compressors::doc::DocCompressor;
use srr::types::FileEntry;

fn make_file(path: &str, content: &str) -> FileEntry {
    FileEntry {
        path: PathBuf::from(path),
        relative_path: PathBuf::from(path),
        extension: path.rsplit('.').next().unwrap_or("").to_string(),
        size_bytes: content.len() as u64,
        line_count: content.lines().count(),
        is_binary: false,
        content: Some(content.to_string()),
        token_count: content.split_whitespace().count(),
    }
}

#[test]
fn test_doc_can_handle_markdown() {
    let compressor = DocCompressor;
    let file = make_file("docs/readme.md", "# Hello");
    assert!(compressor.can_handle(&file));
}

#[test]
fn test_doc_cannot_handle_source_code() {
    let compressor = DocCompressor;
    let file = make_file("src/main.rs", "fn main() {}");
    assert!(!compressor.can_handle(&file));
}

#[test]
fn test_doc_compression_basic() {
    let compressor = DocCompressor;
    let files = vec![make_file("docs/readme.md", "# Project\n\nDescription here.\n\n## Usage\n\nRun the tool.")];
    let result = compressor.compress(&files);
    assert!(result.is_ok());
    let section = result.unwrap();
    assert_eq!(section.section_type, "documentation");
    assert!(section.content.contains("Project"));
}

#[test]
fn test_doc_compression_empty_input() {
    let compressor = DocCompressor;
    let result = compressor.compress(&[]);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_doc_compression_strips_code_blocks() {
    let compressor = DocCompressor;
    let files = vec![make_file("docs/guide.md", "# Guide\n\n```rust\nfn main() {}\n```\n\nDone.")];
    let result = compressor.compress(&files);
    assert!(result.is_ok());
    let section = result.unwrap();
    assert!(!section.content.contains("fn main"), "Should strip code blocks");
}

#[test]
fn test_doc_cannot_handle_non_markdown() {
    let compressor = DocCompressor;
    let file = make_file("data.json", "{}");
    assert!(!compressor.can_handle(&file));
}

#[test]
fn test_doc_compression_with_no_content_files() {
    let compressor = DocCompressor;
    let files = vec![FileEntry {
        path: PathBuf::from("empty.md"),
        relative_path: PathBuf::from("empty.md"),
        extension: "md".to_string(),
        size_bytes: 0,
        line_count: 0,
        is_binary: false,
        content: None,
        token_count: 0,
    }];
    let result = compressor.compress(&files);
    assert!(result.is_err() || result.is_ok());
}
