use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::LazyLock;
use colored::Colorize;
use regex::Regex;
use chrono::Utc;
use serde_json::Value;

use crate::error::SrrResult;
use crate::llm::{LlmClient, StreamIter};
use crate::types::LlmMessage;
use crate::types::*;
use crate::verify::VerificationEngine;
use crate::storage::StorageManager;
static TOOL_CALL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"<tool_call>\s*(\w+)\s*\n([\s\S]*?)</tool_call>").unwrap()
});

pub trait AgentTool: Send {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn run(&self, args: &Value) -> ToolResult;
}

pub struct ReadFileTool;

impl AgentTool for ReadFileTool {
    fn name(&self) -> &str { "read_file" }
    fn description(&self) -> &str { "Read the contents of a file. Args: {\"path\": \"<file_path>\"}" }
    fn run(&self, args: &Value) -> ToolResult {
        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
        match std::fs::read_to_string(path) {
            Ok(content) => {
                let truncated = if content.len() > 50000 {
                    format!("{}...\n[truncated at 50000 chars]", &content[..50000])
                } else {
                    content
                };
                ToolResult { success: true, output: truncated, error: None }
            }
            Err(e) => ToolResult { success: false, output: String::new(), error: Some(e.to_string()) },
        }
    }
}

pub struct GlobSearchTool;

impl AgentTool for GlobSearchTool {
    fn name(&self) -> &str { "glob" }
    fn description(&self) -> &str { "Search for files matching a glob pattern. Args: {\"pattern\": \"**/*.rs\"}" }
    fn run(&self, args: &Value) -> ToolResult {
        let pattern = args.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        match glob::glob(&format!("{}/{}", path.trim_end_matches('/'), pattern).replace("//", "/")) {
            Ok(entries) => {
                let files: Vec<String> = entries
                    .filter_map(|e| e.ok().map(|p| p.to_string_lossy().to_string()))
                    .collect();
                ToolResult { success: true, output: files.join("\n"), error: None }
            }
            Err(e) => ToolResult { success: false, output: String::new(), error: Some(e.to_string()) },
        }
    }
}

pub struct RunCommandTool;

impl AgentTool for RunCommandTool {
    fn name(&self) -> &str { "run_command" }
    fn description(&self) -> &str { "Run a shell command. Args: {\"command\": \"<cmd>\", \"timeout\": <ms>}" }
    fn run(&self, args: &Value) -> ToolResult {
        let command = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
        let timeout_ms = args.get("timeout").and_then(|v| v.as_u64()).unwrap_or(30_000);
        let workdir = args.get("workdir").and_then(|v| v.as_str()).unwrap_or(".");
        let output = std::process::Command::new("cmd")
            .args(["/C", command])
            .current_dir(workdir)
            .output();
        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                let combined = if stderr.is_empty() { stdout } else { format!("{stdout}\n--- stderr ---\n{stderr}") };
                let truncated = if combined.len() > timeout_ms as usize {
                    "...\n[truncated]".to_string()
                } else {
                    combined
                };
                ToolResult { success: out.status.success(), output: truncated, error: None }
            }
            Err(e) => ToolResult { success: false, output: String::new(), error: Some(e.to_string()) },
        }
    }
}

pub struct SearchSymbolTool {
    storage: StorageManager,
}

impl AgentTool for SearchSymbolTool {
    fn name(&self) -> &str { "search_symbols" }
    fn description(&self) -> &str { "Search for symbols in the codebase index. Args: {\"query\": \"<search term>\"}" }
    fn run(&self, args: &Value) -> ToolResult {
        let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
        match self.storage.search_symbols(query, limit) {
            Ok(symbols) => {
                let lines: Vec<String> = symbols.iter().map(|s| {
                    format!("{} {}:{}  {} ({})",
                        s.kind.as_str(), s.file_path.display(), s.line,
                        s.name, s.signature.as_deref().unwrap_or(""))
                }).collect();
                ToolResult { success: true, output: lines.join("\n"), error: None }
            }
            Err(e) => ToolResult { success: false, output: String::new(), error: Some(e.to_string()) },
        }
    }
}

