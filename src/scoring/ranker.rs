use std::collections::HashMap;
use std::sync::LazyLock;
use regex::Regex;
use rayon::prelude::*;

use crate::types::{FileEntry, ScoredFile, Architecture};
use super::FileScorer;

static IMPORT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*(?:use|import|from|require|include)\s+[\w:{}]+").unwrap()
});

pub struct ImportanceRanker;

impl FileScorer for ImportanceRanker {
    fn score(&self, files: &[FileEntry], architecture: &Architecture) -> Vec<ScoredFile> {
        let import_counts: HashMap<&std::path::Path, usize> = files
            .par_iter()
            .filter_map(|file| {
                file.content.as_ref().map(|content| {
                    let count = IMPORT_RE.find_iter(content).count();
                    (file.relative_path.as_path(), count)
                })
            })
            .collect();

        let file_refs = count_references(files);

        let raw_scores: Vec<(f64, &FileEntry)> = files
            .par_iter()
            .map(|file| {
                let mut score = 0.0;

                let fname = file.path.file_name()
                    .map(|n| n.to_string_lossy().to_lowercase())
                    .unwrap_or_default();

                if fname == "main.rs" || fname == "main.py" || fname == "index.js"
                    || fname == "index.ts" || fname == "app.ts" || fname == "app.py"
                    || fname == "app.js" || fname == "main.go" || fname == "main.java" {
                    score += 30.0;
                }

                if fname == "lib.rs" || fname == "mod.rs" || fname == "__init__.py" {
                    score += 20.0;
                }

                let import_count = import_counts.get(file.relative_path.as_path()).copied().unwrap_or(0);
                score += (import_count as f64).min(25.0);

                let ref_count = file_refs.get(file.relative_path.as_path()).copied().unwrap_or(0);
                score += (ref_count as f64).min(25.0);

                if fname.contains("config") || fname.contains("setting")
                    || fname == ".env" || fname.ends_with(".toml")
                    || fname.ends_with(".yaml") || fname.ends_with(".json") {
                    score += 20.0;
                }

                let depth = file.relative_path.components().count() as f64;
                score += (10.0 - depth * 1.5).max(0.0);

                let size_tokens = file.token_count as f64;
                if size_tokens > 50.0 && size_tokens < 5000.0 {
                    score += 10.0;
                } else if size_tokens <= 50.0 {
                    score += 3.0;
                }

                for layer in &architecture.layers {
                    let path_lower = file.path.to_string_lossy().to_lowercase();
                    let layer_lower = layer.name.to_lowercase();
                    let matches_component = path_lower
                        .split(&['/', '\\'][..])
                        .any(|comp| comp.contains(&layer_lower));
                    if matches_component {
                        score += 5.0;
                    }
                }

                (score, file)
            })
            .collect();

        if raw_scores.is_empty() {
            return Vec::new();
        }

        let max_score = raw_scores.iter()
            .map(|(s, _)| *s)
            .fold(0.0_f64, |a, b| a.max(b));

        let min_score = raw_scores.iter()
            .map(|(s, _)| *s)
            .fold(0.0_f64, |a, b| a.min(b));

        let range = (max_score - min_score).max(1.0);

        let mut scored: Vec<ScoredFile> = raw_scores
            .into_iter()
            .map(|(raw, file)| {
                let normalized = ((raw - min_score) / range * 100.0).round().clamp(0.0, 100.0);
                ScoredFile {
                    path: file.path.clone(),
                    score: normalized,
                    token_count: file.token_count,
                }
            })
            .collect();

        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        scored
    }
}

fn count_references(files: &[FileEntry]) -> HashMap<&std::path::Path, usize> {
    // Build index: stem → files with that stem
    let mut stem_to_files: HashMap<String, Vec<&std::path::Path>> = HashMap::new();
    for file in files {
        if file.content.is_some() {
            if let Some(stem) = file.path.file_stem().map(|s| s.to_string_lossy().to_lowercase()) {
                if stem.len() >= 3 {
                    stem_to_files.entry(stem).or_default().push(&file.path);
                }
            }
        }
    }

    let mut refs: HashMap<&std::path::Path, usize> = HashMap::new();

    // For each file, scan content for words that match stems
    for file in files {
        if let Some(ref content) = file.content {
            let content_lower = content.to_lowercase();
            let words: std::collections::HashSet<String> = content_lower
                .split(|c: char| !c.is_alphanumeric())
                .filter(|w| w.len() >= 3)
                .map(|w| w.to_string())
                .collect();

            for word in &words {
                if let Some(matched_files) = stem_to_files.get(word) {
                    for matched_path in matched_files {
                        if *matched_path != file.path {
                            *refs.entry(matched_path).or_insert(0) += 1;
                        }
                    }
                }
            }
        }
    }

    refs
}
