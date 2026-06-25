use srr::scoring::FileScorer;
use srr::scoring::ranker::ImportanceRanker;
use srr::types::{FileEntry, Architecture};
use std::path::PathBuf;

fn make_entry(path: &str, content: Option<&str>) -> FileEntry {
    FileEntry {
        path: PathBuf::from(path),
        relative_path: PathBuf::from(path),
        extension: path.rsplit('.').next().unwrap_or("").to_string(),
        size_bytes: content.map(|c| c.len() as u64).unwrap_or(0),
        line_count: content.map(|c| c.lines().count()).unwrap_or(0),
        is_binary: false,
        content: content.map(|c| c.to_string()),
        token_count: content.map(|c| c.len() / 4).unwrap_or(0),
    }
}

#[test]
fn test_entry_point_scores_highest() {
    let files = vec![
        make_entry("/project/src/main.rs", Some("fn main() {}")),
        make_entry("/project/src/helper.rs", Some("fn helper() {}")),
    ];

    let arch = Architecture {
        layers: vec![],
        graph_dot: String::new(),
        hierarchy_text: String::new(),
    };

    let ranker = ImportanceRanker;
    let scores = ranker.score(&files, &arch);
    assert_eq!(scores.len(), 2);
    assert!(scores[0].score >= scores[1].score);
}

#[test]
fn test_scoring_empty() {
    let ranker = ImportanceRanker;
    let arch = Architecture {
        layers: vec![],
        graph_dot: String::new(),
        hierarchy_text: String::new(),
    };
    let scores = ranker.score(&[], &arch);
    assert_eq!(scores.len(), 0);
}

#[test]
fn test_scores_normalized_0_to_100() {
    let files = vec![
        make_entry("/project/src/main.rs", Some("fn main() {}")),
        make_entry("/project/src/lib.rs", Some("pub fn foo() {}")),
        make_entry("/project/README.md", Some("# Documentation")),
    ];

    let arch = Architecture {
        layers: vec![],
        graph_dot: String::new(),
        hierarchy_text: String::new(),
    };

    let ranker = ImportanceRanker;
    let scores = ranker.score(&files, &arch);

    for score in &scores {
        assert!(score.score >= 0.0, "Score {} is below 0", score.score);
        assert!(score.score <= 100.0, "Score {} is above 100", score.score);
    }
}
