pub mod markdown;
pub mod json;

use crate::types::{ProjectState, CompressionMetrics};
use crate::config::Config;
use crate::error::SrrResult;

pub trait ContextWriter: Send + Sync {
    fn generate(&self, state: &ProjectState, config: &Config, metrics: &CompressionMetrics) -> SrrResult<GeneratedOutput>;
}

pub fn compute_base_metrics(state: &ProjectState, _config: &Config) -> CompressionMetrics {
    let original_tokens = state.total_tokens;
    let duplicate_files_removed: usize = state.duplicate_groups.iter()
        .map(|g| g.files.len().saturating_sub(1))
        .sum::<usize>()
        + state.near_duplicate_groups.iter()
            .map(|g| g.files.len().saturating_sub(1))
            .sum::<usize>();
    let has_duplicates = !state.duplicate_groups.is_empty() || !state.near_duplicate_groups.is_empty();
    let has_logs = state.log_summary.is_some();
    let has_docs = state.doc_summary.is_some();
    let has_patterns = !state.patterns.is_empty();
    let original_lines: usize = state.files.iter().map(|f| f.line_count).sum();

    let mut retention = 85.0_f64;
    if has_duplicates { retention += 5.0; }
    if has_logs { retention += 3.0; }
    if has_docs { retention += 3.0; }
    if has_patterns { retention += 2.0; }

    CompressionMetrics {
        original_tokens,
        compressed_tokens: 0,
        reduction_percent: 0.0,
        estimated_retention_percent: retention.min(100.0),
        cost_savings_gpt4o: 0.0,
        cost_savings_claude: 0.0,
        cost_savings_gemini: 0.0,
        original_lines,
        compressed_lines: 0,
        original_files: state.files.len(),
        duplicate_files_removed,
    }
}

pub fn finalize_metrics(metrics: &mut CompressionMetrics, analysis: &str) {
    let compressed_tokens = crate::tokenizer::estimator::count_tokens_fast(analysis);
    let compressed_lines = analysis.lines().count();
    metrics.compressed_tokens = compressed_tokens;
    metrics.compressed_lines = compressed_lines;
    if metrics.original_tokens > 0 && compressed_tokens < metrics.original_tokens {
        metrics.reduction_percent = ((metrics.original_tokens - compressed_tokens) as f64 / metrics.original_tokens as f64 * 100.0).max(0.0);
        let compression_ratio = compressed_tokens as f64 / metrics.original_tokens as f64;
        if compression_ratio < 0.5 {
            metrics.estimated_retention_percent = metrics.estimated_retention_percent.min(98.0);
        }
    }
    let saved_tokens = metrics.original_tokens.saturating_sub(compressed_tokens);
    metrics.cost_savings_gpt4o = saved_tokens as f64 * 2.50 / 1_000_000.0;
    metrics.cost_savings_claude = saved_tokens as f64 * 3.00 / 1_000_000.0;
    metrics.cost_savings_gemini = saved_tokens as f64 * 1.25 / 1_000_000.0;
}

pub struct GeneratedOutput {
    pub text: String,
    pub analysis: String,
    pub metrics: CompressionMetrics,
}
