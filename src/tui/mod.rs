pub mod layout;
pub mod title;
pub mod conversation;
pub mod input;
pub mod status;
pub mod keybind;
pub mod commands;
pub mod help;
pub mod approval;
pub mod autocomplete;
pub mod markdown;

use std::path::Path;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::Frame;

use crate::agent::{AgentMode, AgentRuntime, PendingToolCall};
use crate::error::SrrResult;
use crate::types::{LlmMessage, ToolResult};
use crate::theme::Theme;
use crate::types::Session;
use approval::PendingApproval;
use autocomplete::Autocomplete;
use keybind::{Action, BindingContext, default_bindings, lookup};

#[derive(Clone)]
pub struct ChatMessage {
    role: String,
    content: String,
}

const POLL_MS: u64 = 100;
const STREAM_POLL_MS: u64 = 16;
const MAX_TOOL_ROUNDS: usize = 5;
const MAX_MESSAGES: usize = 200;
const COMPACT_THRESHOLD: usize = 75_000;

enum AppState {
    Idle,
    Streaming {
        stream: Box<dyn Iterator<Item = SrrResult<String>>>,
        full_text: String,
        user_msg: String,
    },
    ToolFollowUp {
        stream: Box<dyn Iterator<Item = SrrResult<String>>>,
        full_text: String,
    },
}

struct App {
    messages: Vec<ChatMessage>,
    input: String,
    cursor: usize,
    input_history: Vec<String>,
    history_pos: Option<usize>,
    scroll: usize,
    auto_scroll: bool,
    running: bool,
    runtime: AgentRuntime,
    theme: Theme,
    status: String,
    mode: AgentMode,
    model: String,
    show_help: bool,
    pending_approvals: Vec<PendingApproval>,
    selected_approval: usize,
    state: AppState,
    autocomplete: Autocomplete,
    session: Option<Session>,
    line_count: usize,
    // Phase A+D — tool call pipeline state
    parsed_tool_calls: Vec<PendingToolCall>,
    instant_tool_results: Vec<(String, ToolResult)>,
    pending_response_text: String,
    #[allow(dead_code)]
    tool_round: usize,
    quit_pending: bool,
    quit_pending_at: Instant,
    compacted: bool,
}

impl App {
    fn new(mut runtime: AgentRuntime, theme: Theme, model: String) -> Self {
        let _ = runtime.start_session(None);
        Self {
            messages: Vec::new(),
            input: String::new(),
            cursor: 0,
            input_history: Vec::new(),
            history_pos: None,
            scroll: 0,
            auto_scroll: true,
            running: true,
            runtime,
            theme,
            status: "Ready — type a message and press Enter".to_string(),
            mode: AgentMode::Agent,
            model,
            show_help: false,
            pending_approvals: Vec::new(),
            selected_approval: 0,
            state: AppState::Idle,
            autocomplete: Autocomplete::new(),
            session: None,
            line_count: 1,
            parsed_tool_calls: Vec::new(),
            instant_tool_results: Vec::new(),
            pending_response_text: String::new(),
            tool_round: 0,
            quit_pending: false,
            quit_pending_at: Instant::now(),
            compacted: false,
        }
    }

    fn add_msg(&mut self, role: &str, content: String) {
        self.messages.push(ChatMessage { role: role.to_string(), content: content.clone() });
        while self.messages.len() > MAX_MESSAGES {
            self.messages.remove(0);
        }
        self.runtime.add_to_session(role, &content, Vec::new());
    }

    fn estimated_tokens(&self) -> usize {
        self.messages.iter().map(|m| m.content.len()).sum::<usize>() / 4
    }

