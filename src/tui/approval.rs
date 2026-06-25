use serde_json::Value;

use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::theme::Theme;

pub struct PendingApproval {
    pub tool_name: String,
    pub args: Value,
    pub index: usize,
    pub total: usize,
}

pub fn render(f: &mut Frame, area: Rect, theme: &Theme, pending: &[PendingApproval], selected: usize) {
    let border_color = theme.fg("accent");
    let bg = theme.fg("conversation_bg");

    let mut lines = vec![
        Line::from(Span::styled(" ⚡ Tool Approval Required", Style::default().fg(theme.fg("accent")).bold())),
        Line::from(""),
    ];

    for (i, p) in pending.iter().enumerate() {
        let marker = if i == selected { " ▸ " } else { "   " };
        lines.push(Line::from(Span::styled(
            format!("{}{}  ({}/{})", marker, p.tool_name, i + 1, pending.len()),
            Style::default().fg(if i == selected { theme.fg("accent") } else { theme.fg("user") }),
        )));
        if let Some(path) = p.args.get("path").and_then(|v| v.as_str()) {
            lines.push(Line::from(Span::styled(
                format!("     file: {}", path),
                Style::default().fg(theme.fg("tool_result")),
            )));
        }
        if let Some(cmd) = p.args.get("command").and_then(|v| v.as_str()) {
            let truncated = if cmd.len() > 80 { format!("{}...", &cmd[..80]) } else { cmd.to_string() };
            lines.push(Line::from(Span::styled(
                format!("     command: {}", truncated),
                Style::default().fg(theme.fg("tool_result")),
            )));
        }
        lines.push(Line::from(""));
    }

    lines.push(Line::from(Span::styled(
        " [1] Approve    [2] Approve All    [3] Reject    [Esc] Cancel",
        Style::default().fg(theme.fg("status")),
    )));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(" Approval ")
        .title_style(Style::default().fg(theme.fg("accent")));

    let para = Paragraph::new(Text::from(lines))
        .block(block)
        .style(Style::default().bg(bg))
        .wrap(Wrap { trim: false })
        .alignment(Alignment::Left);

    let panel_area = Rect {
        x: area.x + 4,
        y: area.y + 2,
        width: area.width.saturating_sub(8),
        height: area.height.saturating_sub(4),
    };
    f.render_widget(para, panel_area);
}
