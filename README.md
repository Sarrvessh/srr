# SRR — AI Coding Agent Platform

Repository intelligence, semantic indexing, agent runtime, vector search, and LLM context compression — all in one CLI.

```
$ compress-context tui
```

## Features

### 🤖 AI Agent (TUI & CLI)
- **Terminal UI** — Ratatui-based TUI with streaming, markdown rendering, autocomplete, and session persistence
- **Three modes**: Agent (tool approval), Plan (read-only research), Yolo (auto-approve all tools)
- **10 tools**: `read_file`, `edit_file`, `create_file`, `delete_file`, `glob`, `search_symbols`, `search_code`, `run_command`, `run_lint`, `run_tests`
- **Approval pipeline** — Tools require user approval before execution (except in Yolo mode)
- **Multi-round tool loops** — Up to 5 rounds of tool calls per user message
- **Streaming SSE** — Real-time token streaming with interrupt support
- **Conversation persistence** — Sessions auto-saved, restored on TUI start
- **Markdown rendering** — Bold, italic, links, blockquotes, strikethrough, headings, lists
- **Autocomplete** — `/commands` and `@files` with file caching

### 📦 Context Compression (original)
- **Smart File Scanning** — Recursive project scanning with binary detection, gitignore support
- **Duplicate Detection** — SHA-256 exact match + Jaccard near-duplicate detection
- **CRUD Pattern Analysis** — Regex-based detection of create/read/update/delete patterns
- **Architecture Analysis** — Layer classification (frontend, API, services, database, config, docs)
- **Dependency Graph** — petgraph-directed graph of file dependencies
- **File Scoring** — 5-factor importance ranking (entry point, imports, references, config, depth)
- **Domain Clustering** — File clustering by path-based domain detection
- **Token Budget Control** — `--max-tokens` and `--summary-level` for fine-grained control
- **LLM-Optimized Markdown** — 10-section markdown with project summary, architecture, rankings, clusters, patterns, metrics

### 🔍 Search & Indexing
- **Tree-sitter semantic indexing** — Rust symbol extraction (functions, structs, enums, traits, impls, modules)
- **Regex-based extraction** — Python functions/classes, JS/TS functions/classes/interfaces/types/variables, Go functions/structs/interfaces
- **FTS5 full-text search** — `compress-context search <query>`
- **Vector search** — `compress-context search --vector <query>` with cosine similarity ranking
- **Watch mode** — `compress-context index --watch` for incremental re-indexing
- **OpenRouter embeddings** — Flexible LLM provider support

### 💰 Cost Estimation
- Per-model cost savings for GPT-4o, Claude 3.5 Sonnet, Gemini 1.5 Pro
- Token counting via tiktoken-rs (cl100k_base)

## Quick Start

