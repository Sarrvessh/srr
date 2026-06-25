use clap::Parser;
use clap::CommandFactory;
use clap_complete::Shell;
use colored::Colorize;
use srr::cli::Cli;
use srr::config::Config;
use srr::pipeline::Pipeline;
use srr::ui;

fn main() {
    let raw: Vec<String> = std::env::args().collect();

    // Check for --completions in raw args regardless of mode
    if let Some(pos) = raw.iter().position(|a| a == "--completions") {
        if let Some(shell_str) = raw.get(pos + 1) {
            let shell = match shell_str.to_lowercase().as_str() {
                "bash" => Shell::Bash,
                "zsh" => Shell::Zsh,
                "powershell" | "ps1" => Shell::PowerShell,
                "fish" => Shell::Fish,
                _ => {
                    eprintln!("Unknown shell '{}'. Supported: bash, zsh, powershell, fish", shell_str);
                    std::process::exit(1);
                }
            };
            let mut cmd = Cli::command();
            let name = cmd.get_name().to_string();
            clap_complete::generate(shell, &mut cmd, &name, &mut std::io::stdout());
            return;
        }
    }

    // Detect subcommand from raw args (before clap to avoid path parse conflict)
    let subcommand = raw.iter().skip(1).find(|a| !a.starts_with('-')).map(|s| s.as_str());

    match subcommand {
        Some("index") => run_index(&raw),
        Some("task") => run_task(&raw),
        Some("plan") => run_plan(&raw),
        Some("agent") => run_agent(&raw),
        Some("tui") => run_tui(&raw),
        Some("search") => run_search(&raw),
        Some("compress") => {
            // 'compress' subcommand - strip it then parse normally
            let filtered: Vec<String> = raw.iter().filter(|a| *a != "compress").cloned().collect();
            run_compress(&filtered);
        }
        Some(_) | None => run_compress(&raw),
    }
}

fn run_compress(raw: &[String]) {
    let cli = Cli::parse_from(raw);
    let config = Config::from_cli(&cli);

    if config.no_color {
        colored::control::set_override(false);
    }
    if let Some(n) = config.concurrency {
        let _ = rayon::ThreadPoolBuilder::new()
            .num_threads(n)
            .build_global();
    }

    for w in &config.warnings {
        eprintln!("  {} {}", "⚠".yellow().bold(), w.yellow());
    }

    if !config.quiet && !cli.no_banner {
        ui::print_banner();
    }

    let pipeline = Pipeline::new(config);

    match pipeline.run() {
        Ok(output) => {
            if pipeline.config.dry_run {
                return;
            }

            if let Some(output_path) = &pipeline.config.output {
                let write_result = if pipeline.config.gzip {
                    let ext = std::path::Path::new(output_path).extension()
                        .map(|e| e.to_string_lossy().to_lowercase())
                        .filter(|e| e == "gz")
                        .is_some();
                    let gz_path = if ext {
                        output_path.clone()
                    } else {
                        let mut p = output_path.to_path_buf();
                        p.set_extension(format!("{}.gz", p.extension().unwrap_or_default().to_string_lossy()));
                        p
                    };
                    use std::io::Write;
                    let file = std::fs::File::create(&gz_path);
                    match file {
                        Ok(f) => {
                            let mut encoder = flate2::write::GzEncoder::new(f, flate2::Compression::default());
                            encoder.write_all(output.text.as_bytes()).and_then(|_| encoder.finish().map(|_| ()))
                                .map(|_| gz_path)
                        }
                        Err(e) => Err(e),
                    }
                } else {
                    std::fs::write(output_path, &output.text).map(|_| output_path.clone())
                };

                match write_result {
                    Ok(path) => {
                        if !pipeline.config.quiet {
                            println!("  {} Output written to {}", "✓".green().bold(), path.display().to_string().cyan());
                        }
                    }
                    Err(e) => {
                        ui::print_error_box(&format!("Failed to write output: {}", e), &[]);
                        std::process::exit(1);
                    }
                }
            } else if !pipeline.config.quiet {
                println!("{}", output.text);
            }

            if !output.warnings.is_empty() {
                eprintln!();
                eprintln!("  {} Warnings:", "◆".yellow().bold());
                for w in &output.warnings {
                    eprintln!("    {} {}", "·".yellow(), w);
                }
            }
        }
        Err(e) => {
            let mut causes: Vec<String> = Vec::new();
            let mut current: Option<&dyn std::error::Error> = Some(&e);
            while let Some(err) = current {
                let s = format!("{}", err);
                if !causes.iter().any(|c| c == &s) {
                    causes.push(s);
                }
                current = err.source();
            }
            ui::print_error_box(&format!("{}", e), &causes[1..]);
            std::process::exit(1);
        }
    }
}

