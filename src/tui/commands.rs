use std::sync::LazyLock;

use crate::agent::AgentMode;

pub struct Cmd {
    pub name: &'static str,
    pub desc: &'static str,
    pub usage: &'static str,
}

static ALL_COMMANDS: LazyLock<Vec<Cmd>> = LazyLock::new(|| {
    vec![
        Cmd { name: "/help", desc: "Show this help screen", usage: "/help" },
        Cmd { name: "/clear", desc: "Clear the conversation", usage: "/clear" },
        Cmd { name: "/compact", desc: "Compact conversation (keeps last 10 messages)", usage: "/compact" },
        Cmd { name: "/mode", desc: "Show current mode", usage: "/mode" },
        Cmd { name: "/plan", desc: "Toggle plan mode (read-only)", usage: "/plan" },
        Cmd { name: "/yolo", desc: "Toggle YOLO mode (auto-approve)", usage: "/yolo" },
        Cmd { name: "/model", desc: "Set the LLM model (e.g. /model gpt-4o)", usage: "/model <name>" },
        Cmd { name: "/exit", desc: "Exit the TUI", usage: "/exit" },
    ]
});

pub fn all() -> &'static Vec<Cmd> {
    &ALL_COMMANDS
}

pub fn execute(input: &str, mode: &mut AgentMode) -> Option<String> {
    let trimmed = input.trim();
    if !trimmed.starts_with('/') {
        return None;
    }
    let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
    let cmd_name = parts[0];

    match cmd_name {
    "/help" => Some("__help__".to_string()),
    "/clear" => Some("__clear__".to_string()),
    "/compact" => Some("__compact__".to_string()),
        "/mode" => {
            let msg = format!("Current mode: **{}** mode", mode.as_str());
            Some(msg)
        }
        "/plan" => {
            *mode = if *mode == AgentMode::Plan { AgentMode::Agent } else { AgentMode::Plan };
            Some(format!("Switched to **{}** mode", mode.as_str()))
        }
        "/yolo" => {
            *mode = if *mode == AgentMode::Yolo { AgentMode::Agent } else { AgentMode::Yolo };
            Some(format!("Switched to **{}** mode", mode.as_str()))
        }
        "/model" => {
            if parts.len() > 1 {
                let model_name = parts[1].trim();
                Some(format!("__model__:{}", model_name))
            } else {
                Some("Usage: /model <name>  (e.g. /model gpt-4o)".to_string())
            }
        }
        "/exit" => Some("__exit__".to_string()),
        _ => Some(format!("Unknown command: {}\nType /help for available commands.", cmd_name)),
    }
}