    fn maybe_compact(&mut self) {
        if self.compacted {
            return;
        }
        if self.estimated_tokens() < COMPACT_THRESHOLD || self.messages.len() < 5 {
            return;
        }
        let keep = self.messages.split_off(self.messages.len().saturating_sub(10));
        self.messages = keep;
        self.messages.insert(0, ChatMessage {
            role: "system".to_string(),
            content: "Conversation compacted. Earlier messages removed.".to_string(),
        });
        self.scroll = 0;
        self.compacted = true;
        self.status = "Conversation auto-compacted".to_string();
    }

    fn cycle_mode(&mut self) {
        self.quit_pending = false;
        self.mode = self.mode.next();
        self.runtime.set_mode(self.mode);
        self.add_msg("system", format!("Switched to {} mode", self.mode.as_str()));
        self.auto_scroll = true;
    }

    fn is_streaming(&self) -> bool {
        matches!(self.state, AppState::Streaming { .. } | AppState::ToolFollowUp { .. })
    }

    fn interrupt(&mut self) {
        self.state = AppState::Idle;
        self.status = "Interrupted".to_string();
        self.pending_approvals.clear();
        self.parsed_tool_calls.clear();
        self.instant_tool_results.clear();
        self.pending_response_text.clear();
    }

    fn send_message(&mut self) {
        self.tool_round = 0;
        let msg = std::mem::take(&mut self.input);
        self.cursor = 0;
        self.line_count = 1;
        if msg.is_empty() {
            return;
        }
        if msg.eq_ignore_ascii_case("exit") || msg.eq_ignore_ascii_case("quit") {
            self.running = false;
            return;
        }
        self.input_history.push(msg.clone());
        self.history_pos = None;

        if msg.starts_with('/') {
            let result = commands::execute(&msg, &mut self.mode);
            if let Some(response) = result {
                match response.as_str() {
                    "__help__" => { self.show_help = true; return; }
                    "__clear__" => { self.messages.clear(); self.scroll = 0; self.instant_tool_results.clear(); self.parsed_tool_calls.clear(); self.pending_approvals.clear(); self.selected_approval = 0; self.pending_response_text.clear(); self.auto_scroll = true; self.compacted = false; return; }
                    "__compact__" => {
                        if self.messages.len() > 10 {
                            let keep = self.messages.split_off(self.messages.len().saturating_sub(10));
                            self.messages = keep;
                            self.messages.insert(0, ChatMessage { role: "system".to_string(), content: "Conversation compacted. Earlier messages removed.".to_string() });
                            self.scroll = 0;
                        }
                        self.compacted = false;
                        return;
                    }
                    "__exit__" => { self.running = false; return; }
                    cmd if cmd.starts_with("__model__:") => {
                        let model = cmd.trim_start_matches("__model__:").to_string();
                        self.model = model.clone();
                        self.runtime.set_model(&model);
                        self.add_msg("system", format!("Model switched to **{}**", model));
                        self.auto_scroll = true;
                        return;
                    }
                    _ => {
                        self.add_msg("system", response);
                        self.auto_scroll = true;
                        return;
                    }
                }
            }
        }

        self.add_msg("user", msg.clone());

        // Build messages for LLM
        let mode_info = match self.mode {
            AgentMode::Plan => "\nYou are in PLAN MODE — read-only research and planning. Only use read_file, glob, search_symbols, and search_code tools.".to_string(),
            AgentMode::Yolo => "\nYou are in YOLO MODE — all tools are auto-approved.".to_string(),
            AgentMode::Agent => String::new(),
        };
        let tool_desc = self.runtime.tool_descriptions();
        let sys_msg = LlmMessage {
            role: "system".to_string(),
            content: format!("You are SRR Agent. Available tools:\n{}{}", tool_desc, mode_info),
            tool_calls: None,
        };
        let user_msg = LlmMessage {
            role: "user".to_string(),
            content: msg.clone(),
            tool_calls: None,
        };

        match self.runtime.stream_chat(&[sys_msg, user_msg]) {
            Ok(stream) => {
                self.state = AppState::Streaming {
                    stream,
                    full_text: String::new(),
                    user_msg: msg,
                };
                self.status = "Streaming...".to_string();
                self.auto_scroll = true;
            }
            Err(e) => {
                self.add_msg("error", format!("Error: {e}"));
                self.status = "Error".to_string();
            }
        }
    }

