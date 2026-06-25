use std::time::Instant;
use std::sync::Mutex;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use colored::Colorize;

use crate::config::Config;
use crate::error::SrrResult;
use crate::types::*;
use crate::ui;
use toml::Value;

use crate::scanner::walker::{DefaultScanner, filter_by_include};
use crate::scanner::FileScanner;
use crate::dedup::ExactDuplicateDetector;
use crate::dedup::NearDuplicateDetector;
use crate::dedup::exact::ExactDuplicateDetectorImpl;
use crate::dedup::near::NearDuplicateDetectorImpl;
use crate::pattern::PatternDetector;
use crate::pattern::crud::CrudDetector;
use crate::architecture::ArchitectureAnalyzer;
use crate::architecture::detector::ArchitectureDetector;
use crate::scoring::FileScorer;
use crate::scoring::ranker::ImportanceRanker;
use crate::clustering::FileClusterer;
use crate::clustering::domain::DomainClusterer;
use crate::compressors::FileCompressor;
use crate::compressors::log::LogCompressor;
use crate::compressors::doc::DocCompressor;
use crate::compressors::code::CodeCompressor;
use crate::context::ContextWriter;
use crate::context::{compute_base_metrics, GeneratedOutput};

pub struct Pipeline {
    pub config: Config,
    multi: MultiProgress,
    warnings: Mutex<Vec<String>>,
}

