use colored::Colorize;

use crate::types::CompressionMetrics;

const CHECK: &str = "✓";
const CROSS: &str = "✗";

pub fn print_banner() {
    let border = |s: &str| s.cyan().bold();
    let letter = |s: &str| s.bright_cyan().bold();
    let ver = |s: &str| s.yellow();
    let tag = |s: &str| s.white();

    let art = [
        "                ███████   ███████   ███████                 ",
        "                ██        ██   ██   ██   ██                 ",
        "                ██        ██   ██   ██   ██                 ",
        "                ███████   ███████   ███████                 ",
        "                     ██   ██ ██     ██ ██                   ",
        "                ███████   ██   ██   ██   ██                 ",
    ];

    println!();
    println!("{}", border("╔══════════════════════════════════════════════════════════════╗"));
    println!("{}", border("║                                                              ║"));
    for line in &art {
        println!("║{}║", letter(line));
    }
    println!("{}", border("║                                                              ║"));

    let version_line = format!("              SRR Context Compressor  v{}", env!("CARGO_PKG_VERSION"));
    let padded_ver = format!("{:<60}", version_line);
    println!("{}", border(&format!("║{}{}║", ver(&padded_ver), "")));

    let tagline = "           Reduce LLM context \u{00B7} Keep the signal               ";
    println!("{}", border(&format!("║{}{}║", tag(tagline), "")));

    println!("{}", border("║                                                              ║"));
    println!("{}", border("╚══════════════════════════════════════════════════════════════╝"));
    println!();
}

pub fn print_results_box(metrics: &CompressionMetrics, elapsed_secs: f64) {
    let border = |s: &str| s.cyan().bold();
    let label = |s: &str| s.white();
    let value = |s: &str| s.yellow();
    let positive = |s: &str| s.green().bold();
    let header = |s: &str| s.cyan().bold();
    let dim = |s: &str| s.bright_black();
    let warn = |s: &str| s.yellow().bold();

    let is_inflated = metrics.compressed_tokens > metrics.original_tokens;
    let reduction = metrics.reduction_percent;
    let reduction_str = if is_inflated {
        warn("inflated  ")
    } else if reduction > 0.0 {
        positive(&format_reduction(reduction))
    } else {
        dim(&format_reduction(reduction))
    };

    let dup_pct = if metrics.original_files > 0 {
        metrics.duplicate_files_removed as f64 / metrics.original_files as f64 * 100.0
    } else {
        0.0
    };

    let elapsed = format!("{:.2}s", elapsed_secs);

    println!();
    println!("{}", border("╭──────────────────────────────────────────────────╮"));
    println!("{} {:43} {}", border("│"), header("Results"), border("│"));
    println!("{}", border("├──────────────────────────────────────────────────┤"));
    println!("{} {:29} {:>12} {}", border("│"), label("  Original files"),       value(&format_number(metrics.original_files)),              border("│"));
    let dup_display = format!("{:.1}%", dup_pct);
    println!("{} {:29} {:>4} ({:>5} {}) {}", border("│"), label("  Duplicates removed"), value(&format_number(metrics.duplicate_files_removed)), dim(&dup_display), dim(""), border("│"));
    println!("{} {:29} {:>12} {}", border("│"), label("  Original tokens"),       value(&format!("{} tokens", format_number(metrics.original_tokens))),  border("│"));
    println!("{} {:29} {:>12} {}", border("│"), label("  Compressed tokens"),     value(&format!("{} tokens", format_number(metrics.compressed_tokens))),border("│"));
    println!("{} {:29} {:>12} {}", border("│"), label("  Reduction"),             reduction_str,                                                   border("│"));
    let retention_str = if is_inflated {
        format!("{:.1}%*", metrics.estimated_retention_percent)
    } else {
        format!("{:.1}%", metrics.estimated_retention_percent)
    };
    println!("{} {:29} {:>12} {}", border("│"), label("  Retention"),             value(&retention_str), border("│"));
    println!("{}", border("├──────────────────────────────────────────────────┤"));
    if is_inflated {
        let msg1 = "  Small project — analysis overhead exceeds";
        let msg2 = "  raw content. Benefits scale with size.  ";
        println!("{} {} {}", border("│"), warn(msg1), border("│"));
        println!("{} {} {}", border("│"), warn(msg2), border("│"));
        println!("{}", border("├──────────────────────────────────────────────────┤"));
    }
    println!("{} {:43} {}", border("│"), header("Cost Savings"), border("│"));
    println!("{} {:29} {:>12} {}", border("│"), label("  GPT-4o"),               value(&format!("${:.4}", metrics.cost_savings_gpt4o)),              border("│"));
    println!("{} {:29} {:>12} {}", border("│"), label("  Claude 3.5 Sonnet"),    value(&format!("${:.4}", metrics.cost_savings_claude)),              border("│"));
    println!("{} {:29} {:>12} {}", border("│"), label("  Gemini 1.5 Pro"),       value(&format!("${:.4}", metrics.cost_savings_gemini)),              border("│"));
    println!("{}", border("├──────────────────────────────────────────────────┤"));
    println!("{} {:29} {:>12} {}", border("│"), label("  Completed in"),         border(&elapsed), border("│"));
    println!("{}", border("╰──────────────────────────────────────────────────╯"));
    println!();
}

