use crate::error::SrrResult;
use crate::types::{ProjectState, SummaryLevel, ModelType, CompressionMetrics};
use crate::config::Config;
use crate::tokenizer::pricing::get_pricing;
use crate::tokenizer::estimator::count_tokens_fast;
use super::{ContextWriter, GeneratedOutput, finalize_metrics};

pub struct MarkdownWriter;

impl ContextWriter for MarkdownWriter {
    fn generate(&self, state: &ProjectState, config: &Config, base_metrics: &CompressionMetrics) -> SrrResult<GeneratedOutput> {
        let budget = config.max_tokens.unwrap_or(usize::MAX);

        if budget > 0 && budget < 100 {
            let note = format!(
                "# Project Summary\n\n**Token budget too small ({}).** Increase `--max-tokens` for a richer summary.\n\n",
                budget
            );
            let dummy = base_metrics.clone();
            let metrics_text = render_metrics_table(&dummy);
            let analysis = format!("{}{}", note, metrics_text);
            return Ok(GeneratedOutput { text: analysis.clone(), analysis, metrics: base_metrics.clone() });
        }

        let mut sections = String::new();
        let mut tokens_used = 0usize;

        let mut try_add = |content: &str| -> bool {
            if content.is_empty() { return true; }
            let t = count_tokens_fast(content);
            if tokens_used + t > budget {
                return false;
            }
            sections.push_str(content);
            sections.push('\n');
            tokens_used += t;
            true
        };

        try_add(&generate_project_summary(state, config));
        try_add(&generate_architecture(state));
        try_add(&generate_key_components(state));
        try_add(&generate_important_files(state, config));
        try_add(&generate_rankings(state, config));
        try_add(&generate_clusters(state));
        try_add(&generate_patterns(state));

        if let Some(ref doc) = state.doc_summary {
            try_add(&doc.content);
        }
        if let Some(ref log) = state.log_summary {
            try_add(&log.content);
        }

        // Finalize metrics using actual section text
        let mut metrics = base_metrics.clone();
        finalize_metrics(&mut metrics, &sections);
        sections.push_str(&render_metrics_table(&metrics));
        sections.push('\n');

        // Remaining budget for recommended context
        let remaining_budget = budget.saturating_sub(count_tokens_fast(&sections));
        let context = generate_recommended_context(state, config, remaining_budget);
        let text = format!("{}{}", sections, context);

        Ok(GeneratedOutput { text, analysis: sections, metrics })
    }
}

fn render_metrics_table(metrics: &CompressionMetrics) -> String {
    let mut s = String::new();
    s.push_str("# Compression Metrics\n\n");
    s.push_str("| Metric | Value |\n");
    s.push_str("|--------|-------|\n");
    s.push_str(&format!("| Original Tokens | {} |\n", metrics.original_tokens));
    s.push_str(&format!("| Compressed Tokens | {} |\n", metrics.compressed_tokens));

    if metrics.reduction_percent > 0.0 {
        s.push_str(&format!("| Reduction | {:.1}% |\n", metrics.reduction_percent));
    } else {
        s.push_str("| Reduction | — (analysis overhead exceeds original; benefits scale with size) |\n");
    }

    s.push_str(&format!("| Estimated Retention | {:.1}% |\n", metrics.estimated_retention_percent));
    s.push_str(&format!("| Original Files | {} |\n", metrics.original_files));
    s.push_str(&format!("| Duplicates Removed | {} |\n", metrics.duplicate_files_removed));
    s.push_str(&format!("| Original Lines | {} |\n", metrics.original_lines));
    s.push_str(&format!("| Compressed Lines | {} |\n", metrics.compressed_lines));
    s.push('\n');

    if metrics.cost_savings_gpt4o > 0.0 || metrics.cost_savings_claude > 0.0 || metrics.cost_savings_gemini > 0.0 {
        s.push_str("### Cost Savings\n\n");
        s.push_str("| Model | Savings |\n");
        s.push_str("|-------|--------|\n");
        s.push_str(&format!("| GPT-4o | ${:.4} |\n", metrics.cost_savings_gpt4o));
        s.push_str(&format!("| Claude 3.5 Sonnet | ${:.4} |\n", metrics.cost_savings_claude));
        s.push_str(&format!("| Gemini 1.5 Pro | ${:.4} |\n", metrics.cost_savings_gemini));
        s.push('\n');
    }
    s
}

fn generate_project_summary(state: &ProjectState, _config: &Config) -> String {
    let mut s = String::new();
    s.push_str("# Project Summary\n\n");
    s.push_str(&format!("- **Project**: {}\n", state.project_name));
    s.push_str(&format!("- **Primary Language**: {}\n", state.primary_language));
    s.push_str(&format!("- **Total Files**: {}\n", state.files.len()));
    s.push_str(&format!("- **Total Lines**: {}\n", state.metrics.original_lines));
    s.push_str(&format!("- **Original Tokens**: {}\n", state.total_tokens));
    s.push_str("- **Estimated Cost**:\n");

    let models = [
        (ModelType::Gpt4o, "GPT-4o"),
        (ModelType::Claude35Sonnet, "Claude 3.5"),
        (ModelType::Gemini15Pro, "Gemini 1.5"),
    ];
    for (model, label) in &models {
        let pricing = get_pricing(model);
        let rate = pricing.first().map(|p| p.input_price_per_1m).unwrap_or(2.50);
        s.push_str(&format!("  - {}: ${:.4}\n", label, state.total_tokens as f64 * rate / 1_000_000.0));
    }

    s.push('\n');
    s
}

