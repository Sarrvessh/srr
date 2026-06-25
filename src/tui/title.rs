use ratatui::{
    Frame,
    layout::Rect,
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::theme::Theme;

pub fn render(f: &mut Frame, area: Rect, theme: &Theme, model: &str, mode: &str, context_pct: usize) {
    let accent = theme.fg("accent");
    let dim = theme.fg("status");
    let model_display = if model.is_empty() { "no model" } else { model };
    let ctx_str = format!("{}% ctx", context_pct);
    let ctx_color = if context_pct > 90 {
        theme.fg("error")
    } else if context_pct > 70 {
        theme.fg("accent")
    } else {
        dim
    };

    let mode_icon = match mode {
        "plan" => " \u{1f4cb} ",
        "yolo" => " \u{1f916} ",
        _ => " \u{2728} ",
    };

    let line = Line::from(vec![
        Span::styled(mode_icon, Style::default().fg(accent).bold()),
        Span::styled("SRR", Style::default().fg(accent).bold()),
        Span::styled("  ·  ", Style::default().fg(dim)),
        Span::styled(model_display, Style::default().fg(dim)),
        Span::styled("  ·  ", Style::default().fg(dim)),
        Span::styled(ctx_str, Style::default().fg(ctx_color)),
    ]);
    f.render_widget(Paragraph::new(line), area);
}
