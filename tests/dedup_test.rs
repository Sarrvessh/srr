use srr::dedup::ExactDuplicateDetector;
use srr::dedup::NearDuplicateDetector;
use srr::dedup::exact::ExactDuplicateDetectorImpl;
use srr::dedup::near::NearDuplicateDetectorImpl;
use srr::types::FileEntry;
use std::path::PathBuf;

fn make_entry(path: &str, content: Option<&str>, extension: &str) -> FileEntry {
    FileEntry {
        path: PathBuf::from(path),
        relative_path: PathBuf::from(path),
        extension: extension.to_string(),
        size_bytes: content.map(|c| c.len() as u64).unwrap_or(0),
        line_count: content.map(|c| c.lines().count()).unwrap_or(0),
        is_binary: false,
        content: content.map(|c| c.to_string()),
        token_count: content.map(|c| c.len() / 4).unwrap_or(0),
    }
}

#[test]
fn test_exact_duplicates_detected() {
    let files = vec![
        make_entry("a.txt", Some("hello world"), "txt"),
        make_entry("b.txt", Some("hello world"), "txt"),
        make_entry("c.txt", Some("different"), "txt"),
    ];

    let detector = ExactDuplicateDetectorImpl;
    let groups = detector.find_exact_duplicates(&files);
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].files.len(), 2);
}

#[test]
fn test_exact_no_false_positive() {
    let files = vec![
        make_entry("a.txt", Some("hello world"), "txt"),
        make_entry("b.txt", Some("goodbye world"), "txt"),
    ];

    let detector = ExactDuplicateDetectorImpl;
    let groups = detector.find_exact_duplicates(&files);
    assert_eq!(groups.len(), 0);
}

#[test]
fn test_near_duplicates_detected() {
    let files = vec![
        make_entry("a.txt", Some("line one\nline two\nline three\nline four\n"), "txt"),
        make_entry("b.txt", Some("line one\nline two\nline three\nline four\n"), "txt"),
    ];

    let detector = NearDuplicateDetectorImpl;
    let groups = detector.find_near_duplicates(&files);
    // Identical content yields Jaccard = 1.0
    assert!(!groups.is_empty());
    assert_eq!(groups[0].files.len(), 2);
}

#[test]
fn test_near_duplicates_finds_similar() {
    let lines_a: Vec<String> = (1..=15).map(|i| format!("line {}", i)).collect();
    let mut lines_b: Vec<String> = (1..=15).map(|i| format!("line {}", i)).collect();
    lines_b[8] = "CHANGED".to_string(); // change line 9 (0-indexed 8)

    let files = vec![
        make_entry("a.txt", Some(&lines_a.join("\n")), "txt"),
        make_entry("b.txt", Some(&lines_b.join("\n")), "txt"),
    ];

    let detector = NearDuplicateDetectorImpl;
    let groups = detector.find_near_duplicates(&files);
    assert!(!groups.is_empty());
}

#[test]
fn test_dedup_empty_files() {
    let detector = ExactDuplicateDetectorImpl;
    let groups = detector.find_exact_duplicates(&[]);
    assert_eq!(groups.len(), 0);
}
