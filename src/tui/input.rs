use ratatui::{
    Frame,
    layout::Rect,
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
};

use crate::theme::Theme;
use crate::tui::autocomplete::Autocomplete;

pub fn render(
    f: &mut Frame,
    area: Rect,
    theme: &Theme,
    input: &str,
    cursor: usize,
    autocomplete: &Autocomplete,
) {
    let border_color = theme.fg("border");
    let input_bg = theme.fg("input_bg");

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(" Input ")
        .title_style(Style::default().fg(theme.fg("status")));

    // Show placeholder when empty
    if input.is_empty() && !autocomplete.active {
        let para = Paragraph::new(Text::from(vec![Line::from(Span::styled(
            " Ask anything... (@ to mention files, / for commands)",
            Style::default().dark_gray().italic(),
        ))]))
        .block(block)
        .style(Style::default().bg(input_bg));
        f.render_widget(para, area);
        return;
    }

    // Split input into lines (for multi-line rendering)
    let lines: Vec<&str> = input.split('\n').collect();
    let mut visual_cursor = cursor;
    let mut line_idx = 0usize;
    let mut col_idx = 0usize;

    // Find cursor position in terms of line/column
    for (i, line) in lines.iter().enumerate() {
        let line_len = line.len() + 1; // +1 for the newline
        if visual_cursor <= line_len.saturating_sub(1) {
            line_idx = i;
            col_idx = visual_cursor;
            break;
        }
        if i < lines.len().saturating_sub(1) {
            visual_cursor = visual_cursor.saturating_sub(line_len);
        } else {
            line_idx = i;
            col_idx = visual_cursor.min(line.len());
        }
    }

    let mut display_lines: Vec<Line> = Vec::new();

    for (i, line_text) in lines.iter().enumerate() {
        if i == line_idx {
            // Show cursor on this line
            let col = col_idx.min(line_text.len());
            let before = &line_text[..col];
            let after = &line_text[col..];
            let cursor_char = after.chars().next();
            let after_rest = if let Some(c) = cursor_char {
                &after[c.len_utf8()..]
            } else {
                ""
            };

            let mut spans = vec![Span::raw(before.to_string())];
            match cursor_char {
                Some(c) => {
                    spans.push(Span::styled(
                        c.to_string(),
                        Style::default().bg(theme.fg("status")).fg(theme.fg("input_bg")),
                    ));
                }
                None => {
                    spans.push(Span::styled(
                        " ",
                        Style::default().bg(theme.fg("status")).fg(theme.fg("input_bg")),
                    ));
                }
            }
            spans.push(Span::raw(after_rest.to_string()));
            display_lines.push(Line::from(spans));
        } else {
            display_lines.push(Line::from(Span::raw(line_text.to_string())));
        }
    }

    // If autocomplete is active, show popup results
    if autocomplete.active {
        let max_results = (area.height as usize).saturating_sub(2).min(8);
        for (i, result) in autocomplete.results.iter().enumerate().take(max_results) {
            if i == autocomplete.selected {
                display_lines.push(Line::from(Span::styled(
                    format!(" ▸ {}", result),
                    Style::default().fg(theme.fg("input_bg")).bg(theme.fg("accent")),
                )));
            } else {
                display_lines.push(Line::from(Span::styled(
                    format!("   {result}"),
                    Style::default().fg(theme.fg("user")),
                )));
            }
        }
    }

    let para = Paragraph::new(Text::from(display_lines))
        .block(block)
        .style(Style::default().bg(input_bg));
    f.render_widget(para, area);
}
