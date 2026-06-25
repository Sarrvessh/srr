use std::path::Path;
use std::fs;
use srr::scanner::FileScanner;

#[test]
fn test_scanner_empty_directory() {
    let dir = tempfile::tempdir().unwrap();
    let result = srr::scanner::walker::DefaultScanner
        .scan(dir.path(), &["target".to_string()], false, false);
    assert!(result.is_ok());
    let files = result.unwrap();
    assert_eq!(files.len(), 0);
}

#[test]
fn test_scanner_single_file() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.txt");
    fs::write(&file_path, "hello world").unwrap();

    let result = srr::scanner::walker::DefaultScanner
        .scan(dir.path(), &["target".to_string()], false, false);
    let files = result.unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].extension, "txt");
    assert_eq!(files[0].line_count, 1);
    assert!(!files[0].is_binary);
}

#[test]
fn test_scanner_respects_excludes() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("keep.txt"), "keep").unwrap();
    fs::write(dir.path().join("node_modules"), "skip").unwrap();
    fs::write(dir.path().join("target"), "skip").unwrap();

    let result = srr::scanner::walker::DefaultScanner
        .scan(dir.path(), &["target".to_string(), "node_modules".to_string()], false, false);
    let files = result.unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path.file_name().unwrap(), "keep.txt");
}

#[test]
fn test_scanner_handles_binary() {
    let dir = tempfile::tempdir().unwrap();
    let bin_path = dir.path().join("test.bin");
    let mut bytes = vec![0u8; 100];
    bytes[0] = 0x00;
    fs::write(&bin_path, &bytes).unwrap();

    let result = srr::scanner::walker::DefaultScanner
        .scan(dir.path(), &["target".to_string()], false, false);
    assert!(result.is_ok());
}

#[test]
fn test_scanner_nonexistent_directory() {
    let result = srr::scanner::walker::DefaultScanner
        .scan(Path::new("/nonexistent_path_xyz"),
              &["target".to_string()], false, false);
    assert!(result.is_err());
}