pub struct EditFileTool;

impl AgentTool for EditFileTool {
    fn name(&self) -> &str { "edit_file" }
    fn description(&self) -> &str { "Edit a file by replacing text. Args: {\"path\": \"<path>\", \"old\": \"<text to replace>\", \"new\": \"<replacement text>\"}" }
    fn run(&self, args: &Value) -> ToolResult {
        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
        let old = args.get("old").and_then(|v| v.as_str()).unwrap_or("");
        let new = args.get("new").and_then(|v| v.as_str()).unwrap_or("");
        if path.is_empty() || old.is_empty() {
            return ToolResult { success: false, output: String::new(), error: Some("path and old are required".to_string()) };
        }
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => return ToolResult { success: false, output: String::new(), error: Some(e.to_string()) },
        };
        if !content.contains(old) {
            return ToolResult { success: false, output: String::new(), error: Some("old text not found in file".to_string()) };
        }
        let new_content = content.replace(old, new);
        match std::fs::write(path, &new_content) {
            Ok(()) => ToolResult { success: true, output: format!("Edited {path}"), error: None },
            Err(e) => ToolResult { success: false, output: String::new(), error: Some(e.to_string()) },
        }
    }
}

pub struct CreateFileTool;

impl AgentTool for CreateFileTool {
    fn name(&self) -> &str { "create_file" }
    fn description(&self) -> &str { "Create a new file with content. Args: {\"path\": \"<path>\", \"content\": \"<file content>\"}" }
    fn run(&self, args: &Value) -> ToolResult {
        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
        let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
        if path.is_empty() {
            return ToolResult { success: false, output: String::new(), error: Some("path is required".to_string()) };
        }
        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.exists() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    return ToolResult { success: false, output: String::new(), error: Some(e.to_string()) };
                }
            }
        }
        match std::fs::write(path, content) {
            Ok(()) => ToolResult { success: true, output: format!("Created {path}"), error: None },
            Err(e) => ToolResult { success: false, output: String::new(), error: Some(e.to_string()) },
        }
    }
}

pub struct SearchCodeTool;

impl AgentTool for SearchCodeTool {
    fn name(&self) -> &str { "search_code" }
    fn description(&self) -> &str { "Regex search across all files. Args: {\"pattern\": \"<regex>\", \"path\": \"<root dir>\"}" }
    fn run(&self, args: &Value) -> ToolResult {
        let pattern = args.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
        let root = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        if pattern.is_empty() {
            return ToolResult { success: false, output: String::new(), error: Some("pattern is required".to_string()) };
        }
        let re = match Regex::new(pattern) {
            Ok(r) => r,
            Err(e) => return ToolResult { success: false, output: String::new(), error: Some(format!("invalid regex: {e}")) },
        };
        let mut results = Vec::new();
        let walker = walkdir::WalkDir::new(root)
            .into_iter()
            .filter_entry(|e| {
                e.file_name().to_string_lossy().starts_with('.')
                    && e.file_type().is_dir()
                    && e.file_name() != ".git"
            });
        for entry in walker.filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                let path = entry.path();
                if let Ok(content) = std::fs::read_to_string(path) {
                    for (i, line) in content.lines().enumerate() {
                        if re.is_match(line) {
                            results.push(format!("{}:{}  {}", path.display(), i + 1, line.trim()));
                        }
                    }
                }
            }
        }
        if results.len() > 100 {
            results.truncate(100);
            results.push("... truncated at 100 results".to_string());
        }
        ToolResult { success: true, output: results.join("\n"), error: None }
    }
}

pub struct RunLintTool;