pub fn print_error_box(msg: &str, causes: &[String]) {
    let border = |s: &str| s.red().bold();
    let msg_s = |s: &str| s.bright_red().bold();
    let cause_s = |s: &str| s.red();
    let tip = |s: &str| s.yellow();

    println!();
    println!("{}", border("╭──────────────────────────────────────────────────╮"));
    println!("{}", border(&format!("│  {}  {:<42} │", CROSS, "")));
    println!("{} {:48} {}", border("│"), msg_s(msg), border("│"));

    if !causes.is_empty() {
        for c in causes {
            println!("{} {:48} {}", border("│"), cause_s(c), border("│"));
        }
    }

    println!("{}", border(&format!("│  {}  {:<42} │", "", "")));
    println!("{} {:48} {}", border("│"), tip("Tip: Run with --help for usage information"), border("│"));
    println!("{}", border(&format!("│  {}  {:<42} │", "", "")));
    println!("{}", border("╰──────────────────────────────────────────────────╯"));
    println!();
}

pub fn format_number(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let len = s.len();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (len - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(c);
    }
    result
}

fn format_reduction(pct: f64) -> String {
    if pct > 0.0 {
        format!("{:.1}%", pct)
    } else {
        "—".to_string()
    }
}

pub fn format_phase_msg(phase: &str) -> String {
    format!("{} {}", "▶".cyan(), phase.cyan())
}

pub fn format_phase_done(msg: &str) -> String {
    format!("{} {}", CHECK.green().bold(), msg.blue())
}

pub fn format_phase_result(counts: &str) -> String {
    counts.blue().to_string()
}

// ── Agent mode UI helpers ──

pub mod symbols {
    pub const AGENT: &str = "◆";
    pub const TOOL: &str = "⚡";
    pub const USER: &str = "▼";
    pub const RESPONSE: &str = "▶";
    pub const ERROR: &str = "✗";
    pub const OK: &str = "✓";
    pub const INFO: &str = "ℹ";
}

pub fn style_agent_msg(msg: &str) -> String {
    format!("{} {}", symbols::AGENT.cyan().bold(), msg.cyan())
}

pub fn style_agent_response(msg: &str) -> String {
    format!("{} {}", symbols::RESPONSE.green().bold(), msg.green())
}

pub fn style_user_input(msg: &str) -> String {
    format!("{} {}", symbols::USER.blue().bold(), msg.bright_blue())
}

pub fn print_agent_banner() {
    println!();
    println!("{}", "╭──────────────────────────────────────────────╮".cyan().bold());
    println!("{} {:44} {}", "│".cyan().bold(), "SRR Agent v0.1".cyan().bold(), "│".cyan().bold());
    println!("{}", "├──────────────────────────────────────────────┤".cyan().bold());
    println!("{} {:44} {}", "│".cyan().bold(), "Type 'exit' to quit".white(), "│".cyan().bold());
    println!("{}", "╰──────────────────────────────────────────────╯".cyan().bold());
    println!();
}

pub fn print_index_banner(path: &str) {
    println!();
    println!("{}", "╭──────────────────────────────────────────────╮".cyan().bold());
    println!("{} {:44} {}", "│".cyan().bold(), "SRR Index v0.1".cyan().bold(), "│".cyan().bold());
    println!("{}", "├──────────────────────────────────────────────┤".cyan().bold());
    println!("{} {:44} {}", "│".cyan().bold(), format!("Indexing: {path}").white(), "│".cyan().bold());
    println!("{}", "╰──────────────────────────────────────────────╯".cyan().bold());
    println!();
}

pub fn print_task_plan(plan: &crate::types::TaskPlan) {
    println!();
    println!("{}", "╭──────────────────────────────────────────────╮".cyan().bold());
    println!("{} {:44} {}", "│".cyan().bold(), "Task Plan".cyan().bold(), "│".cyan().bold());
    println!("{}", "├──────────────────────────────────────────────┤".cyan().bold());
    println!("{} {:44} {}", "│".cyan().bold(), format!("Intent: {:?}", plan.intent.intent_type).white(), "│".cyan().bold());
    println!("{} {:44} {}", "│".cyan().bold(), format!("Confidence: {:.0}%", plan.confidence * 100.0).white(), "│".cyan().bold());
    println!("{} {:44} {}", "│".cyan().bold(), format!("Files: {}", plan.relevant_files.len()).white(), "│".cyan().bold());
    println!("{}", "├──────────────────────────────────────────────┤".cyan().bold());
    for step in &plan.steps {
        println!("{} {:44} {}", "│".cyan().bold(), format!("{}. {}", step.order, step.description).yellow(), "│".cyan().bold());
    }
    println!("{}", "╰──────────────────────────────────────────────╯".cyan().bold());
    println!();
}

pub fn print_search_results(query: &str, symbols: &[crate::types::Symbol]) {
    println!();
    println!("{}", "╭──────────────────────────────────────────────╮".cyan().bold());
    println!("{} {:44} {}", "│".cyan().bold(), format!("Search: {query}").cyan().bold(), "│".cyan().bold());
    println!("{}", "├──────────────────────────────────────────────┤".cyan().bold());
    if symbols.is_empty() {
        println!("{} {:44} {}", "│".cyan().bold(), "No results found".white(), "│".cyan().bold());
    } else {
        for sym in symbols.iter().take(20) {
            let loc = format!("{}:{}", sym.file_path.display(), sym.line);
            println!("{} {:44} {}", "│".cyan().bold(), format!(" {} {}", sym.name, loc).white(), "│".cyan().bold());
        }
        if symbols.len() > 20 {
            println!("{} {:44} {}", "│".cyan().bold(), format!("... and {} more", symbols.len() - 20).dimmed(), "│".cyan().bold());
        }
    }
    println!("{}", "╰──────────────────────────────────────────────╯".cyan().bold());
    println!();
}
