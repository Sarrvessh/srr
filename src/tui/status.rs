use chrono::Local;

use ratatui::{
    Frame,
    layout::Rect,
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::theme::Theme;

pub fn render(f: &mut Frame, area: Rect, theme: &Theme, mode: &str, model: &str, status: &str, total_tokens: usize) {
    let dim = theme.fg("status");
    let now = Local::now();
    let time_str = now.format("%H:%M").to_string();
    let badge = match mode {
        "plan" => " \u{1f4cb} Plan ",
        "yolo" => " \u{1f916} YOLO ",
        _ => " \u{2728} Agent ",
    };
    let badge_color = match mode {
        "plan" => theme.fg("tool_call"),
        "yolo" => theme.fg("accent"),
        _ => theme.fg("success"),
    };
    let model_display = if model.is_empty() { "no model" } else { model };

    let status_color = match status {
        s if s.contains("Error") || s.contains("error") => theme.fg("error"),
        s if s.contains("Streaming") => theme.fg("accent"),
        s if s.contains("Ready") => theme.fg("success"),
        s if s.contains("Approve") => theme.fg("tool_call"),
        _ => dim,
    };

    let tokens_str = format!("{}t", total_tokens);

    let line = Line::from(vec![
        Span::styled(badge, Style::default().fg(badge_color).bold().bg(dim)),
        Span::styled("  ", Style::default().fg(dim)),
        Span::styled(model_display, Style::default().fg(dim)),
        Span::styled("  |  ", Style::default().fg(dim)),
        Span::styled(status, Style::default().fg(status_color)),
        Span::styled("  \u{2022}  ", Style::default().fg(dim)),
        Span::styled(tokens_str, Style::default().fg(dim)),
        Span::styled("  \u{2022}  ", Style::default().fg(dim)),
        Span::styled(time_str, Style::default().fg(dim)),
    ]);
    f.render_widget(Paragraph::new(line), area);
}
