use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use petgraph::graph::DiGraph;

pub type DepGraph = DiGraph<String, String>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: PathBuf,
    pub relative_path: PathBuf,
    pub extension: String,
    pub size_bytes: u64,
    pub line_count: usize,
    pub is_binary: bool,
    pub content: Option<String>,
    pub token_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateGroup {
    pub reason: String,
    pub files: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub pattern_type: String,
    pub entity: String,
    pub operations: Vec<String>,
    pub files: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Architecture {
    pub layers: Vec<LayerInfo>,
    pub graph_dot: String,
    pub hierarchy_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerInfo {
    pub name: String,
    pub file_count: usize,
    pub technologies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredFile {
    pub path: PathBuf,
    pub score: f64,
    pub token_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cluster {
    pub name: String,
    pub description: String,
    pub files: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedSection {
    pub section_type: String,
    pub content: String,
    pub original_tokens: usize,
    pub compressed_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogGroup {
    pub level: String,
    pub message_template: String,
    pub occurrences: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompressionMetrics {
    pub original_tokens: usize,
    pub compressed_tokens: usize,
    pub reduction_percent: f64,
    pub estimated_retention_percent: f64,
    pub cost_savings_gpt4o: f64,
    pub cost_savings_claude: f64,
    pub cost_savings_gemini: f64,
    pub original_lines: usize,
    pub compressed_lines: usize,
    pub original_files: usize,
    pub duplicate_files_removed: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SummaryLevel {
    Compact,
    Detailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ModelType {
    Gpt4o,
    Claude35Sonnet,
    Gemini15Pro,
}

// ── Semantic Index Types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub file_path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub signature: Option<String>,
    pub doc_comment: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SymbolKind {
    Function,
    Method,
    Struct,
    Trait,
    Enum,
    Type,
    Module,
    Class,
    Interface,
    Variable,
    Constant,
    Macro,
    Import,
}

impl SymbolKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            SymbolKind::Function => "function",
            SymbolKind::Method => "method",
            SymbolKind::Struct => "struct",
            SymbolKind::Trait => "trait",
            SymbolKind::Enum => "enum",
            SymbolKind::Type => "type",
            SymbolKind::Module => "module",
            SymbolKind::Class => "class",
            SymbolKind::Interface => "interface",
            SymbolKind::Variable => "variable",
            SymbolKind::Constant => "constant",
            SymbolKind::Macro => "macro",
            SymbolKind::Import => "import",
        }
    }
}

// ── Knowledge Graph Types ──

pub type KGraph = petgraph::graph::DiGraph<GraphNode, GraphEdge>;

#[derive(Debug, Clone)]
pub struct GraphNode {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub file: Option<PathBuf>,
}

impl std::fmt::Display for GraphNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} [{}]", self.name, self.id)
    }
}

#[derive(Debug, Clone)]
pub struct GraphEdge {
    pub relation: String,
}

impl std::fmt::Display for GraphEdge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.relation)
    }
}

#[derive(Debug, Clone)]
pub struct ProjectGraph {
    pub graph: KGraph,
}

// ── LLM Types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    pub role: String,
    pub content: String,
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub provider: String,
    pub model: String,
    pub temperature: f64,
    pub max_tokens: usize,
    pub api_key: String,
}

// ── Agent Types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub args: serde_json::Value,
    pub result: Option<ToolResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub role: String,
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub project_path: PathBuf,
    pub task: Option<String>,
    pub messages: Vec<AgentMessage>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

// ── Task Engine Types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskIntent {
    pub intent_type: IntentType,
    pub targets: Vec<String>,
    pub constraints: Vec<String>,
    pub query: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum IntentType {
    Refactor,
    AddFeature,
    FixBug,
    Explain,
    Optimize,
    Test,
    Document,
    Search,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPlan {
    pub intent: TaskIntent,
    pub steps: Vec<PlanStep>,
    pub relevant_files: Vec<PathBuf>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub order: usize,
    pub action: String,
    pub description: String,
    pub files: Vec<PathBuf>,
}

// ── Verification Types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub tool: String,
    pub passed: bool,
    pub output: String,
    pub duration_ms: u64,
}

// ── Index Types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexConfig {
    pub path: PathBuf,
    pub force: bool,
    pub watch: bool,
}

impl std::fmt::Display for ModelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelType::Gpt4o => write!(f, "GPT-4o"),
            ModelType::Claude35Sonnet => write!(f, "Claude 3.5 Sonnet"),
            ModelType::Gemini15Pro => write!(f, "Gemini 1.5 Pro"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectState {
    pub project_name: String,
    pub primary_language: String,
    pub files: Vec<FileEntry>,
    pub total_tokens: usize,
    pub duplicate_groups: Vec<DuplicateGroup>,
    pub near_duplicate_groups: Vec<DuplicateGroup>,
    pub patterns: Vec<Pattern>,
    pub architecture: Architecture,
    pub scores: Vec<ScoredFile>,
    pub clusters: Vec<Cluster>,
    pub log_summary: Option<CompressedSection>,
    pub doc_summary: Option<CompressedSection>,
    pub metrics: CompressionMetrics,
}
