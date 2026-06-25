use std::path::Path;

fn get_sample_project_path() -> &'static Path {
    Path::new("examples/sample_project")
}

#[test]
fn test_sample_project_exists() {
    let path = get_sample_project_path();
    assert!(path.exists(), "Sample project directory should exist");
    assert!(path.join("README.md").exists());
    assert!(path.join("Cargo.toml").exists());
    assert!(path.join("src/main.rs").exists());
    assert!(path.join("src/auth/login.rs").exists());
    assert!(path.join("logs/app.log").exists());
    assert!(path.join("duplicate_test/README_COPY.md").exists());
}

#[test]
fn test_full_pipeline_does_not_panic() {
    use srr::pipeline::Pipeline;
    use srr::config::Config;

    let config = Config {
        path: get_sample_project_path().to_path_buf(),
        output: None,
        json: false,
        verbose: false,
        exclude: vec!["target".to_string()],
        max_tokens: None,
        summary_level: srr::types::SummaryLevel::Detailed,
        model: srr::types::ModelType::Gpt4o,
        warnings: vec![],
        respect_gitignore: false,
        include_glob: None,
        gzip: false,
        quiet: false,
        no_color: false,
        concurrency: None,
        dry_run: false,
    };

    let pipeline = Pipeline::new(config);
    let result = pipeline.run();
    assert!(result.is_ok(), "Pipeline should succeed: {:?}", result.err());
}

#[test]
fn test_full_pipeline_json_output() {
    use srr::pipeline::Pipeline;
    use srr::config::Config;

    let config = Config {
        path: get_sample_project_path().to_path_buf(),
        output: None,
        json: true,
        verbose: false,
        exclude: vec!["target".to_string()],
        max_tokens: None,
        summary_level: srr::types::SummaryLevel::Detailed,
        model: srr::types::ModelType::Gpt4o,
        warnings: vec![],
        respect_gitignore: false,
        include_glob: None,
        gzip: false,
        quiet: false,
        no_color: false,
        concurrency: None,
        dry_run: false,
    };

    let pipeline = Pipeline::new(config);
    let result = pipeline.run();
    assert!(result.is_ok(), "JSON pipeline should succeed: {:?}", result.err());

    let output = result.unwrap();
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&output.text);
    assert!(parsed.is_ok(), "Output should be valid JSON");
}