impl AgentTool for RunLintTool {
    fn name(&self) -> &str { "run_lint" }
    fn description(&self) -> &str { "Run the linter on the project. Args: {\"path\": \"<project dir>\"} (optional)" }
    fn run(&self, args: &Value) -> ToolResult {
        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        let path_buf = std::path::PathBuf::from(path);
        let result = VerificationEngine::run_lint(&path_buf);
        let output = if result.passed {
            format!("Lint passed ({})", result.tool)
        } else {
            format!("Lint failed ({})\n{}", result.tool, result.output)
        };
        ToolResult { success: result.passed, output, error: None }
    }
}

pub struct RunTestTool;

impl AgentTool for RunTestTool {
    fn name(&self) -> &str { "run_tests" }
    fn description(&self) -> &str { "Run the test suite. Args: {\"path\": \"<project dir>\"} (optional)" }
    fn run(&self, args: &Value) -> ToolResult {
        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        let path_buf = std::path::PathBuf::from(path);
        let result = VerificationEngine::run_tests(&path_buf);
        let output = if result.passed {
            format!("Tests passed ({})", result.tool)
        } else {
            format!("Tests failed ({})\n{}", result.tool, result.output)
        };
        ToolResult { success: result.passed, output, error: None }
    }
}

#[derive(Debug, Clone)]
pub struct PendingToolCall {
    pub name: String,
    pub args: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentMode {
    Agent,
    Plan,
    Yolo,
}

impl AgentMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentMode::Agent => "agent",
            AgentMode::Plan => "plan",
            AgentMode::Yolo => "yolo",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            AgentMode::Agent => AgentMode::Plan,
            AgentMode::Plan => AgentMode::Yolo,
            AgentMode::Yolo => AgentMode::Agent,
        }
    }

    pub fn allows_tool(&self, tool_name: &str) -> bool {
        match self {
            AgentMode::Yolo => true,
            AgentMode::Agent => true,
            AgentMode::Plan => matches!(tool_name,
                "read_file" | "glob" | "search_symbols" | "search_code"),
        }
    }
}

pub struct AgentRuntime {
    tools: HashMap<String, Box<dyn AgentTool>>,
    storage: Option<StorageManager>,
    llm: Box<dyn LlmClient>,
    project_path: PathBuf,
    session: Option<Session>,
    mode: AgentMode,
}

impl AgentRuntime {
    pub fn new(llm: Box<dyn LlmClient>, project_path: PathBuf) -> Self {
        let session_storage = StorageManager::open(&project_path).ok();
        let mut runtime = Self {
            tools: HashMap::new(),
            storage: session_storage,
            llm,
            project_path,
            session: None,
            mode: AgentMode::Agent,
        };
        runtime.register_tool(Box::new(ReadFileTool));
        runtime.register_tool(Box::new(GlobSearchTool));
        runtime.register_tool(Box::new(RunCommandTool));
        runtime.register_tool(Box::new(EditFileTool));
        runtime.register_tool(Box::new(CreateFileTool));
        runtime.register_tool(Box::new(SearchCodeTool));
        runtime.register_tool(Box::new(RunLintTool));
        runtime.register_tool(Box::new(RunTestTool));
        runtime
    }

    pub fn with_storage(mut self, storage: StorageManager) -> Self {
        self.storage = Some(storage);
        if let Ok(storage) = StorageManager::open(&self.project_path) {
            self.register_tool(Box::new(SearchSymbolTool { storage }));
        }
        self
    }

    pub fn set_mode(&mut self, mode: AgentMode) {
        self.mode = mode;
    }

    pub fn set_model(&mut self, model: &str) {
        self.llm.set_model(model);
    }

    pub fn mode(&self) -> AgentMode {
        self.mode
    }

    pub fn llm(&self) -> &dyn LlmClient {
        &*self.llm
    }

    pub fn stream_chat(&self, messages: &[LlmMessage]) -> SrrResult<StreamIter> {
        self.llm.chat_stream(messages)
    }

    pub fn storage_ref(&self) -> Option<&StorageManager> {
        self.storage.as_ref()
    }

