use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
};

use crate::theme::Theme;
use crate::tui::markdown::render_markdown;

pub fn render(
    f: &mut Frame,
    area: Rect,
    theme: &Theme,
    messages: &[super::ChatMessage],
    streaming_buffer: Option<&str>,
    scroll: usize,
    auto_scroll: bool,
) {
    let border_color = theme.fg("border");
    let bg = theme.fg("conversation_bg");

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title_style(Style::default().fg(theme.fg("status")));

    let max_w = area.width.saturating_sub(4) as usize;

    let mut lines: Vec<Line> = Vec::new();

    for msg in messages {
        let role = &msg.role;
        let content = &msg.content;
        let (prefix, style) = match role.as_str() {
            "user" => ("▼ You: ", theme.fg("user")),
            "assistant" => ("▶ SRR: ", theme.fg("assistant")),
            "tool" => ("⚡ ", theme.fg("tool_call")),
            "tool_result" => ("  ", theme.fg("tool_result")),
            "error" => ("✗ ", theme.fg("error")),
            "system" => ("◆ ", theme.fg("accent")),
            _ => ("", theme.fg("status")),
        };

        if role == "assistant" || role == "user" {
            // Use markdown rendering for longer responses
            if content.len() > 40 {
                let md_lines = render_markdown(content, theme);
                if let Some(first) = md_lines.first() {
                    let mut first_line = vec![Span::styled(prefix, Style::default().fg(style).bold())];
                    first_line.extend(first.spans.iter().cloned());
                    lines.push(Line::from(first_line));
                }
                for rest in md_lines.iter().skip(1) {
                    lines.push(rest.clone());
                }
            } else {
                let wrapped = textwrap::fill(content, max_w);
                for (i, line) in wrapped.lines().enumerate() {
                    if i == 0 {
                        lines.push(Line::from(vec![
                            Span::styled(prefix, Style::default().fg(style).bold()),
                            Span::styled(line.to_string(), Style::default().fg(style)),
                        ]));
                    } else {
                        lines.push(Line::from(Span::styled(
                            format!("  {line}"),
                            Style::default().fg(style),
                        )));
                    }
                }
            }
        } else {
            let wrapped = textwrap::fill(content, max_w);
            for (i, line) in wrapped.lines().enumerate() {
                if i == 0 {
                    lines.push(Line::from(vec![
                        Span::styled(prefix, Style::default().fg(style).bold()),
                        Span::styled(line.to_string(), Style::default().fg(style)),
                    ]));
                } else {
                    lines.push(Line::from(Span::styled(
                        format!("  {line}"),
                        Style::default().fg(style),
                    )));
                }
            }
        }
        lines.push(Line::from(""));
    }

    // Render streaming buffer if active
    if let Some(buffer) = streaming_buffer {
        if !buffer.is_empty() {
            let prefix = "▶ SRR: ";
            let style = theme.fg("assistant");
            let md_lines = render_markdown(buffer, theme);
            if let Some(first) = md_lines.first() {
                let mut first_line = vec![
                    Span::styled(prefix, Style::default().fg(style).bold()),
                ];
                first_line.extend(first.spans.iter().cloned());
                lines.push(Line::from(first_line));
            }
            for rest in md_lines.iter().skip(1) {
                lines.push(rest.clone());
            }
        }
    }

    // Add scroll indicator when scrolled up (insert BEFORE computing effective_scroll)
    if !auto_scroll {
        let hidden = lines.len().saturating_sub(scroll + area.height as usize);
        if hidden > 0 {
            lines.insert(scroll.min(lines.len()), Line::from(Span::styled(
                format!(" ↑ {} more messages", hidden),
                Style::default().fg(theme.fg("status")).add_modifier(Modifier::DIM),
            )));
        }
    }

    let effective_scroll = if auto_scroll {
        lines.len().saturating_sub(area.height as usize).saturating_sub(2)
    } else {
        scroll
    };

    let para = Paragraph::new(Text::from(lines))
        .block(block)
        .style(Style::default().bg(bg))
        .scroll((effective_scroll as u16, 0));
    f.render_widget(para, area);
}
