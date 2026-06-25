use std::path::PathBuf;
use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::Deserialize;
use crate::cli::Cli;
use crate::types::{SummaryLevel, ModelType};

#[derive(Debug, Clone)]
pub struct Config {
    pub path: PathBuf,
    pub output: Option<PathBuf>,
    pub json: bool,
    pub verbose: bool,
    pub exclude: Vec<String>,
    pub max_tokens: Option<usize>,
    pub summary_level: SummaryLevel,
    pub model: ModelType,
    pub warnings: Vec<String>,
    pub respect_gitignore: bool,
    pub include_glob: Option<GlobSet>,
    pub gzip: bool,
    pub quiet: bool,
    pub no_color: bool,
    pub concurrency: Option<usize>,
    pub dry_run: bool,
}

#[derive(Debug, Deserialize)]
struct ConfigFile {
    exclude: Option<Vec<String>>,
    respect_gitignore: Option<bool>,
    summary_level: Option<String>,
    model: Option<String>,
    max_tokens: Option<usize>,
    include: Option<Vec<String>>,
    gzip: Option<bool>,
    quiet: Option<bool>,
    no_color: Option<bool>,
    concurrency: Option<usize>,
    dry_run: Option<bool>,
    output: Option<String>,
    json: Option<bool>,
    verbose: Option<bool>,
}

impl Config {
    pub fn from_cli(cli: &Cli) -> Self {
        let mut config = Config::from_cli_inner(cli);
        config.merge_config_file(cli.config.as_deref());
        config
    }

    fn from_cli_inner(cli: &Cli) -> Self {
        let exclude = cli.exclude
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let mut warnings = Vec::new();

        let summary_level = match cli.summary_level.to_lowercase().as_str() {
            "compact" => SummaryLevel::Compact,
            "detailed" => SummaryLevel::Detailed,
            other => {
                warnings.push(format!("Unknown summary level '{}', using 'detailed'", other));
                SummaryLevel::Detailed
            }
        };

        let model = match cli.model.to_lowercase().as_str() {
            "gpt4o" | "gpt-4o" => ModelType::Gpt4o,
            "claude" | "claude3.5" | "claude-3.5" => ModelType::Claude35Sonnet,
            "gemini" | "gemini1.5" | "gemini-1.5" => ModelType::Gemini15Pro,
            other => {
                warnings.push(format!("Unknown model '{}', using 'gpt4o'", other));
                ModelType::Gpt4o
            }
        };

        let output = cli.output.clone().map(PathBuf::from);

        let include_glob = cli.include.as_ref().map(|patterns| {
            let mut builder = GlobSetBuilder::new();
            for pat in patterns.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
                match Glob::new(pat) {
                    Ok(glob) => { builder.add(glob); }
                    Err(e) => {
                        warnings.push(format!("Invalid include glob pattern '{}': {}", pat, e));
                    }
                }
            }
            let set = builder.build().unwrap_or_else(|_| GlobSetBuilder::new().build().unwrap());
            if set.is_empty() {
                warnings.push("Include glob set is empty — all files will be excluded.".to_string());
            }
            set
        });