    pub fn register_tool(&mut self, tool: Box<dyn AgentTool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn start_session(&mut self, task: Option<String>) -> SrrResult<()> {
        let session = Session {
            id: uuid::Uuid::new_v4().to_string(),
            project_path: self.project_path.clone(),
            task: task.clone(),
            messages: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        self.session = Some(session);
        Ok(())
    }

    pub fn run_interactive(&mut self, system_prompt: &str) -> SrrResult<()> {
        self.start_session(None)?;
        println!("{}", ui::style_agent_msg("Agent session started. Type your request or 'exit' to quit."));

        let sys_msg = LlmMessage {
            role: "system".to_string(),
            content: format!("{}\n\nAvailable tools:\n{}", system_prompt, self.tool_descriptions()),
            tool_calls: None,
        };

        loop {
            let mut input = String::new();
            if std::io::stdin().read_line(&mut input).is_err() || input.trim().eq_ignore_ascii_case("exit") {
                break;
            }
            let input = input.trim().to_string();
            if input.is_empty() {
                continue;
            }

            let user_msg = LlmMessage {
                role: "user".to_string(),
                content: input.clone(),
                tool_calls: None,
            };

            self.add_to_session("user", &input, Vec::new());
            let mut messages = vec![sys_msg.clone()];
            for m in &self.session.as_ref().unwrap().messages {
                messages.push(LlmMessage {
                    role: m.role.clone(),
                    content: m.content.clone(),
                    tool_calls: Some(m.tool_calls.iter().map(|t| ToolCall { name: t.name.clone(), args: t.args.clone(), result: None }).collect()),
                });
            }
            messages.push(user_msg);

            // Use streaming response
            let response_text = match self.llm.chat_stream(&messages) {
                Ok(stream) => {
                    let mut full = String::new();
                    for chunk in stream {
                        match chunk {
                            Ok(text) => {
                                full.push_str(&text);
                                print!("{}", text);
                                use std::io::Write;
                                std::io::stdout().flush().ok();
                            }
                            Err(e) => {
                                eprintln!("\n  {} LLM stream error: {}", symbols::ERROR.red(), e);
                                break;
                            }
                        }
                    }
                    println!();
                    full
                }
                Err(e) => {
                    eprintln!("  {} LLM error: {}", symbols::ERROR.red(), e);
                    continue;
                }
            };

            if response_text.is_empty() {
                continue;
            }

            let tool_calls = self.process_response(&response_text);
            let has_tools = !tool_calls.is_empty();

            if has_tools {
                self.add_to_session("assistant", &response_text, tool_calls);

                let mut followup = vec![sys_msg.clone()];
                for m in &self.session.as_ref().unwrap().messages {
                    followup.push(LlmMessage {
                        role: m.role.clone(),
                        content: m.content.clone(),
                        tool_calls: None,
                    });
                }
                followup.push(LlmMessage {
                    role: "user".to_string(),
                    content: "Continue with the tool results above.".to_string(),
                    tool_calls: None,
                });

                if let Ok(r2) = self.llm.chat(&followup) {
                    println!("\n{}", ui::style_agent_response(&r2.content));
                    self.add_to_session("assistant", &r2.content, Vec::new());
                }
            } else {
                self.add_to_session("assistant", &response_text, tool_calls);
            }
        }

        if let Some(session) = self.session.take() {
            if let Some(ref storage) = self.storage {
                let _ = storage.save_session(&session);
            }
        }
        Ok(())
    }

    /// Send user input to LLM and return (response_text, parsed_tool_calls_with_results)
    pub fn call_llm(&self, user_input: &str) -> SrrResult<(String, Vec<ToolCall>)> {
        let mode_info = match self.mode {
            AgentMode::Plan => "\nYou are in PLAN MODE — read-only research and planning. Only use read_file, glob, search_symbols, and search_code tools. Do not edit files or run commands.".to_string(),
            AgentMode::Yolo => "\nYou are in YOLO MODE — all tools are auto-approved.".to_string(),
            AgentMode::Agent => String::new(),
        };
        let sys_msg = LlmMessage {
            role: "system".to_string(),
            content: format!("You are SRR Agent. Available tools:\n{}{}", self.tool_descriptions(), mode_info),
            tool_calls: None,
        };
        let user_msg = LlmMessage {
            role: "user".to_string(),
            content: user_input.to_string(),
            tool_calls: None,
        };
        let response = self.llm.chat(&[sys_msg, user_msg])?;
        let tool_calls = self.process_response(&response.content);
        Ok((response.content, tool_calls))
    }

    /// Execute a specific tool call and return the result
    pub fn run_tool(&self, name: &str, args: &serde_json::Value) -> ToolResult {
        self.tools.get(name).map(|tool| tool.run(args))
            .unwrap_or_else(|| ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Unknown tool: {name}")),
            })
    }

    /// Check if a tool name typically needs user approval
    pub fn tool_needs_approval(name: &str) -> bool {
        matches!(name, "edit_file" | "create_file" | "run_command")
    }

    pub fn execute_one(&mut self, user_input: &str) -> SrrResult<String> {
        let (_, tool_calls) = self.call_llm(user_input)?;
        if tool_calls.is_empty() {
            return Ok(String::new());
        }
        let mut results = Vec::new();
        for tc in &tool_calls {
            if let Some(ref r) = tc.result {
                results.push(format!("{}: {}", tc.name, if r.success { &r.output } else { r.error.as_deref().unwrap_or("error") }));
            }
        }
        Ok(results.join("\n"))
    }

    pub fn process_response(&self, content: &str) -> Vec<ToolCall> {
        let mut calls = Vec::new();
        for cap in TOOL_CALL_RE.captures_iter(content) {
            let name = cap[1].to_string();
            if !self.mode.allows_tool(&name) {
                calls.push(ToolCall {
                    name: name.clone(),
                    args: Value::Null,
                    result: Some(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(format!("Tool '{name}' not available in {} mode", self.mode.as_str())),
                    }),
                });
                continue;
            }
            let args_str = cap[2].trim();
            let args: Value = serde_json::from_str(args_str).unwrap_or(Value::Null);
            let result = self.tools.get(&name).map(|tool| tool.run(&args));
            calls.push(ToolCall { name, args, result });
        }
        calls
    }