impl Pipeline {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            multi: MultiProgress::new(),
            warnings: Mutex::new(Vec::new()),
        }
    }

    pub fn run(&self) -> SrrResult<ProjectOutput> {
        let start = Instant::now();

        // Phase 1: Scan
        let files = self.phase_scan()?;
        if files.is_empty() {
            return Ok(self.empty_output());
        }

        // Apply --include glob filter if configured
        let files = if let Some(ref glob_set) = self.config.include_glob {
            filter_by_include(files, glob_set)
        } else {
            files
        };

        if files.is_empty() {
            return Ok(self.empty_output());
        }

        // Phase 2: Deduplicate
        let (duplicate_groups, near_duplicate_groups) = self.phase_dedup(&files);
        let unique_files: Vec<FileEntry> = self.filter_unique(&files, &duplicate_groups);

        // Phase 3-7: Run independent phases in parallel
        // Patterns, architecture, and compression have no dependencies on each other
        let project_name = self.detect_project_name(&files);
        let primary_language = self.detect_primary_language(&files);
        let total_tokens: usize = files.iter().map(|f| f.token_count).sum();

        let (patterns, architecture, log_summary, doc_summary, _code_summary, scores, clusters) = {
            // Use std::thread::scope for safe parallel execution
            std::thread::scope(|s| {
                let patterns_h = s.spawn(|| self.phase_patterns(&unique_files));
                let arch_h = s.spawn(|| self.phase_architecture(&unique_files));
                let logs_h = s.spawn(|| self.phase_compress_logs(&files));
                let docs_h = s.spawn(|| self.phase_compress_docs(&files));
                let code_h = s.spawn(|| self.phase_compress_code(&unique_files));
                let scores_h = {
                    // scoring depends on architecture, so we wait for it
                    let arch = arch_h.join().unwrap()?;
                    Some(s.spawn(|| {
                        let s = self.phase_scoring(&unique_files, &arch);
                        (arch, s)
                    }))
                };
                let clusters_h = s.spawn(|| self.phase_clustering(&unique_files));

                let patterns = patterns_h.join().unwrap();
                let logs = logs_h.join().unwrap();
                let docs = docs_h.join().unwrap();
                let ccode = code_h.join().unwrap();
                let clusters = clusters_h.join().unwrap();
                let (architecture, scores) = scores_h.unwrap().join().unwrap();

                anyhow::Ok((patterns, architecture, logs, docs, ccode, scores, clusters))
            })
        }?;

        // Build project state (drop content for non-important files)
        let files = self.drop_nonessential_content(files, &scores);

        let mut state = ProjectState {
            project_name,
            primary_language,
            files,
            total_tokens,
            duplicate_groups,
            near_duplicate_groups,
            patterns,
            architecture,
            scores,
            clusters,
            log_summary,
            doc_summary,
            metrics: CompressionMetrics::default(),
        };

        // Compute base metrics from state data (no output text needed)
        let base_metrics = compute_base_metrics(&state, &self.config);

        // Update state.metrics so content generators see real values
        state.metrics = base_metrics.clone();

        // Single pass: generate with pre-computed base metrics
        // (the writer will finalize compressed_tokens from actual output)
        let generated = self.phase_generate(&state, &base_metrics)?;

        let elapsed = start.elapsed();

        if !self.config.quiet && !self.config.json {
            ui::print_results_box(&generated.metrics, elapsed.as_secs_f64());
        }

        Ok(ProjectOutput {
            text: generated.text,
            metrics: generated.metrics,
            warnings: self.warnings.lock().unwrap().clone(),
        })
    }

    fn drop_nonessential_content(&self, mut files: Vec<FileEntry>, scores: &[ScoredFile]) -> Vec<FileEntry> {
        let important: std::collections::HashSet<&std::path::Path> = scores.iter()
            .take(50)
            .map(|s| s.path.as_path())
            .collect();
        for file in &mut files {
            if !important.contains(file.path.as_path()) {
                file.content = None;
            }
        }
        files
    }

    fn phase_scan(&self) -> SrrResult<Vec<FileEntry>> {
        let pb = self.create_progress_bar(1, &ui::format_phase_msg("Scanning filesystem..."), "");
        pb.set_length(1);
        let scanner = DefaultScanner;
        let result = scanner.scan(&self.config.path, &self.config.exclude, self.config.verbose, self.config.respect_gitignore);
        let files = result?;
        pb.finish_with_message(format!("{} {} {}", "✓".green().bold(), "Scanning filesystem".blue(), ui::format_phase_result(&format!("{} files found", files.len()))));
        if self.config.verbose {
            println!("  {} files found", files.len());
        }
        Ok(files)
    }

    fn phase_dedup(&self, files: &[FileEntry]) -> (Vec<DuplicateGroup>, Vec<DuplicateGroup>) {
        let pb = self.create_progress_bar(1, &ui::format_phase_msg("Detecting duplicates..."), "");
        pb.set_length(1);
        let exact = ExactDuplicateDetectorImpl;
        let near = NearDuplicateDetectorImpl;
        let exact_groups = exact.find_exact_duplicates(files);
        let near_groups = near.find_near_duplicates(files);
        pb.finish_with_message(format!("{} {} {} {} {}",
            "✓".green().bold(), "Detecting duplicates".blue(),
            ui::format_phase_result(&format!("{} exact", exact_groups.len())),
            "|", ui::format_phase_result(&format!("{} near", near_groups.len()))));
        if self.config.verbose {
            println!("  {} exact duplicate groups, {} near-duplicate groups found", exact_groups.len(), near_groups.len());
        }
        (exact_groups, near_groups)
    }

    fn filter_unique(&self, files: &[FileEntry], groups: &[DuplicateGroup]) -> Vec<FileEntry> {
        let mut unique = Vec::new();
        let mut removed = std::collections::HashSet::new();
        for group in groups {
            if group.files.len() >= 2 {
                for file in &group.files[1..] {
                    removed.insert(file.clone());
                }
            }
        }
        for file in files {
            if !removed.contains(&file.path) {
                unique.push(file.clone());
            }
        }
        unique
    }

    fn phase_patterns(&self, files: &[FileEntry]) -> Vec<Pattern> {
        let pb = self.create_progress_bar(1, &ui::format_phase_msg("Analyzing code patterns..."), "");
        pb.set_length(1);
        let detector = CrudDetector;
        let patterns = detector.detect_patterns(files);
        pb.finish_with_message(format!("{} {} {}",
            "✓".green().bold(), "Analyzing code patterns".blue(),
            ui::format_phase_result(&format!("{} patterns found", patterns.len()))));
        if self.config.verbose {
            println!("  {} patterns found", patterns.len());
        }
        patterns
    }

    fn phase_architecture(&self, files: &[FileEntry]) -> SrrResult<Architecture> {
        let pb = self.create_progress_bar(1, &ui::format_phase_msg("Mapping architecture..."), "");
        pb.set_length(1);
        let analyzer = ArchitectureDetector;
        let arch = analyzer.analyze(files)?;
        pb.finish_with_message(format!("{} {} {}",
            "✓".green().bold(), "Mapping architecture".blue(),
            ui::format_phase_result(&format!("{} layers", arch.layers.len()))));
        Ok(arch)
    }

    fn phase_scoring(&self, files: &[FileEntry], architecture: &Architecture) -> Vec<ScoredFile> {
        let pb = self.create_progress_bar(1, &ui::format_phase_msg("Scoring files..."), "");
        pb.set_length(1);
        let ranker = ImportanceRanker;
        let scores = ranker.score(files, architecture);
        pb.finish_with_message(format!("{} {} {}",
            "✓".green().bold(), "Scoring files".blue(),
            ui::format_phase_result(&format!("{} files ranked", scores.len()))));
        scores
    }

    fn phase_clustering(&self, files: &[FileEntry]) -> Vec<Cluster> {
        let pb = self.create_progress_bar(1, &ui::format_phase_msg("Clustering by domain..."), "");
        pb.set_length(1);
        let clusterer = DomainClusterer;
        let clusters = clusterer.cluster(files);
        pb.finish_with_message(format!("{} {} {}",
            "✓".green().bold(), "Clustering by domain".blue(),
            ui::format_phase_result(&format!("{} domains", clusters.len()))));
        clusters
    }

    fn warn(&self, msg: String) {
        if let Ok(mut w) = self.warnings.lock() {
            w.push(msg);
        }
    }

    fn phase_compress_logs(&self, files: &[FileEntry]) -> Option<CompressedSection> {
        let compressor = LogCompressor;
        if files.iter().any(|f| compressor.can_handle(f)) {
            let pb = self.create_progress_bar(1, &ui::format_phase_msg("Compressing logs..."), "");
            pb.set_length(1);
            let result = match compressor.compress(files) {
                Ok(s) => Some(s),
                Err(e) => { self.warn(format!("Log compression failed: {}", e)); None }
            };
            pb.finish_with_message(format!("{} {}",
                "✓".green().bold(), "Compressing logs".blue()));
            result
        } else {
            None
        }
    }

    fn phase_compress_docs(&self, files: &[FileEntry]) -> Option<CompressedSection> {
        let compressor = DocCompressor;
        if files.iter().any(|f| compressor.can_handle(f)) {
            let pb = self.create_progress_bar(1, &ui::format_phase_msg("Compressing documentation..."), "");
            pb.set_length(1);
            let result = match compressor.compress(files) {
                Ok(s) => Some(s),
                Err(e) => { self.warn(format!("Doc compression failed: {}", e)); None }
            };
            pb.finish_with_message(format!("{} {}",
                "✓".green().bold(), "Compressing documentation".blue()));
            result
        } else {
            None
        }
    }

    fn phase_compress_code(&self, files: &[FileEntry]) -> Option<CompressedSection> {
        let compressor = CodeCompressor;
        if files.iter().any(|f| compressor.can_handle(f)) {
            match compressor.compress(files) {
                Ok(s) => Some(s),
                Err(e) => { self.warn(format!("Code compression failed: {}", e)); None }
            }
        } else {
            None
        }
    }

    fn phase_generate(&self, state: &ProjectState, base_metrics: &CompressionMetrics) -> SrrResult<GeneratedOutput> {
        let format_label = if self.config.json { "JSON" } else { "Markdown" };
        let pb = self.create_progress_bar(1, &ui::format_phase_msg(&format!("Generating {} context...", format_label)), "");
        pb.set_length(1);

        let output = if self.config.json {
            let writer = crate::context::json::JsonWriter;
            writer.generate(state, &self.config, base_metrics)?
        } else {
            let writer = crate::context::markdown::MarkdownWriter;
            writer.generate(state, &self.config, base_metrics)?
        };

        pb.finish_with_message(format!("{} {}",
            "✓".green().bold(), format!("Generating {} context", format_label).blue()));
        Ok(output)
    }

    fn create_progress_bar(&self, len: u64, msg: &str, _style: &str) -> ProgressBar {
        if self.config.quiet {
            return ProgressBar::hidden();
        }
        let pb = self.multi.add(ProgressBar::new(len));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} {wide_bar:.cyan/blue} {pos}/{len}")
                .unwrap()
                .progress_chars("█▓▒░ "),
        );
        pb.set_message(msg.to_string());
        pb
    }

    fn empty_output(&self) -> ProjectOutput {
        ProjectOutput {
            text: "# Project Summary\n\n**Empty project directory.** No files found to analyze.\n".to_string(),
            metrics: CompressionMetrics::default(),
            warnings: Vec::new(),
        }
    }

    fn detect_project_name(&self, files: &[FileEntry]) -> String {
        for file in files {
            let name = file.path.file_name().unwrap_or_default().to_string_lossy();
            if name == "Cargo.toml" {
                if let Some(ref content) = file.content {
                    if let Ok(val) = content.parse::<Value>() {
                        if let Some(pkg) = val.get("package") {
                            if let Some(pkg_name) = pkg.get("name").and_then(|v| v.as_str()) {
                                return pkg_name.to_string();
                            }
                        }
                    }
                }
            }
            if (name == "package.json" || name == "README.md") && file.content.is_some() {
                return name.to_string();
            }
        }
        self.config.path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }

    fn detect_primary_language(&self, files: &[FileEntry]) -> String {
        let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for file in files {
            match file.extension.as_str() {
                "rs" => *counts.entry("Rust").or_insert(0) += 1,
                "py" => *counts.entry("Python").or_insert(0) += 1,
                "js" | "jsx" => *counts.entry("JavaScript").or_insert(0) += 1,
                "ts" | "tsx" => *counts.entry("TypeScript").or_insert(0) += 1,
                "go" => *counts.entry("Go").or_insert(0) += 1,
                "java" => *counts.entry("Java").or_insert(0) += 1,
                "rb" => *counts.entry("Ruby").or_insert(0) += 1,
                "c" | "h" => *counts.entry("C").or_insert(0) += 1,
                "cpp" | "hpp" | "cc" => *counts.entry("C++").or_insert(0) += 1,
                "cs" => *counts.entry("C#").or_insert(0) += 1,
                "swift" => *counts.entry("Swift").or_insert(0) += 1,
                "kt" | "kts" => *counts.entry("Kotlin").or_insert(0) += 1,
                "scala" => *counts.entry("Scala").or_insert(0) += 1,
                "php" => *counts.entry("PHP").or_insert(0) += 1,
                "dart" => *counts.entry("Dart").or_insert(0) += 1,
                "lua" => *counts.entry("Lua").or_insert(0) += 1,
                "hs" => *counts.entry("Haskell").or_insert(0) += 1,
                "ex" | "exs" => *counts.entry("Elixir").or_insert(0) += 1,
                "clj" | "cljs" => *counts.entry("Clojure").or_insert(0) += 1,
                "erl" => *counts.entry("Erlang").or_insert(0) += 1,
                "pl" | "pm" => *counts.entry("Perl").or_insert(0) += 1,
                "r" => *counts.entry("R").or_insert(0) += 1,
                "zig" => *counts.entry("Zig").or_insert(0) += 1,
                "nim" => *counts.entry("Nim").or_insert(0) += 1,
                "jl" => *counts.entry("Julia").or_insert(0) += 1,
                "m" | "mm" => *counts.entry("Objective-C").or_insert(0) += 1,
                "sh" | "bash" | "zsh" => *counts.entry("Shell").or_insert(0) += 1,
                "ps1" => *counts.entry("PowerShell").or_insert(0) += 1,
                "html" | "htm" => *counts.entry("HTML").or_insert(0) += 1,
                "css" | "scss" | "less" => *counts.entry("CSS").or_insert(0) += 1,
                "sql" => *counts.entry("SQL").or_insert(0) += 1,
                "md" | "markdown" => *counts.entry("Markdown").or_insert(0) += 1,
                "json" => *counts.entry("JSON").or_insert(0) += 1,
                "yaml" | "yml" | "toml" => *counts.entry("Config").or_insert(0) += 1,
                _ => {}
            }
        }
        counts.into_iter()
            .max_by_key(|&(_, c)| c)
            .map(|(lang, _)| lang.to_string())
            .unwrap_or_else(|| "Unknown".to_string())
    }

}

#[derive(Debug)]
pub struct ProjectOutput {
    pub text: String,
    pub metrics: CompressionMetrics,
    pub warnings: Vec<String>,
}
