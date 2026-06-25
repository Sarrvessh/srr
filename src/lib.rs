//! SRR - Context Compressor: Reduce LLM context size while preserving maximum useful information.
//!
//! SRR analyzes a project directory and generates an AI-optimized context package
//! that can be fed into GPT, Claude, Gemini, or any LLM agent system.
//!
//! It detects redundant information, duplicate content, repetitive code patterns,
//! summarizes architecture, compresses logs, ranks file importance, and generates
//! an LLM-ready context package with compression metrics.
//!
//! # Usage
//!
//! ```bash
//! compress-context /path/to/project --output context.md
//! compress-context /path/to/project --json --output context.json
//! compress-context /path/to/project --max-tokens 8000 --summary-level compact
//! ```
//!
//! # Architecture
//!
//! The pipeline consists of 10 phases, each defined by a trait:
//!
//! | Phase | Trait | Description |
//! |-------|-------|-------------|
//! | Scan | `FileScanner` | Recursive file discovery with exclusion filtering |
//! | Dedup | `DuplicateDetector` | Exact SHA-256 + Jaccard near-duplicate detection |
//! | Patterns | `PatternDetector` | Regex-based CRUD operation detection |
//! | Architecture | `ArchitectureAnalyzer` | Layer classification + dependency graph |
//! | Scoring | `FileScorer` | 5-factor importance ranking |
//! | Clustering | `FileClusterer` | Path-based domain clustering |
//! | Compression | `FileCompressor` | Log/doc/code content compression |
//! | Context | `ContextWriter` | Markdown/JSON output generation |
//! | Metrics | `calculator` | Token reduction, retention, cost savings |
//!
//! Every phase is trait-based for testability and future extension
//! (Claude API, GPT-4o mini, Gemini, RAG, Vector stores, etc.).

pub mod cli;
pub mod config;
pub mod error;
pub mod types;
pub mod scanner;
pub mod tokenizer;
pub mod dedup;
pub mod pattern;
pub mod architecture;
pub mod scoring;
pub mod clustering;
pub mod compressors;
pub mod context;
pub mod metrics;
pub mod pipeline;
pub mod ui;
pub mod storage;
pub mod index;
pub mod graph;
pub mod llm;
pub mod task;
pub mod agent;
pub mod verify;
pub mod tui;
pub mod theme;