fn get_arg(raw: &[String], sub: &str, offset: usize) -> Option<String> {
    let pos = raw.iter().position(|a| a == sub)?;
    raw.get(pos + offset).cloned()
}

fn get_flag(raw: &[String], flag: &str) -> bool {
    raw.iter().any(|a| a == flag)
}

fn run_index(raw: &[String]) {
    let path = get_arg(raw, "index", 1).unwrap_or_else(|| ".".to_string());
    let force = get_flag(raw, "--force") || get_flag(raw, "-f");
    let watch = get_flag(raw, "--watch") || get_flag(raw, "-w");

    if !get_flag(raw, "--quiet") && !get_flag(raw, "-q") {
        ui::print_index_banner(&path);
    }

    let path_buf = std::path::PathBuf::from(&path);
    let exclude: Vec<String> = "target,node_modules,.git,dist,build,venv,__pycache__,.cache"
        .split(',')
        .map(|s| s.to_string())
        .collect();

    match srr::storage::StorageManager::open(&path_buf) {
        Ok(storage) => {
            match srr::index::SemanticIndexer::build_index(&storage, &path_buf, &exclude, true, force) {
                Ok(count) => {
                    println!("  {} Indexed {} symbols", ui::symbols::OK.green().bold(), count.to_string().cyan().bold());
                }
                Err(e) => {
                    ui::print_error_box(&format!("Index failed: {e}"), &[]);
                    std::process::exit(1);
                }
            }
            // Auto-embed if SRR_API_KEY is set
            auto_embed_symbols(&storage);
            if watch {
                if let Err(e) = srr::index::start_watcher(&storage, &path_buf, &exclude) {
                    ui::print_error_box(&format!("Watch failed: {e}"), &[]);
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            ui::print_error_box(&format!("Failed to create index: {e}"), &[]);
            std::process::exit(1);
        }
    }
}

fn auto_embed_symbols(storage: &srr::storage::StorageManager) {
    let api_key = match std::env::var("SRR_API_KEY") {
        Ok(k) if !k.is_empty() => k,
        _ => return,
    };
    let provider = std::env::var("SRR_LLM_PROVIDER").unwrap_or_else(|_| "openrouter".to_string());
    if provider == "dummy" {
        return;
    }
    let model = std::env::var("SRR_LLM_MODEL").unwrap_or_else(|_| "anthropic/claude-3.5-haiku".to_string());
    let embed_model = "openai/text-embedding-3-small";

    let existing = match storage.get_embeddings() {
        Ok(e) => e,
        Err(_) => return,
    };
    let existing_ids: std::collections::HashSet<i64> = existing.iter().map(|(id, _, _)| *id).collect();
    let all_symbols = match storage.get_all_symbols_with_ids() {
        Ok(s) => s,
        Err(_) => return,
    };
    let to_embed: Vec<_> = all_symbols.iter().filter(|(id, _, _, _)| !existing_ids.contains(id)).collect();
    if to_embed.is_empty() {
        return;
    }

    let config = srr::types::LlmConfig {
        provider: provider.clone(),
        model,
        temperature: 0.0,
        max_tokens: 0,
        api_key,
    };
    let client: Box<dyn srr::llm::LlmClient> = match provider.as_str() {
        "openrouter" => Box::new(srr::llm::OpenRouterClient::new(config)),
        "openai" => Box::new(srr::llm::OpenAiClient::new(config)),
        _ => return,
    };

    let mut embedded = 0;
    for chunk in to_embed.chunks(20) {
        let texts: Vec<&str> = chunk.iter().map(|(_, name, sig, _)| {
            if !sig.is_empty() { sig.as_str() } else { name.as_str() }
        }).collect();
        match client.embed(&texts) {
            Ok(embeddings) => {
                let ids: Vec<i64> = chunk.iter().map(|(id, _, _, _)| *id).collect();
                if storage.store_embeddings(&ids, &embeddings, embed_model).is_ok() {
                    embedded += embeddings.len();
                }
            }
            Err(e) => {
                eprintln!("  {} Embedding error: {e}", ui::symbols::ERROR.red());
            }
        }
    }
    if embedded > 0 {
        println!("  {} Embedded {} symbols", ui::symbols::OK.green().bold(), embedded.to_string().cyan().bold());
    }
}

fn run_task(raw: &[String]) {
    let query = get_arg(raw, "task", 1).unwrap_or_default();
    if query.is_empty() {
        eprintln!("  {} Error: task query is required (e.g. 'srr task \"add user authentication\"')",
            ui::symbols::ERROR.red().bold());
        std::process::exit(1);
    }
    let path = get_arg(raw, "task", 2).unwrap_or_else(|| ".".to_string());
    let path_buf = std::path::PathBuf::from(&path);

    match srr::storage::StorageManager::open(&path_buf) {
        Ok(storage) => {
            let engine = srr::task::TaskEngine::new(storage);
            match engine.find_relevant_files(&query, 20) {
                Ok(files) => {
                    println!();
                    println!("  {} Relevant files for \"{}\":", ui::symbols::AGENT.cyan().bold(), query.cyan());
                    for f in &files {
                        println!("    {} {}", "·".cyan(), f.display().to_string().white());
                    }
                    if files.is_empty() {
                        println!("    {} No files found. Run 'compress-context index' first.",
                            ui::symbols::INFO.yellow());
                    }
                    println!();
                }
                Err(e) => {
                    ui::print_error_box(&format!("Task lookup failed: {e}"), &[]);
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            ui::print_error_box(&format!("Failed to open index: {e}. Run 'compress-context index' first."), &[]);
            std::process::exit(1);
        }
    }
}

fn run_plan(raw: &[String]) {
    let query = get_arg(raw, "plan", 1).unwrap_or_default();
    if query.is_empty() {
        eprintln!("  {} Error: plan query is required", ui::symbols::ERROR.red().bold());
        std::process::exit(1);
    }
    let path = get_arg(raw, "plan", 2).unwrap_or_else(|| ".".to_string());
    let path_buf = std::path::PathBuf::from(&path);

    match srr::storage::StorageManager::open(&path_buf) {
        Ok(storage) => {
            let engine = srr::task::TaskEngine::new(storage);
            match engine.generate_plan(&query, 20) {
                Ok(plan) => {
                    ui::print_task_plan(&plan);
                }
                Err(e) => {
                    ui::print_error_box(&format!("Plan generation failed: {e}"), &[]);
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            ui::print_error_box(&format!("Failed to open index: {e}. Run 'compress-context index' first."), &[]);
            std::process::exit(1);
        }
    }
}

fn run_agent(raw: &[String]) {
    let path = get_arg(raw, "agent", 1).unwrap_or_else(|| ".".to_string());
    let project_path = std::path::PathBuf::from(&path);

    if !get_flag(raw, "--quiet") && !get_flag(raw, "-q") {
        ui::print_agent_banner();
    }

    let storage = match srr::storage::StorageManager::open(&project_path) {
        Ok(s) => s,
        Err(e) => {
            ui::print_error_box(&format!("Failed to initialize storage: {e}"), &[]);
            std::process::exit(1);
        }
    };

    let api_key = std::env::var("SRR_API_KEY").unwrap_or_default();
    let provider = std::env::var("SRR_LLM_PROVIDER").unwrap_or_else(|_| {
        if api_key.is_empty() { "dummy".to_string() } else { "openrouter".to_string() }
    });

    let default_model = match provider.as_str() {
        "openrouter" => "anthropic/claude-3.5-haiku",
        "openai" => "gpt-4o",
        "anthropic" => "claude-3-5-haiku-20241022",
        _ => "",
    };

    if provider != "dummy" && api_key.is_empty() {
        eprintln!("  {} SRR_API_KEY not set but provider is '{provider}'. Set SRR_API_KEY or use provider='dummy'.", ui::symbols::ERROR.red().bold());
        std::process::exit(1);
    }

    let llm_config = srr::types::LlmConfig {
        provider: provider.clone(),
        model: std::env::var("SRR_LLM_MODEL").unwrap_or_else(|_| default_model.to_string()),
        temperature: 0.7,
        max_tokens: 4096,
        api_key,
    };

    let llm_client: Box<dyn srr::llm::LlmClient> = match provider.as_str() {
        "openai" => Box::new(srr::llm::OpenAiClient::new(llm_config)),
        "anthropic" => Box::new(srr::llm::AnthropicClient::new(llm_config)),
        "openrouter" => Box::new(srr::llm::OpenRouterClient::new(llm_config)),
        _ => Box::new(srr::llm::DummyClient),
    };

    let mut runtime = srr::agent::AgentRuntime::new(llm_client, project_path).with_storage(storage);
    let system_prompt = "You are SRR Agent, an AI coding assistant with access to repository analysis. \
        You can read files, search symbols, run commands, and help with code tasks. \
        Use <tool_call> blocks to invoke tools:\n\
        <tool_call>\ntool_name\n{\"arg\": \"value\"}\n</tool_call>";

    if let Err(e) = runtime.run_interactive(system_prompt) {
        ui::print_error_box(&format!("Agent error: {e}"), &[]);
        std::process::exit(1);
    }
}

fn run_tui(raw: &[String]) {
    let path = get_arg(raw, "tui", 1).unwrap_or_else(|| ".".to_string());
    let project_path = std::path::PathBuf::from(&path);

    let storage = match srr::storage::StorageManager::open(&project_path) {
        Ok(s) => s,
        Err(e) => {
            ui::print_error_box(&format!("Failed to initialize storage: {e}"), &[]);
            std::process::exit(1);
        }
    };

    let api_key = std::env::var("SRR_API_KEY").unwrap_or_default();
    let provider = std::env::var("SRR_LLM_PROVIDER").unwrap_or_else(|_| {
        if api_key.is_empty() { "dummy".to_string() } else { "openrouter".to_string() }
    });

    let default_model = match provider.as_str() {
        "openrouter" => "anthropic/claude-3.5-haiku",
        "openai" => "gpt-4o",
        "anthropic" => "claude-3-5-haiku-20241022",
        _ => "",
    };

    let llm_config = srr::types::LlmConfig {
        provider: provider.clone(),
        model: std::env::var("SRR_LLM_MODEL").unwrap_or_else(|_| default_model.to_string()),
        temperature: 0.7,
        max_tokens: 4096,
        api_key,
    };

    let llm_client: Box<dyn srr::llm::LlmClient> = match provider.as_str() {
        "openai" => Box::new(srr::llm::OpenAiClient::new(llm_config)),
        "anthropic" => Box::new(srr::llm::AnthropicClient::new(llm_config)),
        "openrouter" => Box::new(srr::llm::OpenRouterClient::new(llm_config)),
        _ => Box::new(srr::llm::DummyClient),
    };

    let runtime = srr::agent::AgentRuntime::new(llm_client, project_path.clone()).with_storage(storage);

    match srr::tui::run_tui(&project_path, runtime) {
        Ok(()) => {}
        Err(e) => {
            ui::print_error_box(&format!("TUI error: {e}"), &[]);
            std::process::exit(1);
        }
    }
}

fn run_search(raw: &[String]) {
    let query = get_arg(raw, "search", 1).unwrap_or_default();
    if query.is_empty() {
        eprintln!("  {} Error: search query is required", ui::symbols::ERROR.red().bold());
        std::process::exit(1);
    }
    let path = get_arg(raw, "search", 2).unwrap_or_else(|| ".".to_string());
    let use_vector = get_flag(raw, "--vector") || get_flag(raw, "-v");
    let path_buf = std::path::PathBuf::from(&path);

    match srr::storage::StorageManager::open(&path_buf) {
        Ok(storage) => {
            if use_vector {
                let results = vector_search(&storage, &query);
                match results {
                    Ok(symbols) => ui::print_search_results(&query, &symbols),
                    Err(e) => {
                        ui::print_error_box(&format!("Vector search failed: {e}"), &[]);
                        std::process::exit(1);
                    }
                }
            } else {
                match storage.search_symbols(&query, 50) {
                    Ok(symbols) => {
                        ui::print_search_results(&query, &symbols);
                    }
                    Err(e) => {
                        ui::print_error_box(&format!("Search failed: {e}"), &[]);
                        std::process::exit(1);
                    }
                }
            }
        }
        Err(e) => {
            ui::print_error_box(&format!("Failed to open index: {e}. Run 'compress-context index' first."), &[]);
            std::process::exit(1);
        }
    }
}

fn vector_search(storage: &srr::storage::StorageManager, query: &str) -> srr::error::SrrResult<Vec<srr::types::Symbol>> {
    let api_key = std::env::var("SRR_API_KEY")
        .map_err(|_| anyhow::anyhow!("SRR_API_KEY required for vector search"))?;
    let provider = std::env::var("SRR_LLM_PROVIDER").unwrap_or_else(|_| "openrouter".to_string());
    let model = std::env::var("SRR_LLM_MODEL").unwrap_or_else(|_| "anthropic/claude-3.5-haiku".to_string());
    let config = srr::types::LlmConfig {
        provider,
        model,
        temperature: 0.0,
        max_tokens: 0,
        api_key,
    };
    let client: Box<dyn srr::llm::LlmClient> = match std::env::var("SRR_LLM_PROVIDER").unwrap_or_default().as_str() {
        "openai" => Box::new(srr::llm::OpenAiClient::new(config)),
        _ => Box::new(srr::llm::OpenRouterClient::new(config)),
    };

    let query_emb = client.embed(&[query])
        .map_err(|e| anyhow::anyhow!("Failed to embed query: {e}"))?;
    let query_vec = match query_emb.first() {
        Some(v) => v,
        None => return Ok(Vec::new()),
    };

    let stored = storage.get_embeddings()?;
    if stored.is_empty() {
        return Ok(Vec::new());
    }

    // Get symbol details from storage
    let all_symbols = storage.get_all_symbols_with_ids()?;
    let sym_map: std::collections::HashMap<i64, &(i64, String, String, String)> =
        all_symbols.iter().map(|s| (s.0, s)).collect();

    // Compute cosine similarity
    struct Scored {
        id: i64,
        score: f64,
    }
    let mut scored: Vec<Scored> = stored.iter()
        .filter(|(id, emb, _)| sym_map.contains_key(id) && emb.len() == query_vec.len())
        .map(|(id, emb, _)| {
            let dot: f64 = emb.iter().zip(query_vec.iter()).map(|(a, b)| (*a as f64) * (*b as f64)).sum();
            let norm_a: f64 = emb.iter().map(|v| (*v as f64).powi(2)).sum::<f64>().sqrt();
            let norm_b: f64 = query_vec.iter().map(|v| (*v as f64).powi(2)).sum::<f64>().sqrt();
            let score = if norm_a > 0.0 && norm_b > 0.0 { dot / (norm_a * norm_b) } else { 0.0 };
            Scored { id: *id, score }
        })
        .collect();
    scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(20);

    let mut symbols = Vec::new();
    for s in &scored {
        if let Some((_, name, sig, file_path)) = sym_map.get(&s.id) {
            symbols.push(srr::types::Symbol {
                name: name.clone(),
                kind: srr::types::SymbolKind::Function,
                file_path: std::path::PathBuf::from(file_path),
                line: 0,
                column: 0,
                signature: Some(sig.clone()),
                doc_comment: None,
            });
        }
    }
    Ok(symbols)
}
