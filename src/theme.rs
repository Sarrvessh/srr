use std::path::Path;
use ratatui::style::Color;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Theme {
    pub name: String,
    pub user: String,
    pub assistant: String,
    pub tool_call: String,
    pub tool_result: String,
    pub error: String,
    pub success: String,
    pub status: String,
    pub border: String,
    pub input_bg: String,
    pub conversation_bg: String,
    pub accent: String,
}

impl Theme {
    pub fn tokyonight() -> Self {
        Self {
            name: "tokyonight".into(),
            user: "#7aa2f7".into(),
            assistant: "#9ece6a".into(),
            tool_call: "#bb9af7".into(),
            tool_result: "#565f89".into(),
            error: "#f7768e".into(),
            success: "#9ece6a".into(),
            status: "#565f89".into(),
            border: "#3b4261".into(),
            input_bg: "#1a1b26".into(),
            conversation_bg: "#24283b".into(),
            accent: "#f7768e".into(),
        }
    }

    pub fn catppuccin_macchiato() -> Self {
        Self {
            name: "catppuccin-macchiato".into(),
            user: "#8aadf4".into(),
            assistant: "#a6da95".into(),
            tool_call: "#c6a0f6".into(),
            tool_result: "#5b6078".into(),
            error: "#ed8796".into(),
            success: "#a6da95".into(),
            status: "#5b6078".into(),
            border: "#494d64".into(),
            input_bg: "#1e2030".into(),
            conversation_bg: "#24273a".into(),
            accent: "#f5a97f".into(),
        }
    }

    pub fn gruvbox_dark() -> Self {
        Self {
            name: "gruvbox-dark".into(),
            user: "#83a598".into(),
            assistant: "#b8bb26".into(),
            tool_call: "#d3869b".into(),
            tool_result: "#504945".into(),
            error: "#fb4934".into(),
            success: "#b8bb26".into(),
            status: "#504945".into(),
            border: "#665c54".into(),
            input_bg: "#1d2021".into(),
            conversation_bg: "#282828".into(),
            accent: "#fe8019".into(),
        }
    }

    pub fn parse_color(hex: &str) -> Color {
        let hex = hex.trim_start_matches('#');
        if hex.len() == 6 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&hex[0..2], 16),
                u8::from_str_radix(&hex[2..4], 16),
                u8::from_str_radix(&hex[4..6], 16),
            ) {
                return Color::Rgb(r, g, b);
            }
        }
        Color::Reset
    }

    pub fn fg(&self, token: &str) -> Color {
        match token {
            "user" => Self::parse_color(&self.user),
            "assistant" => Self::parse_color(&self.assistant),
            "tool_call" => Self::parse_color(&self.tool_call),
            "tool_result" => Self::parse_color(&self.tool_result),
            "error" => Self::parse_color(&self.error),
            "success" => Self::parse_color(&self.success),
            "status" => Self::parse_color(&self.status),
            "border" => Self::parse_color(&self.border),
            "input_bg" => Self::parse_color(&self.input_bg),
            "conversation_bg" => Self::parse_color(&self.conversation_bg),
            "accent" => Self::parse_color(&self.accent),
            _ => Color::Reset,
        }
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path).map_err(|e| format!("Cannot read theme file: {e}"))?;
        serde_json::from_str(&content).map_err(|e| format!("Invalid theme JSON: {e}"))
    }

    #[allow(clippy::type_complexity)]
    pub fn all() -> Vec<(&'static str, fn() -> Self)> {
        vec![
            ("tokyonight", Self::tokyonight),
            ("catppuccin-macchiato", Self::catppuccin_macchiato),
            ("gruvbox-dark", Self::gruvbox_dark),
        ]
    }
}
