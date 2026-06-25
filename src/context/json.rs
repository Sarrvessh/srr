use serde::Serialize;

use crate::error::SrrResult;
use crate::types::{ProjectState, CompressionMetrics};
use crate::config::Config;
use super::{ContextWriter, GeneratedOutput};

pub struct JsonWriter;

impl ContextWriter for JsonWriter {
    fn generate(&self, state: &ProjectState, _config: &Config, metrics: &CompressionMetrics) -> SrrResult<GeneratedOutput> {
        let output = JsonOutput {
            project_name: state.project_name.clone(),
            primary_language: state.primary_language.clone(),
            project_summary: JsonProjectSummary {
                total_files: state.files.len(),
                original_lines: metrics.original_lines,
                original_tokens: state.total_tokens,
                total_tokens: state.total_tokens,
            },
            architecture: JsonArchitecture {
                layers: state.architecture.layers.iter().map(|l| JsonLayer {
                    name: l.name.clone(),
                    file_count: l.file_count,
                    technologies: l.technologies.clone(),
                }).collect(),
                hierarchy: state.architecture.hierarchy_text.clone(),
            },
            clusters: state.clusters.iter().map(|c| JsonCluster {
                name: c.name.clone(),
                description: c.description.clone(),
                files: c.files.iter().map(|f| f.to_string_lossy().to_string()).collect(),
            }).collect(),
            important_files: state.scores.iter().take(20).map(|s| JsonScoredFile {
                path: s.path.to_string_lossy().to_string(),
                score: s.score,
                token_count: s.token_count,
            }).collect(),
            duplicate_groups: state.duplicate_groups.iter().map(|g| JsonDupGroup {
                reason: g.reason.clone(),
                files: g.files.iter().map(|f| f.to_string_lossy().to_string()).collect(),
            }).collect(),
            near_duplicate_groups: state.near_duplicate_groups.iter().map(|g| JsonDupGroup {
                reason: g.reason.clone(),
                files: g.files.iter().map(|f| f.to_string_lossy().to_string()).collect(),
            }).collect(),
            patterns: state.patterns.iter().map(|p| JsonPattern {
                pattern_type: p.pattern_type.clone(),
                entity: p.entity.clone(),
                operations: p.operations.clone(),
                files: p.files.iter().map(|f| f.to_string_lossy().to_string()).collect(),
            }).collect(),
            log_summary: state.log_summary.as_ref().map(|l| l.content.clone()),
            doc_summary: state.doc_summary.as_ref().map(|d| d.content.clone()),
            compression_metrics: JsonMetrics {
                original_tokens: metrics.original_tokens,
                compressed_tokens: metrics.compressed_tokens,
                reduction_percent: metrics.reduction_percent,
                estimated_retention_percent: metrics.estimated_retention_percent,
                cost_savings_gpt4o: metrics.cost_savings_gpt4o,
                cost_savings_claude: metrics.cost_savings_claude,
                cost_savings_gemini: metrics.cost_savings_gemini,
                original_lines: metrics.original_lines,
                compressed_lines: metrics.compressed_lines,
                original_files: metrics.original_files,
                duplicate_files_removed: metrics.duplicate_files_removed,
            },
        };

        let text = serde_json::to_string_pretty(&output)
            .map_err(|e| crate::error::SrrError::Anyhow(e.into()))?;

        Ok(GeneratedOutput { analysis: text.clone(), text, metrics: metrics.clone() })
    }
}

#[derive(Serialize)]
struct JsonOutput {
    project_name: String,
    primary_language: String,
    project_summary: JsonProjectSummary,
    architecture: JsonArchitecture,
    clusters: Vec<JsonCluster>,
    important_files: Vec<JsonScoredFile>,
    duplicate_groups: Vec<JsonDupGroup>,
    near_duplicate_groups: Vec<JsonDupGroup>,
    patterns: Vec<JsonPattern>,
    log_summary: Option<String>,
    doc_summary: Option<String>,
    compression_metrics: JsonMetrics,
}

#[derive(Serialize)]
struct JsonProjectSummary {
    total_files: usize,
    original_lines: usize,
    original_tokens: usize,
    total_tokens: usize,
}

#[derive(Serialize)]
struct JsonArchitecture {
    layers: Vec<JsonLayer>,
    hierarchy: String,
}

#[derive(Serialize)]
struct JsonLayer {
    name: String,
    file_count: usize,
    technologies: Vec<String>,
}

#[derive(Serialize)]
struct JsonCluster {
    name: String,
    description: String,
    files: Vec<String>,
}

#[derive(Serialize)]
struct JsonScoredFile {
    path: String,
    score: f64,
    token_count: usize,
}

#[derive(Serialize)]
struct JsonDupGroup {
    reason: String,
    files: Vec<String>,
}

#[derive(Serialize)]
struct JsonPattern {
    pattern_type: String,
    entity: String,
    operations: Vec<String>,
    files: Vec<String>,
}

#[derive(Serialize)]
struct JsonMetrics {
    original_tokens: usize,
    compressed_tokens: usize,
    reduction_percent: f64,
    estimated_retention_percent: f64,
    cost_savings_gpt4o: f64,
    cost_savings_claude: f64,
    cost_savings_gemini: f64,
    original_lines: usize,
    compressed_lines: usize,
    original_files: usize,
    duplicate_files_removed: usize,
}
