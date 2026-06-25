use std::fs;
use srr::scanner::FileScanner;

#[test]
fn test_gitignore_excludes_ignored_files() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("keep.txt"), "keep").unwrap();
    fs::write(dir.path().join("ignore.txt"), "ignore").unwrap();
    fs::write(dir.path().join(".gitignore"), "ignore.txt").unwrap();

    let result = srr::scanner::walker::DefaultScanner
        .scan(dir.path(), &[] as &[String], false, true);
    assert!(result.is_ok(), "Scan should succeed");
    let files = result.unwrap();
    assert!(files.iter().any(|f| f.path.ends_with("keep.txt")),
        "Should include keep.txt");
    assert!(!files.iter().any(|f| f.path.ends_with("ignore.txt")),
        "Should exclude ignore.txt per .gitignore");
}

#[test]
fn test_gitignore_respects_nested_gitignore() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("subdir")).unwrap();
    fs::write(dir.path().join("subdir/.gitignore"), "*.log").unwrap();
    fs::write(dir.path().join("subdir/keep.txt"), "keep").unwrap();
    fs::write(dir.path().join("subdir/debug.log"), "log content").unwrap();
    fs::write(dir.path().join("keep.txt"), "keep at root").unwrap();

    let result = srr::scanner::walker::DefaultScanner
        .scan(dir.path(), &[] as &[String], false, true);
    assert!(result.is_ok());
    let files = result.unwrap();
    assert!(files.iter().any(|f| f.path.ends_with("keep.txt")));
    assert!(files.iter().any(|f| f.path.ends_with("subdir/keep.txt")));
    assert!(!files.iter().any(|f| f.path.ends_with("debug.log")),
        "Should exclude debug.log per nested .gitignore");
}

#[test]
fn test_gitignore_false_does_not_use_gitignore() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("keep.txt"), "keep").unwrap();
    fs::write(dir.path().join("ignore.txt"), "ignore").unwrap();
    fs::write(dir.path().join(".gitignore"), "ignore.txt").unwrap();

    let result = srr::scanner::walker::DefaultScanner
        .scan(dir.path(), &[] as &[String], false, false);
    assert!(result.is_ok());
    let files = result.unwrap();
    assert!(files.iter().any(|f| f.path.ends_with("ignore.txt")),
        "Without --respect-gitignore, .gitignore should not apply");
}

#[test]
fn test_gitignore_excludes_hidden_dirs() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join(".hidden")).unwrap();
    fs::write(dir.path().join(".hidden/data.txt"), "data").unwrap();
    fs::write(dir.path().join("visible.txt"), "visible").unwrap();

    let result = srr::scanner::walker::DefaultScanner
        .scan(dir.path(), &[] as &[String], false, true);
    assert!(result.is_ok());
    let files = result.unwrap();
    assert!(!files.iter().any(|f| f.path.to_string_lossy().contains(".hidden")),
        "Should skip .hidden directory by default via ignore::WalkBuilder");
}

#[test]
fn test_gitignore_non_existent_gitignore() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("keep.txt"), "keep").unwrap();

    let result = srr::scanner::walker::DefaultScanner
        .scan(dir.path(), &[] as &[String], false, true);
    assert!(result.is_ok());
    let files = result.unwrap();
    assert_eq!(files.len(), 1);
}
