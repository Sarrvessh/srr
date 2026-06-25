use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::theme::Theme;

pub fn render(f: &mut Frame, area: Rect, theme: &Theme) {
    let border_color = theme.fg("border");
    let accent = theme.fg("accent");
    let dim = theme.fg("status");

    let cmds = crate::tui::commands::all().iter();

    let mut lines = vec![
        Line::from(Span::styled(" SRR TUI Help", Style::default().fg(accent).bold())),
        Line::from(""),
        Line::from(Span::styled(" Slash Commands:", Style::default().fg(accent).bold())),
        Line::from(""),
    ];

    for c in cmds {
        lines.push(Line::from(vec![
            Span::styled(format!("  {:<12}", c.name), Style::default().fg(accent)),
            Span::styled(c.desc, Style::default().fg(dim)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(" Key Bindings:", Style::default().fg(accent).bold())));
    lines.push(Line::from(""));

    let keys = crate::tui::keybind::default_bindings();
    for k in &keys {
        let key_str = format!("  {:20}", format!("{:?}", k.key));
        lines.push(Line::from(vec![
            Span::styled(key_str, Style::default().fg(dim)),
            Span::styled(k.desc, Style::default().fg(theme.fg("user"))),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(" Press Esc to close", Style::default().fg(dim).italic())));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(" Help ")
        .title_style(Style::default().fg(accent));

    let para = Paragraph::new(Text::from(lines))
        .block(block)
        .style(Style::default().bg(theme.fg("conversation_bg")))
        .wrap(Wrap { trim: false })
        .alignment(Alignment::Left);

    // Center the help in the area
    let help_area = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(2),
    };
    f.render_widget(para, help_area);
}
