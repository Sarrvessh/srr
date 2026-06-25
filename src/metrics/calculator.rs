use crate::types::{CompressionMetrics, ProjectState};
use crate::config::Config;
use crate::tokenizer::estimator::count_tokens_fast;

pub fn calculate_metrics(state: &ProjectState, output_text: &str, _config: &Config) -> CompressionMetrics {
    let original_tokens = state.total_tokens;
    let compressed_tokens = count_tokens_fast(output_text);

    let reduction_percent = if original_tokens > 0 && compressed_tokens < original_tokens {
        ((original_tokens - compressed_tokens) as f64 / original_tokens as f64 * 100.0).max(0.0)
    } else {
        0.0
    };

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

    let mut retention = 85.0_f64;
    if has_duplicates { retention += 5.0; }
    if has_logs { retention += 3.0; }
    if has_docs { retention += 3.0; }
    if has_patterns { retention += 2.0; }

    let compression_ratio = if original_tokens > 0 {
        compressed_tokens as f64 / original_tokens as f64
    } else {
        1.0
    };
    if compression_ratio < 0.5 {
        retention = retention.min(98.0);
    }
    let estimated_retention_percent = retention.min(100.0);

    let cost_savings_gpt4o = savings(original_tokens, compressed_tokens, 2.50);
    let cost_savings_claude = savings(original_tokens, compressed_tokens, 3.00);
    let cost_savings_gemini = savings(original_tokens, compressed_tokens, 1.25);

    let original_lines: usize = state.files.iter().map(|f| f.line_count).sum();
    let compressed_lines = output_text.lines().count();

    CompressionMetrics {
        original_tokens,
        compressed_tokens,
        reduction_percent,
        estimated_retention_percent,
        cost_savings_gpt4o,
        cost_savings_claude,
        cost_savings_gemini,
        original_lines,
        compressed_lines,
        original_files: state.files.len(),
        duplicate_files_removed,
    }
}

fn savings(original: usize, compressed: usize, price_per_m: f64) -> f64 {
    if compressed >= original { return 0.0; }
    let saved_tokens = original - compressed;
    saved_tokens as f64 * price_per_m / 1_000_000.0
}