        Config {
            path: PathBuf::from(&cli.path),
            output,
            json: cli.json,
            verbose: cli.verbose,
            exclude,
            max_tokens: cli.max_tokens,
            summary_level,
            model,
            warnings,
            respect_gitignore: cli.respect_gitignore,
            include_glob,
            gzip: cli.gzip,
            quiet: cli.quiet,
            no_color: cli.no_color,
            concurrency: cli.concurrency,
            dry_run: cli.dry_run,
        }
    }

    fn merge_config_file(&mut self, explicit_path: Option<&str>) {
        let file_path = find_config_file(explicit_path);
        let file_path = match file_path {
            Some(p) => p,
            None => return,
        };

        let content = match std::fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(_) => {
                if explicit_path.is_some() {
                    self.warnings.push(format!("Config file not found: {}", file_path.display()));
                }
                return;
            }
        };

        let file_cfg: ConfigFile = match toml::from_str(&content) {
            Ok(c) => c,
            Err(e) => {
                self.warnings.push(format!("Failed to parse config file '{}': {}", file_path.display(), e));
                return;
            }
        };

        let explicit_flags: Vec<String> = std::env::args()
            .filter(|a| a.starts_with("--"))
            .collect();

        let is_explicit = |flag: &str| explicit_flags.iter().any(|a| a == flag);

        if !is_explicit("--exclude") {
            if let Some(ref excl) = file_cfg.exclude {
                self.exclude = excl.clone();
            }
        }
        if !is_explicit("--respect-gitignore") && !is_explicit("-g") {
            if let Some(v) = file_cfg.respect_gitignore { self.respect_gitignore = v; }
        }
        if !is_explicit("--summary-level") {
            if let Some(ref lvl) = file_cfg.summary_level {
                match lvl.to_lowercase().as_str() {
                    "compact" => self.summary_level = SummaryLevel::Compact,
                    "detailed" => self.summary_level = SummaryLevel::Detailed,
                    other => self.warnings.push(format!("Unknown summary level '{}' in config", other)),
                }
            }
        }
        if !is_explicit("--model") {
            if let Some(ref m) = file_cfg.model {
                match m.to_lowercase().as_str() {
                    "gpt4o" | "gpt-4o" => self.model = ModelType::Gpt4o,
                    "claude" | "claude3.5" | "claude-3.5" => self.model = ModelType::Claude35Sonnet,
                    "gemini" | "gemini1.5" | "gemini-1.5" => self.model = ModelType::Gemini15Pro,
                    other => self.warnings.push(format!("Unknown model '{}' in config", other)),
                }
            }
        }
        if !is_explicit("--max-tokens") {
            if let Some(t) = file_cfg.max_tokens { self.max_tokens = Some(t); }
        }
        if !is_explicit("--include") && !is_explicit("-i") {
            if let Some(ref patterns) = file_cfg.include {
                let mut builder = GlobSetBuilder::new();
                for pat in patterns {
                    match Glob::new(pat) {
                        Ok(glob) => { builder.add(glob); }
                        Err(e) => { self.warnings.push(format!("Invalid include glob pattern '{}' in config: {}", pat, e)); }
                    }
                }
                let set = builder.build().unwrap_or_else(|_| GlobSetBuilder::new().build().unwrap());
                if set.is_empty() && !patterns.is_empty() {
                    self.warnings.push("Include glob set is empty — all files will be excluded.".to_string());
                }
                self.include_glob = Some(set);
            }
        }
        if !is_explicit("--gzip") {
            if let Some(v) = file_cfg.gzip { self.gzip = v; }
        }
        if !is_explicit("--quiet") && !is_explicit("-q") {
            if let Some(v) = file_cfg.quiet { self.quiet = v; }
        }
        if !is_explicit("--no-color") {
            if let Some(v) = file_cfg.no_color { self.no_color = v; }
        }
        if !is_explicit("--concurrency") {
            if let Some(c) = file_cfg.concurrency { self.concurrency = Some(c); }
        }
        if !is_explicit("--dry-run") {
            if let Some(v) = file_cfg.dry_run { self.dry_run = v; }
        }
        if !is_explicit("--output") && !is_explicit("-o") {
            if let Some(ref o) = file_cfg.output { self.output = Some(PathBuf::from(o)); }
        }
        if !is_explicit("--json") {
            if let Some(v) = file_cfg.json { self.json = v; }
        }
        if !is_explicit("--verbose") && !is_explicit("-v") {
            if let Some(v) = file_cfg.verbose { self.verbose = v; }
        }
    }
}

fn find_config_file(explicit_path: Option<&str>) -> Option<PathBuf> {
    if let Some(path) = explicit_path {
        let p = PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }
    let cwd = std::env::current_dir().ok()?;
    let local = cwd.join(".srrrc");
    if local.exists() {
        return Some(local);
    }
    #[allow(deprecated)]
    let home = std::env::home_dir();
    if let Some(home_dir) = home {
        let home_cfg = home_dir.join(".srrrc");
        if home_cfg.exists() {
            return Some(home_cfg);
        }
    }
    None
}