    /// Parse tool calls from response text WITHOUT executing them
    pub fn parse_tool_calls(&self, content: &str) -> Vec<PendingToolCall> {
        let mut calls = Vec::new();
        for cap in TOOL_CALL_RE.captures_iter(content) {
            let name = cap[1].to_string();
            if !self.mode.allows_tool(&name) {
                calls.push(PendingToolCall { name, args: serde_json::Value::Null });
                continue;
            }
            let args_str = cap[2].trim();
            let args: serde_json::Value = serde_json::from_str(args_str).unwrap_or(serde_json::Value::Null);
            calls.push(PendingToolCall { name, args });
        }
        calls
    }

    pub fn tool_descriptions(&self) -> String {
        self.tools.iter()
            .filter(|(name, _)| self.mode.allows_tool(name))
            .map(|(_, t)| format!("- {}: {}", t.name(), t.description()))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn add_to_session(&mut self, role: &str, content: &str, tool_calls: Vec<ToolCall>) {
        if let Some(ref mut session) = self.session {
            session.messages.push(AgentMessage {
                role: role.to_string(),
                content: content.to_string(),
                tool_calls,
                timestamp: Utc::now(),
            });
            session.updated_at = Utc::now();
        }
    }

    /// Remove the last assistant message from the session (e.g. after rejection)
    pub fn remove_last_assistant_message(&mut self) {
        if let Some(ref mut session) = self.session {
            if session.messages.last().map(|m| m.role.as_str()) == Some("assistant") {
                session.messages.pop();
                session.updated_at = Utc::now();
            }
        }
    }
}

use crate::ui;
use crate::ui::symbols;
