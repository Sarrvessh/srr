use srr::config::Config;
use srr::cli::Cli;
use srr::types::SummaryLevel;

#[test]
fn test_config_gzip_flag() {
    let cli = clap::Parser::try_parse_from([
        "compress-context", ".", "--gzip"
    ]).unwrap();
    let config = Config::from_cli(&cli);
    assert!(config.gzip);
}

fn make_cli(path: &str) -> Cli {
    clap::Parser::try_parse_from(["compress-context", path]).unwrap()
}

#[test]
fn test_config_default_values() {
    let cli = make_cli(".");
    let config = Config::from_cli(&cli);
    assert_eq!(config.path.to_string_lossy(), ".");
    assert!(config.output.is_none());
    assert!(!config.json);
    assert!(!config.verbose);
    assert!(!config.respect_gitignore);
    assert!(!config.quiet);
    assert!(!config.no_color);
    assert!(!config.dry_run);
    assert!(config.concurrency.is_none());
    assert!(config.include_glob.is_none());
    assert_eq!(config.summary_level, SummaryLevel::Detailed);
}

#[test]
fn test_config_unknown_summary_level_warns() {
    let cli = clap::Parser::try_parse_from([
        "compress-context", ".",
        "--summary-level", "invalid_level"
    ]).unwrap();
    let config = Config::from_cli(&cli);
    assert_eq!(config.summary_level, SummaryLevel::Detailed);
    assert!(config.warnings.iter().any(|w| w.contains("summary level")));
}

#[test]
fn test_config_unknown_model_warns() {
    let cli = clap::Parser::try_parse_from([
        "compress-context", ".",
        "--model", "nonexistent"
    ]).unwrap();
    let config = Config::from_cli(&cli);
    assert!(config.warnings.iter().any(|w| w.contains("model")));
}

#[test]
fn test_config_compact_summary_level() {
    let cli = clap::Parser::try_parse_from([
        "compress-context", ".",
        "--summary-level", "compact"
    ]).unwrap();
    let config = Config::from_cli(&cli);
    assert_eq!(config.summary_level, SummaryLevel::Compact);
    assert!(config.warnings.is_empty());
}

#[test]
fn test_config_respect_gitignore_flag() {
    let cli = clap::Parser::try_parse_from([
        "compress-context", ".",
        "--respect-gitignore"
    ]).unwrap();
    let config = Config::from_cli(&cli);
    assert!(config.respect_gitignore);
}

#[test]
fn test_config_include_glob() {
    let cli = clap::Parser::try_parse_from([
        "compress-context", ".",
        "--include", "**/*.rs,**/*.toml"
    ]).unwrap();
    let config = Config::from_cli(&cli);
    assert!(config.include_glob.is_some());
}

#[test]
fn test_config_max_tokens() {
    let cli = clap::Parser::try_parse_from([
        "compress-context", ".",
        "--max-tokens", "5000"
    ]).unwrap();
    let config = Config::from_cli(&cli);
    assert_eq!(config.max_tokens, Some(5000));
}

#[test]
fn test_config_quiet_no_color_dry_run_concurrency() {
    let cli = clap::Parser::try_parse_from([
        "compress-context", ".",
        "--quiet", "--no-color", "--dry-run", "--concurrency", "4"
    ]).unwrap();
    let config = Config::from_cli(&cli);
    assert!(config.quiet);
    assert!(config.no_color);
    assert!(config.dry_run);
    assert_eq!(config.concurrency, Some(4));
}

#[test]
fn test_config_invalid_include_glob_warns() {
    let cli = clap::Parser::try_parse_from([
        "compress-context", ".",
        "--include", "[invalid"
    ]).unwrap();
    let config = Config::from_cli(&cli);
    assert!(config.warnings.iter().any(|w| w.contains("glob")));
}