    fn start_follow_up(&mut self, response_text: &str, tool_calls_text: &str) {
        let followup_msg = if tool_calls_text.is_empty() {
            format!("Tool results:\n{}", response_text)
        } else {
            format!("Continue with the tool results above.\n\n{}", tool_calls_text)
        };

        let mode_info = match self.mode {
            AgentMode::Plan => "\nYou are in PLAN MODE — read-only research and planning. Only use read_file, glob, search_symbols, and search_code tools.".to_string(),
            AgentMode::Yolo => "\nYou are in YOLO MODE — all tools are auto-approved.".to_string(),
            AgentMode::Agent => String::new(),
        };
        let sys_content = format!("You are SRR Agent. Available tools:\n{}{}", self.runtime.tool_descriptions(), mode_info);
        let mut llm_messages = vec![
            LlmMessage { role: "system".to_string(), content: sys_content, tool_calls: None },
        ];
        // Include conversation history for context continuity
        for msg in &self.messages {
            if msg.role == "user" || msg.role == "assistant" {
                llm_messages.push(LlmMessage {
                    role: msg.role.clone(),
                    content: msg.content.clone(),
                    tool_calls: None,
                });
            }
        }
        llm_messages.push(LlmMessage {
            role: "user".to_string(),
            content: followup_msg,
            tool_calls: None,
        });

        match self.runtime.stream_chat(&llm_messages) {
            Ok(stream) => {
                self.state = AppState::ToolFollowUp {
                    stream,
                    full_text: String::new(),
                };
                self.status = "Streaming follow-up...".to_string();
                self.auto_scroll = true;
            }
            Err(e) => {
                self.add_msg("error", self.categorize_error(&e.to_string()));
                self.status = "Ready".to_string();
                self.state = AppState::Idle;
            }
        }
    }

    fn approve_current(&mut self) {
        if self.pending_approvals.is_empty() {
            return;
        }
        let tool_name = self.pending_approvals[self.selected_approval].tool_name.clone();
        let args = self.pending_approvals[self.selected_approval].args.clone();
        let result = self.runtime.run_tool(&tool_name, &args);
        let msg = format!("{}: {}", tool_name, if result.success { &result.output } else { result.error.as_deref().unwrap_or("error") });
        self.add_msg("tool_result", msg);
        // Store result for follow-up
        self.instant_tool_results.push((tool_name, result));
        self.pending_approvals.remove(self.selected_approval);
        if self.selected_approval >= self.pending_approvals.len() {
            self.selected_approval = self.pending_approvals.len().saturating_sub(1);
        }
        if self.pending_approvals.is_empty() {
            // All approved — start follow-up
            let response_text = std::mem::take(&mut self.pending_response_text);
            let tool_text = self.tool_results_string();
            self.instant_tool_results.clear();
            self.parsed_tool_calls.clear();
            self.auto_scroll = true;
            self.start_follow_up(&response_text, &tool_text);
        }
    }

    fn approve_all(&mut self) {
        while !self.pending_approvals.is_empty() {
            self.selected_approval = 0;
            self.approve_current();
        }
    }

    fn reject_approval(&mut self) {
        self.pending_approvals.clear();
        self.selected_approval = 0;
        self.parsed_tool_calls.clear();
        self.instant_tool_results.clear();
        self.pending_response_text.clear();
        // Remove the rejected assistant response from displayed messages and session
        if self.messages.last().map(|m| m.role.as_str()) == Some("assistant") {
            self.messages.pop();
            self.runtime.remove_last_assistant_message();
        }
        self.status = "Ready".to_string();
    }

