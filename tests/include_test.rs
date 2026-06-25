use std::path::PathBuf;
use globset::{Glob, GlobSetBuilder};
use srr::scanner::walker::filter_by_include;
use srr::types::FileEntry;

fn make_file(path: &str) -> FileEntry {
    FileEntry {
        path: PathBuf::from(path),
        relative_path: PathBuf::from(path),
        extension: path.rsplit('.').next().unwrap_or("").to_string(),
        size_bytes: 0,
        line_count: 0,
        is_binary: false,
        content: None,
        token_count: 0,
    }
}

fn make_globs(patterns: &[&str]) -> globset::GlobSet {
    let mut builder = GlobSetBuilder::new();
    for p in patterns {
        builder.add(Glob::new(p).unwrap());
    }
    builder.build().unwrap()
}

#[test]
fn test_filter_by_include_rust_files() {
    let files = vec![
        make_file("src/main.rs"),
        make_file("README.md"),
        make_file("Cargo.toml"),
        make_file("src/lib.rs"),
    ];
    let globs = make_globs(&["**/*.rs"]);
    let filtered = filter_by_include(files, &globs);
    assert_eq!(filtered.len(), 2);
    assert!(filtered.iter().all(|f| f.extension == "rs"));
}

#[test]
fn test_filter_by_include_multiple_patterns() {
    let files = vec![
        make_file("src/main.rs"),
        make_file("README.md"),
        make_file("Cargo.toml"),
    ];
    let globs = make_globs(&["**/*.rs", "**/*.toml"]);
    let filtered = filter_by_include(files, &globs);
    assert_eq!(filtered.len(), 2);
}

#[test]
fn test_filter_by_include_empty_globset() {
    let files = vec![make_file("any.txt")];
    let globs = GlobSetBuilder::new().build().unwrap();
    let filtered = filter_by_include(files, &globs);
    assert_eq!(filtered.len(), 0);
}

#[test]
fn test_filter_by_include_empty_files() {
    let files = vec![];
    let globs = make_globs(&["**/*.rs"]);
    let filtered = filter_by_include(files, &globs);
    assert!(filtered.is_empty());
}

#[test]
fn test_filter_by_include_nested_paths() {
    let files = vec![
        make_file("src/api/routes.rs"),
        make_file("src/api/mod.rs"),
        make_file("docs/setup.md"),
    ];
    let globs = make_globs(&["src/api/*"]);
    let filtered = filter_by_include(files, &globs);
    assert_eq!(filtered.len(), 2);
}

#[test]
fn test_filter_by_include_all_match() {
    let files = vec![
        make_file("a.rs"),
        make_file("b.py"),
        make_file("c.js"),
    ];
    let globs = make_globs(&["**/*"]);
    let filtered = filter_by_include(files, &globs);
    assert_eq!(filtered.len(), 3);
}
