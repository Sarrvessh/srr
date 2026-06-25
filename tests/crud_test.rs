use std::path::PathBuf;
use srr::pattern::PatternDetector;
use srr::pattern::crud::CrudDetector;
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
fn test_crud_detects_create_read_delete() {
    let files = vec![make_file("src/users.rs", "
pub fn create_user() {}
pub fn get_user() {}
pub fn delete_user() {}
pub fn update_user() {}
")];
    let detector = CrudDetector;
    let patterns = detector.detect_patterns(&files);
    assert!(!patterns.is_empty(), "Should detect CRUD patterns");
    let user_patterns: Vec<_> = patterns.iter().filter(|p| p.entity == "user").collect();
    assert!(!user_patterns.is_empty(), "Should detect 'user' entity");
}

#[test]
fn test_crud_ignores_non_crud() {
    let files = vec![make_file("src/utils.rs", "
pub fn format_string(s: &str) -> String { s.to_string() }
pub fn calculate(a: i32, b: i32) -> i32 { a + b }
pub fn validate_email(email: &str) -> bool { true }
")];
    let detector = CrudDetector;
    let patterns = detector.detect_patterns(&files);
    let non_empty_entity: Vec<_> = patterns.iter().filter(|p| !p.entity.is_empty()).collect();
    assert!(non_empty_entity.is_empty() || non_empty_entity.iter().all(|p| p.operations.len() < 2));
}

#[test]
fn test_crud_empty_input() {
    let detector = CrudDetector;
    let patterns = detector.detect_patterns(&[]);
    assert!(patterns.is_empty());
}

#[test]
fn test_crud_handles_camelcase_entity() {
    let files = vec![make_file("src/api.rs", "
pub fn createUser() {}
pub fn getUser() {}
pub fn updateUser() {}
pub fn deleteUser() {}
")];
    let detector = CrudDetector;
    let patterns = detector.detect_patterns(&files);
    let user_patterns: Vec<_> = patterns.iter().filter(|p| p.entity == "user").collect();
    assert!(!user_patterns.is_empty(), "Should extract 'user' from camelCase 'User'");
}

#[test]
fn test_crud_requires_two_or_more_operations() {
    let files = vec![make_file("src/single.rs", "
pub fn create_single() {}
")];
    let detector = CrudDetector;
    let patterns = detector.detect_patterns(&files);
    let single: Vec<_> = patterns.iter().filter(|p| p.entity == "single").collect();
    assert!(single.is_empty() || single.iter().all(|p| p.operations.len() < 2));
}