    fn tool_results_string(&self) -> String {
        self.instant_tool_results.iter()
            .map(|(name, r)| {
                format!("{}: {}", name, if r.success { &r.output } else { r.error.as_deref().unwrap_or("error") })
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn insert_char(&mut self, c: char) {
        self.input.insert(self.cursor, c);
        self.cursor += c.len_utf8();
        self.line_count = self.input.chars().filter(|&ch| ch == '\n').count() + 1;
        self.autocomplete.update(&self.input, self.cursor);
    }

    fn delete_before_cursor(&mut self) {
        if self.cursor > 0 {
            let prev = self.input[..self.cursor].char_indices().last();
            if let Some((start, _c)) = prev {
                self.input.remove(start);
                self.cursor = start;
            }
            self.line_count = self.input.chars().filter(|&ch| ch == '\n').count() + 1;
            self.autocomplete.update(&self.input, self.cursor);
        }
    }

    fn delete_at_cursor(&mut self) {
        if self.cursor < self.input.len() && self.input.is_char_boundary(self.cursor) {
            self.input.remove(self.cursor);
            self.line_count = self.input.chars().filter(|&ch| ch == '\n').count() + 1;
            self.autocomplete.update(&self.input, self.cursor);
        }
    }

    fn cursor_left(&mut self) {
        if self.cursor > 0 {
            let before = &self.input[..self.cursor];
            self.cursor = before.char_indices().last().map(|(i, _)| i).unwrap_or(0);
            self.autocomplete.update(&self.input, self.cursor);
        }
    }

    fn cursor_right(&mut self) {
        if self.cursor < self.input.len() {
            if let Some(c) = self.input[self.cursor..].chars().next() {
                self.cursor += c.len_utf8();
            }
            self.autocomplete.update(&self.input, self.cursor);
        }
    }

    fn navigate_history(&mut self, direction: isize) {
        if self.input_history.is_empty() {
            return;
        }
        let pos = match self.history_pos {
            Some(p) => {
                let new = p as isize + direction;
                if new < 0 || new >= self.input_history.len() as isize { return; }
                new as usize
            }
            None => {
                if direction > 0 { return; }
                self.input_history.len() - 1
            }
        };
        self.history_pos = Some(pos);
        self.input = self.input_history[pos].clone();
        self.cursor = self.input.len();
        self.line_count = self.input.chars().filter(|&ch| ch == '\n').count() + 1;
        self.autocomplete.update(&self.input, self.cursor);
    }

    fn handle_action(&mut self, action: Action, key: crossterm::event::KeyEvent) {
        // Reset quit confirmation on any action except Quit
        if action != Action::Quit && self.quit_pending {
            self.quit_pending = false;
            self.status = "Ready".to_string();
        }

        // Handle streaming interrupt first
        if self.is_streaming() {
            if matches!(action, Action::Quit | Action::Interrupt) {
                self.interrupt();
                return;
            }
            // Regular char input is ignored while streaming
            return;
        }

        // If autocomplete is active
        if self.autocomplete.active {
            match action {
                Action::HistoryNext | Action::HistoryPrev => {
                    if action == Action::HistoryNext {
                        self.autocomplete.select_next();
                    } else {
                        self.autocomplete.select_prev();
                    }
                    return;
                }
                Action::Submit => {
                    let trigger = self.autocomplete.trigger;
                    let completed = self.autocomplete.apply(&self.input);
                    self.input = completed;
                    self.cursor = self.input.len();
                    self.line_count = self.input.chars().filter(|&ch| ch == '\n').count() + 1;
                    self.autocomplete.deactivate();
                    if trigger == '/' {
                        self.send_message();
                    }
                    return;
                }
                Action::Quit => {
                    self.autocomplete.deactivate();
                    return;
                }
                _ => {}
            }
        }

        // Handle approval panel
        if !self.pending_approvals.is_empty() {
            match action {
                Action::Submit | Action::None => {
                    if let KeyCode::Char(c) = key.code {
                        match c {
                            '1' => self.approve_current(),
                            '2' => self.approve_all(),
                            '3' => self.reject_approval(),
                            _ => {}
                        }
                    }
                }
                Action::Quit => self.reject_approval(),
                _ => {}
            }
            return;
        }

        match action {
            Action::Submit => self.send_message(),
            Action::Backspace => self.delete_before_cursor(),
            Action::Delete => self.delete_at_cursor(),
            Action::CursorLeft => self.cursor_left(),
            Action::CursorRight => self.cursor_right(),
            Action::CursorHome => self.cursor = 0,
            Action::CursorEnd => self.cursor = self.input.len(),
            Action::HistoryPrev => {
                if !self.autocomplete.active {
                    self.navigate_history(-1);
                }
            }
            Action::HistoryNext => {
                if !self.autocomplete.active {
                    self.navigate_history(1);
                }
            }
            Action::CycleMode => self.cycle_mode(),
            Action::Quit => {
                if self.show_help {
                    self.show_help = false;
                    self.quit_pending = false;
                } else if self.quit_pending && self.quit_pending_at.elapsed() < Duration::from_secs(2) {
                    self.running = false;
                } else {
                    self.quit_pending = true;
                    self.quit_pending_at = Instant::now();
                    self.status = "Press Esc again to quit".to_string();
                }
            }
            Action::ScrollUp => {
                self.auto_scroll = false;
                self.scroll = self.scroll.saturating_sub(5);
            }
            Action::ScrollDown => {
                self.scroll = self.scroll.saturating_add(5)
                    .min(self.messages.len().saturating_sub(1));
            }
            Action::ScrollToBottom => {
                self.auto_scroll = true;
                self.scroll = 0;
            }
            Action::ClearInput | Action::Interrupt => {
                self.input.clear();
                self.cursor = 0;
                self.line_count = 1;
            }
            Action::Newline => {
                self.input.push('\n');
                self.cursor += 1;
                self.line_count += 1;
            }
            Action::ExternalEditor => {
                if let Ok(text) = open_external_editor(&self.input) {
                    self.input = text;
                    self.cursor = self.input.len();
                    self.line_count = self.input.chars().filter(|&ch| ch == '\n').count() + 1;
                }
            }
            Action::None => {
                if let KeyCode::Char(c) = key.code {
                    if key.modifiers == KeyModifiers::NONE || key.modifiers == KeyModifiers::SHIFT {
                        self.insert_char(c);
                    }
                }
            }
        }
    }

    fn categorize_error(&self, err: &str) -> String {
        let lower = err.to_lowercase();
        if lower.contains("api_key") || lower.contains("api key") || lower.contains("unauthorized") || lower.contains("401") {
            "Set SRR_API_KEY and try again.".to_string()
        } else if lower.contains("rate limit") || lower.contains("429") || lower.contains("too many requests") {
            "Rate limited. Waiting before retry...".to_string()
        } else if lower.contains("timeout") || lower.contains("timed out") || lower.contains("connection") {
            "Connection issue. Check your network.".to_string()
        } else if lower.contains("context") || lower.contains("maximum length") || lower.contains("too long") {
            "Context window exceeded. Use /clear to reset.".to_string()
        } else {
            format!("Error: {err}")
        }
    }

    fn handle_stream_completion(&mut self, response_text: String) {
        // Parse tool calls (no execution)
        let tool_calls = self.runtime.parse_tool_calls(&response_text);

        if tool_calls.is_empty() {
            self.state = AppState::Idle;
            self.status = "Ready".to_string();
            self.auto_scroll = true;
            return;
        }

        // Separate into pending (needs approval) and instant (auto-execute)
        let mut pending = Vec::new();
        let mut instant = Vec::new();

        for tc in &tool_calls {
            if self.mode != AgentMode::Yolo && AgentRuntime::tool_needs_approval(&tc.name) {
                if self.mode.allows_tool(&tc.name) {
                    pending.push(PendingApproval {
                        tool_name: tc.name.clone(),
                        args: tc.args.clone(),
                        index: pending.len(),
                        total: 0,
                    });
                } else {
                    instant.push((tc.name.clone(), ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(format!("Tool '{}' not available in {} mode", tc.name, self.mode.as_str())),
                    }));
                }
            } else {
                let result = self.runtime.run_tool(&tc.name, &tc.args);
                instant.push((tc.name.clone(), result));
            }
        }

        // Update total in pending
        let pending_total = pending.len();
        for p in &mut pending {
            p.total = pending_total;
        }

        if !pending.is_empty() {
            // Show auto-executed results in conversation immediately
            for (name, result) in &instant {
                let display = format!("{}: {}", name,
                    if result.success { &result.output } else { result.error.as_deref().unwrap_or("error") });
                self.add_msg("tool_result", display);
            }
            self.parsed_tool_calls = tool_calls;
            self.instant_tool_results = instant;
            self.pending_response_text = response_text;
            self.pending_approvals = pending;
            self.selected_approval = 0;
            self.state = AppState::Idle;
            self.status = "Approve tool execution".to_string();
        } else {
            // Show tool results in conversation before checking max rounds
            for (name, result) in &instant {
                let display = format!("{}: {}", name,
                    if result.success { &result.output } else { result.error.as_deref().unwrap_or("error") });
                self.add_msg("tool_result", display);
            }
            self.instant_tool_results = instant;
            let tool_text = self.tool_results_string();
            self.instant_tool_results.clear();
            if self.tool_round >= MAX_TOOL_ROUNDS {
                self.add_msg("system", format!("Max tool rounds ({}) reached.", MAX_TOOL_ROUNDS));
                self.state = AppState::Idle;
                self.status = "Ready".to_string();
                return;
            }
            self.tool_round += 1;
            self.start_follow_up(&response_text, &tool_text);
        }
    }

    fn tick(&mut self) {
        match &mut self.state {
            AppState::Streaming { stream, full_text, user_msg } => {
                match stream.next() {
                    Some(Ok(token)) => {
                        full_text.push_str(&token);
                    }
                    Some(Err(e)) => {
                        self.add_msg("error", self.categorize_error(&e.to_string()));
                        self.state = AppState::Idle;
                        self.status = "Error".to_string();
                    }
                    None => {
                        let response_text = std::mem::take(full_text);
                        let _user_input = std::mem::take(user_msg);
                        if response_text.is_empty() {
                            self.state = AppState::Idle;
                            self.status = "Ready".to_string();
                            return;
                        }
                        self.add_msg("assistant", response_text.clone());
                        self.handle_stream_completion(response_text);
                        self.maybe_compact();
                    }
                }
            }
            AppState::ToolFollowUp { stream, full_text } => {
                match stream.next() {
                    Some(Ok(token)) => full_text.push_str(&token),
                    Some(Err(e)) => {
                        self.add_msg("error", self.categorize_error(&e.to_string()));
                        self.state = AppState::Idle;
                        self.status = "Error".to_string();
                    }
                    None => {
                        let text = std::mem::take(full_text);
                        if text.is_empty() {
                            self.state = AppState::Idle;
                            self.status = "Ready".to_string();
                            return;
                        }
                        self.add_msg("assistant", text.clone());
                        self.handle_stream_completion(text);
                        self.maybe_compact();
                    }
                }
            }
            AppState::Idle => {}
        }
    }
}

pub fn run_tui(project_path: &Path, runtime: AgentRuntime) -> SrrResult<()> {
    let theme_path = project_path.join(".srr").join("theme.json");
    let theme = if theme_path.exists() {
        Theme::load(&theme_path).unwrap_or_else(|_| Theme::tokyonight())
    } else {
        Theme::tokyonight()
    };

    let model = std::env::var("SRR_LLM_MODEL").unwrap_or_else(|_| "claude-3.5-haiku".to_string());

    let mut terminal = ratatui::init();
    let mut app = App::new(runtime, theme, model);
    let bindings = default_bindings();
    let mut last_tick = Instant::now();

    while app.running {
        terminal.draw(|f| render(f, &mut app))?;

        // Determine poll rate — faster during streaming
        let poll_ms = if app.is_streaming() { STREAM_POLL_MS } else { POLL_MS };
        let timeout = Duration::from_millis(poll_ms).saturating_sub(last_tick.elapsed());

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    let ctx = if !app.pending_approvals.is_empty() {
                        BindingContext::Approval
                    } else if app.is_streaming() {
                        BindingContext::Streaming
                    } else if app.autocomplete.active {
                        BindingContext::Autocomplete
                    } else {
                        BindingContext::Input
                    };
                    let action = lookup(key, ctx, &bindings);
                    app.handle_action(action, key);
                }
            }
        }