### Prerequisites
- Rust 1.75+ (install via [rustup](https://rustup.rs))
- Windows MSVC build tools or system C compiler

### Install

```bash
cargo install --path .
```

Or build from source:

```bash
git clone https://github.com/user/srr
cd srr
cargo build --release
```

### Run the TUI

```bash
# Set your API key (OpenRouter, OpenAI, or Anthropic)
export SRR_API_KEY="sk-or-..."

# Start the TUI
compress-context tui

# Or with a specific project directory
compress-context tui /path/to/project
```

### First Message
1. Type a question or task and press Enter
2. The LLM streams its response in real-time
3. If it needs to run tools, you'll see an approval panel
4. Press `1` to approve, `2` for all, `3` to reject

## Subcommands

| Command | Description |
|---------|-------------|
| `compress-context tui [path]` | Launch the Terminal UI |
| `compress-context agent [path]` | CLI-based interactive agent |
| `compress-context compress [path]` | Run context compression pipeline |
| `compress-context index [path]` | Build or update semantic index |
| `compress-context search <query>` | FTS5 full-text search |
| `compress-context search --vector <query>` | Vector cosine-similarity search |
| `compress-context task <query>` | Task breakdown engine |
| `compress-context plan <query>` | Generate task plan |

## TUI Keybindings

| Key | Action |
|-----|--------|
| `Enter` | Send message |
| `Shift+Enter` / `Ctrl+J` | Insert newline |
| `Esc` | Quit / back / interrupt |
| `Ctrl+C` | Interrupt streaming / clear input |
| `Shift+Tab` | Cycle mode (Agent → Plan → Yolo) |
| `Ctrl+G` | Open external editor |
| `Ctrl+End` | Scroll to bottom / auto-scroll |
| `↑` / `↓` | Input history / autocomplete navigation |
| `←` / `→` | Move cursor |
| `Home` / `End` | Jump to start/end |
| `Backspace` / `Delete` | Delete before/at cursor |
| `PgUp` / `PgDn` | Scroll conversation |

### In Approval Panel
| Key | Action |
|-----|--------|
| `1` | Approve current tool |
| `2` | Approve all pending tools |
| `3` | Reject all |
| `Esc` | Cancel / close panel |

### Commands (type `/` in input)
| Command | Description |
|---------|-------------|
| `/help` | Show help screen |
| `/clear` | Clear conversation |
| `/compact` | Keep last 10 messages |
| `/mode` | Show current agent mode |
| `/plan` | Toggle plan mode |
| `/yolo` | Toggle YOLO mode |
| `/exit` | Exit TUI |

## Agent Modes

| Mode | Tool Approval | Use Case |
|------|--------------|----------|
| **Agent** | Approval required for `edit_file`, `create_file`, `run_command` | Safe default for daily coding |
| **Plan** | Read-only tools only (`read_file`, `glob`, `search_*`) | Research and architecture exploration |
| **Yolo** | All tools auto-approved | When you trust the LLM and want speed |

### Available Tools

| Tool | Description | Needs Approval |
|------|-------------|----------------|
| `read_file` | Read a file's contents (50KB max) | No |
| `glob` | Search files by glob pattern | No |
| `search_symbols` | FTS5 search of indexed symbols | No |
| `search_code` | Regex search across all files | No |
| `edit_file` | Replace text in a file | Yes (Agent) |
| `create_file` | Create a new file with content | Yes (Agent) |
| `run_command` | Execute a shell command | Yes (Agent) |
| `run_lint` | Run the project linter | No |
| `run_tests` | Run the test suite | No |

## Configuration

### Environment Variables
| Variable | Default | Description |
|----------|---------|-------------|
| `SRR_API_KEY` | — | API key for LLM provider |
| `SRR_LLM_PROVIDER` | `openrouter` | `openrouter`, `openai`, `anthropic`, or `dummy` |
| `SRR_LLM_MODEL` | per-provider | Model name override |

Default models per provider:
- `openrouter` → `anthropic/claude-3.5-haiku`
- `openai` → `gpt-4o`
- `anthropic` → `claude-3-5-haiku-20241022`

### Themes
Themes are loaded from `.srr/theme.json` in the project directory. Built-in themes:
- Tokyo Night (default)
- Catppuccin Macchiato
- Gruvbox Dark

## Architecture

```
src/
├── main.rs               # Entry point, CLI dispatch
├── lib.rs                # Library root
├── cli.rs                # Clap argument definitions
├── config.rs             # Configuration from CLI + env
├── error.rs              # Typed error hierarchy (thiserror)
├── types.rs              # Shared data structures (Symbol, Session, ToolCall, etc.)
├── pipeline.rs           # 10-phase compression orchestration
├── agent/
│   ├── mod.rs            # AgentRuntime, tools (read_file, edit_file, glob, run_command, etc.)
│   └── ...
├── tui/
│   ├── mod.rs            # App state machine, rendering, event handling
│   ├── autocomplete.rs   # /command and @file autocomplete
│   ├── input.rs          # Input widget with cursor + autocomplete popup
│   ├── conversation.rs   # Message list with markdown rendering
│   ├── approval.rs       # Tool approval overlay
│   ├── markdown.rs       # Inline markdown parser (bold, italic, links, etc.)
│   ├── layout.rs         # Terminal layout chunks
│   ├── keybind.rs        # Keybinding definitions and lookup
│   ├── commands.rs       # Slash commands
│   ├── title.rs          # Title bar (mode icon, model, context %)
│   ├── status.rs         # Status bar (mode, model, status text, token count)
│   └── help.rs           # Help overlay
├── llm/
│   ├── mod.rs            # LlmClient trait, streaming interface
│   ├── openrouter.rs     # OpenRouter SSE client
│   ├── openai.rs         # OpenAI API client
│   ├── anthropic.rs      # Anthropic API client
│   └── dummy.rs          # Test dummy client
├── index/
│   └── mod.rs            # Tree-sitter + regex symbol extraction
├── storage/
│   ├── mod.rs            # SQLite storage (FTS5, embeddings, sessions, symbols)
│   └── ...
├── scanner/              # File scanning with gitignore support
├── dedup/                # SHA-256 exact + Jaccard near-duplicate detection
├── pattern/              # CRUD pattern detection
├── architecture/         # Layer classification + dependency graph
├── scoring/              # 5-factor file importance ranking
├── clustering/           # Path-based domain clustering
├── compressors/          # Specialized compressors (log, doc, code)
├── context/              # Markdown + JSON context generation
├── metrics/              # Token reduction, retention, cost savings
├── tokenizer/            # tiktoken-rs wrapper + pricing
└── theme.rs              # Color theme system
```

## Design Principles

1. **Trait-based modularity** — Every major component is defined by a trait for testability and future extension
2. **Never panic** — All errors handled via `Result` with typed `SrrError` hierarchy
3. **Parallel by default** — File scanning and scoring use Rayon work-stealing
4. **Graceful degradation** — Binary files, large files, and encoding issues are handled silently
5. **Windows-native** — Full Windows support with MSVC build tools

## License

MIT