fn generate_architecture(state: &ProjectState) -> String {
    let mut s = String::new();
    s.push_str("# Architecture Overview\n\n");

    if state.architecture.layers.is_empty() {
        s.push_str("No architecture detected.\n\n");
        return s;
    }

    s.push_str("```\n");
    s.push_str(&state.architecture.hierarchy_text);
    s.push_str("\n```\n\n");

    s.push_str("| Layer | Files | Technologies |\n");
    s.push_str("|-------|-------|-------------|\n");
    for layer in &state.architecture.layers {
        s.push_str(&format!(
            "| {} | {} | {} |\n",
            layer.name,
            layer.file_count,
            layer.technologies.join(", ")
        ));
    }
    s.push('\n');
    s
}

fn generate_key_components(state: &ProjectState) -> String {
    let mut s = String::new();
    s.push_str("# Key Components\n\n");
    for cluster in &state.clusters {
        s.push_str(&format!(
            "- **{}** ({} files) — {}\n",
            cluster.name,
            cluster.files.len(),
            cluster.description
        ));
    }
    s.push('\n');
    s
}

fn generate_important_files(state: &ProjectState, config: &Config) -> String {
    let mut s = String::new();
    s.push_str("# Important Files\n\n");
    s.push_str("| Score | File | Tokens |\n");
    s.push_str("|-------|------|--------|\n");

    let top_n = match config.summary_level {
        SummaryLevel::Compact => 5,
        SummaryLevel::Detailed => 20,
    };

    for scored in state.scores.iter().take(top_n) {
        let rel_path = scored.path
            .strip_prefix(&config.path)
            .unwrap_or(&scored.path)
            .display();
        s.push_str(&format!(
            "| {} | `{}` | {} |\n",
            scored.score as u64,
            rel_path,
            scored.token_count
        ));
    }
    s.push('\n');
    s
}

fn generate_clusters(state: &ProjectState) -> String {
    let mut s = String::new();
    s.push_str("# Semantic Clusters\n\n");

    if state.clusters.is_empty() {
        s.push_str("No clusters identified.\n\n");
        return s;
    }

    for cluster in &state.clusters {
        s.push_str(&format!("## {}\n\n", cluster.name));
        s.push_str(&format!("{} files — {}\n\n", cluster.files.len(), cluster.description));

        for file in &cluster.files {
            s.push_str(&format!("- `{}`\n", file.display()));
        }
        s.push('\n');
    }
    s
}

fn generate_patterns(state: &ProjectState) -> String {
    let mut s = String::new();
    s.push_str("# Repetitive Patterns\n\n");

    if state.patterns.is_empty() {
        s.push_str("No repetitive patterns detected.\n\n");
        return s;
    }

    for pattern in &state.patterns {
        s.push_str(&format!("## {} Pattern: {}\n\n", pattern.pattern_type, pattern.entity));
        s.push_str("Operations:\n");
        for op in &pattern.operations {
            s.push_str(&format!("- {}\n", op));
        }
        s.push_str("\nFiles:\n");
        for file in &pattern.files {
            s.push_str(&format!("- `{}`\n", file.display()));
        }
        s.push('\n');
    }
    s
}

fn generate_rankings(state: &ProjectState, config: &Config) -> String {
    let mut s = String::new();
    s.push_str("# File Importance Rankings\n\n");
    s.push_str("| Rank | Score | File | Tokens |\n");
    s.push_str("|------|-------|------|--------|\n");

    let top_n = match config.summary_level {
        SummaryLevel::Compact => 10,
        SummaryLevel::Detailed => state.scores.len().min(50),
    };

    for (i, scored) in state.scores.iter().take(top_n).enumerate() {
        let rel_path = scored.path
            .strip_prefix(&config.path)
            .unwrap_or(&scored.path)
            .display();
        s.push_str(&format!(
            "| {} | {} | `{}` | {} |\n",
            i + 1,
            scored.score as u64,
            rel_path,
            scored.token_count
        ));
    }
    s.push('\n');
    s
}

fn generate_recommended_context(state: &ProjectState, config: &Config, remaining_budget: usize) -> String {
    let mut s = String::new();
    s.push_str("# Recommended Context\n\n");

    let top_n = match config.summary_level {
        SummaryLevel::Compact => 0,
        SummaryLevel::Detailed => 5,
    };

    if top_n == 0 {
        s.push_str("Run with `--summary-level detailed` to include file contents in context.\n\n");
        return s;
    }

    let mut used = 0usize;

    for file in state.files.iter() {
        if used >= remaining_budget.saturating_sub(std::cmp::max(100, remaining_budget / 10)) {
            break;
        }
        if file.is_binary || file.content.is_none() {
            continue;
        }

        let is_important = state.scores.iter()
            .take(top_n)
            .any(|s| s.path == file.path);

        if !is_important && state.scores.iter().position(|s| s.path == file.path)
            .is_some_and(|p| p >= top_n) {
            continue;
        }

        let content = file.content.as_ref().unwrap();
        let ext = &file.extension;
        let rel_path = file.path
            .strip_prefix(&config.path)
            .unwrap_or(&file.path)
            .display();

        let snippet = format!(
            "### `{}` ({} tokens)\n```{}\n{}\n```\n\n",
            rel_path,
            file.token_count,
            ext,
            content
        );

        let snippet_tokens = count_tokens_fast(&snippet);
        let margin = std::cmp::max(100, remaining_budget / 10);
        if used + snippet_tokens > remaining_budget.saturating_sub(margin) {
            continue;
        }

        s.push_str(&snippet);
        used += snippet_tokens;
    }

    if s.lines().count() <= 3 {
        s.push_str("No files selected for recommended context.\n\n");
    }

    s
}
