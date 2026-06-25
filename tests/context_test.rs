use srr::context::ContextWriter;
use srr::context::markdown::MarkdownWriter;
use srr::context::{compute_base_metrics, finalize_metrics};
use srr::config::Config;
use srr::types::*;
use std::path::PathBuf;

fn make_state() -> ProjectState {
    ProjectState {
        project_name: "test".to_string(),
        primary_language: "Rust".to_string(),
        files: vec![],
        total_tokens: 1000,
        duplicate_groups: vec![],
        near_duplicate_groups: vec![],
        patterns: vec![],
        architecture: Architecture {
            layers: vec![LayerInfo {
                name: "Backend".to_string(),
                file_count: 1,
                technologies: vec!["Rust".to_string()],
            }],
            graph_dot: "digraph {}".to_string(),
            hierarchy_text: "  Backend".to_string(),
        },
        scores: vec![
            ScoredFile { path: PathBuf::from("main.rs"), score: 100.0, token_count: 50 },
            ScoredFile { path: PathBuf::from("lib.rs"), score: 80.0, token_count: 100 },
        ],
        clusters: vec![
            Cluster {
                name: "Auth".to_string(),
                description: "Authentication module".to_string(),
                files: vec![PathBuf::from("auth.rs")],
            },
        ],
        log_summary: None,
        doc_summary: None,
        metrics: CompressionMetrics::default(),
    }
}

fn make_config() -> Config {
    Config {
        path: PathBuf::from("."),
        output: None,
        json: false,
        verbose: false,
        exclude: vec![],
        max_tokens: None,
        summary_level: SummaryLevel::Detailed,
        model: ModelType::Gpt4o,
        warnings: vec![],
        respect_gitignore: false,
        include_glob: None,
        gzip: false,
        quiet: false,
        no_color: false,
        concurrency: None,
        dry_run: false,
    }
}

#[test]
fn test_markdown_contains_sections() {
    let writer = MarkdownWriter;
    let state = make_state();
    let config = make_config();
    let base = compute_base_metrics(&state, &config);
    let result = writer.generate(&state, &config, &base);

    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.text.contains("# Project Summary"));
    assert!(output.text.contains("# Architecture Overview"));
    assert!(output.text.contains("# Key Components"));
    assert!(output.text.contains("# Important Files"));
    assert!(output.text.contains("# Semantic Clusters"));
    assert!(output.text.contains("# File Importance Rankings"));
    assert!(output.text.contains("# Compression Metrics"));
}

#[test]
fn test_json_output_valid() {
    use srr::context::json::JsonWriter;

    let writer = JsonWriter;
    let state = make_state();
    let config = make_config();
    let base = compute_base_metrics(&state, &config);
    let result = writer.generate(&state, &config, &base);

    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.text.starts_with('{') || output.text.starts_with("{\n"));
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&output.text);
    assert!(parsed.is_ok());
}

#[test]
fn test_metrics_calculated() {
    let state = make_state();
    let config = make_config();
    let base = compute_base_metrics(&state, &config);
    let analysis = "# Test\nSmall output text for testing.";
    let mut metrics = base;
    finalize_metrics(&mut metrics, analysis);

    assert!(metrics.original_tokens > 0);
    assert!(metrics.compressed_tokens > 0);
}

#[test]
fn test_max_tokens_respected() {
    let writer = MarkdownWriter;
    let state = make_state();

    let mut config = make_config();
    config.max_tokens = Some(100);
    config.summary_level = SummaryLevel::Compact;

    let base = compute_base_metrics(&state, &config);
    let result = writer.generate(&state, &config, &base);
    assert!(result.is_ok());
}
