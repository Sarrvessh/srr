use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "compress-context",
    about = "SRR - AI Coding Agent Platform: context compression, semantic indexing, agent runtime",
    long_about = "SRR (Semantic Repository Reader) analyzes a project directory and generates AI-optimized \
context packages, builds semantic indexes, understands tasks, and provides an agent runtime.\n\n\
Subcommands: compress, index, task, plan, agent, tui, search\n\
Default (no subcommand): compress mode with backward-compatible CLI flags.",
    version,
)]
pub struct Cli {
    #[arg(help = "Path to the project directory to analyze")]
    pub path: String,

    #[arg(long, short = 'o', help = "Output file path (default: context.md or context.json)")]
    pub output: Option<String>,

    #[arg(long, help = "Output in JSON format instead of Markdown")]
    pub json: bool,

    #[arg(long, short = 'v', help = "Enable verbose logging with detailed per-file information")]
    pub verbose: bool,

    #[arg(
        long,
        default_value = "target,node_modules,.git,dist,build,venv,__pycache__,.cache",
        help = "Comma-separated list of directories/files to exclude"
    )]
    pub exclude: String,

    #[arg(long, help = "Maximum tokens for the generated context package")]
    pub max_tokens: Option<usize>,

    #[arg(
        long,
        default_value = "detailed",
        help = "Summary level: 'compact' (brief) or 'detailed' (comprehensive)"
    )]
    pub summary_level: String,

    #[arg(
        long,
        default_value = "gpt4o",
        help = "Model for cost estimation: 'gpt4o', 'claude', or 'gemini'"
    )]
    pub model: String,

    #[arg(long, help = "Suppress startup banner")]
    pub no_banner: bool,

    #[arg(
        long,
        help = "Path to configuration file (default: .srrrc in current or home directory)"
    )]
    pub config: Option<String>,

    #[arg(long, short = 'g', help = "Respect .gitignore rules when scanning (uses ripgrep's ignore engine)")]
    pub respect_gitignore: bool,

    #[arg(
        long,
        short = 'i',
        help = "Comma-separated glob patterns to include (e.g. '**/*.rs,**/*.toml')"
    )]
    pub include: Option<String>,

    #[arg(long, help = "Compress output with gzip (appends .gz extension)")]
    pub gzip: bool,

    #[arg(long, short = 'q', help = "Quiet mode — suppress banner, progress bars, and results")]
    pub quiet: bool,

    #[arg(long, help = "Disable colored output")]
    pub no_color: bool,

    #[arg(long, help = "Number of threads for parallel processing")]
    pub concurrency: Option<usize>,

    #[arg(long, help = "Preview mode — scan and show metrics without generating output")]
    pub dry_run: bool,
}