        // Tick the streaming state machine
        let elapsed = last_tick.elapsed();
        if elapsed >= Duration::from_millis(poll_ms) {
            app.tick();
            last_tick = Instant::now();
        }
    }

    // Save session on exit
    if let Some(storage) = app.runtime.storage_ref() {
        if let Some(session) = &app.session {
            let _ = storage.save_session(session);
        }
    }

    ratatui::restore();
    Ok(())
}

fn open_external_editor(initial: &str) -> Result<String, String> {
    let editor = std::env::var("EDITOR").or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| if cfg!(windows) { "notepad".to_string() } else { "vim".to_string() });
    let tmp_path = std::env::temp_dir().join("srr_input.txt");
    if !initial.is_empty() {
        std::fs::write(&tmp_path, initial).map_err(|e| e.to_string())?;
    }
    let status = std::process::Command::new(&editor)
        .arg(&tmp_path)
        .status()
        .map_err(|e| format!("Failed to launch editor '{}': {}", editor, e))?;
    if !status.success() {
        return Err(format!("Editor exited with status: {status}"));
    }
    let content = std::fs::read_to_string(&tmp_path).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&tmp_path);
    Ok(content)
}

fn render(f: &mut Frame, app: &mut App) {
    let mut extra = 0u16;
    if app.autocomplete.active {
        extra = (app.autocomplete.results.len() as u16).min(5) + 1;
    }
    let input_height = (app.line_count as u16 + 2 + extra).min(10);
    let chunks = layout::chunks(f.area(), input_height);
    let model_for_bar = app.model.clone();
    let mode_str = app.mode.as_str();

    let total_chars: usize = app.messages.iter().map(|m| m.content.len()).sum();
    let est_tokens = total_chars / 4;
    let context_pct = ((est_tokens as f64 / 100_000.0) * 100.0) as usize;
    title::render(f, chunks[0], &app.theme, &model_for_bar, mode_str, context_pct.min(100));

    if app.show_help {
        help::render(f, chunks[1], &app.theme);
    } else if !app.pending_approvals.is_empty() {
        approval::render(f, chunks[1], &app.theme, &app.pending_approvals, app.selected_approval);
    } else {
        // Get streaming buffer
        let streaming_buffer = match &app.state {
            AppState::Streaming { full_text, .. } if !full_text.is_empty() => Some(full_text.as_str()),
            AppState::ToolFollowUp { full_text, .. } if !full_text.is_empty() => Some(full_text.as_str()),
            _ => None,
        };
        conversation::render(
            f,
            chunks[1],
            &app.theme,
            &app.messages,
            streaming_buffer,
            app.scroll,
            app.auto_scroll,
        );
    }

    let input_text = app.input.clone();
    let cursor_pos = app.cursor;
    input::render(f, chunks[2], &app.theme, &input_text, cursor_pos, &app.autocomplete);
    status::render(f, chunks[3], &app.theme, mode_str, &model_for_bar, &app.status, est_tokens);
}
