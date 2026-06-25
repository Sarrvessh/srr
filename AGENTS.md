# SRR — Project Context

## Build Commands
- `cargo build --release` — Production build
- `cargo clippy --all-targets -- -D warnings` — Lint check (must pass)
- `cargo test` — Run all 79+ tests
- `cargo run --release -- tui` — Launch TUI (requires `SRR_API_KEY`)

## Environment
- **Platform**: Windows (MSVC), Rust Stable
- **Binary name**: `compress-context`, **Package name**: `srr`
- **Default model**: `anthropic/claude-3.5-haiku` via OpenRouter
- **API key**: `SRR_API_KEY`

## Architecture Overview
- `src/main.rs` — CLI dispatch (compress, index, search, agent, tui, task, plan)
- `src/agent/` — AgentRuntime, 10 tools, LlmClient abstraction
- `src/tui/` — Ratatui-based terminal UI with 13+ modules
- `src/index/` — Tree-sitter + regex symbol extraction
- `src/storage/` — SQLite with FTS5, embeddings, sessions, symbols
- `src/llm/` — OpenRouter, OpenAI, Anthropic, Dummy clients
- Original compression pipeline: scanner, dedup, pattern, architecture, scoring, clustering, compressors, context, metrics

## Current State
- **79+ tests passing**, clippy clean, release build clean
- Wave 3 complete (13 phases A-M): tool approval pipeline, reliable streaming, multi-round tool loops, multi-byte safe cursor, autocomplete popup, session persistence, error UX, confirm-before-quit, visual polish
- 10+ additional UX bug fixes applied (cursor, follow-up context, history-autocomplete, scroll indicator, etc.)
- 8 recent fixes: cursor_left multi-byte, MAX_TOOL_ROUNDS result display, auto-executed tool visibility, quit_pending status restore, /clear approval state cleanup, rejected message session cleanup, autocomplete apply bug, auto-execute commands after autocomplete

## Code Conventions
- Trait-based modular design
- Never panic — all errors via Result + SrrError
- Parallel via Rayon where applicable
- No comments in code unless explicitly required
- `add_msg()` for dual-persistence (messages + session)
- Windows path handling throughout

## Key Files
- `src/tui/mod.rs` — App struct (30+ fields), state machine, 788 lines
- `src/tui/autocomplete.rs` — /command and @file autocomplete
- `src/tui/input.rs` — Input widget with cursor rendering
- `src/tui/markdown.rs` — Inline markdown rendering
- `src/tui/approval.rs` — Tool approval panel
- `src/agent/mod.rs` — AgentRuntime, PendingToolCall, 10 tool implementations
- `src/index/mod.rs` — Tree-sitter Rust + regex Python/JS/Go/Go symbol extraction
