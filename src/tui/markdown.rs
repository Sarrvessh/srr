use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::theme::Theme;

#[derive(Clone, Copy)]
enum StyleMod {
    Bold,
    Italic,
    Dim,
    Strikethrough,
}

fn apply_style(base: Style, stack: &[StyleMod]) -> Style {
    let mut s = base;
    for m in stack {
        s = match m {
            StyleMod::Bold => s.add_modifier(Modifier::BOLD),
            StyleMod::Italic => s.add_modifier(Modifier::ITALIC),
            StyleMod::Dim => s.add_modifier(Modifier::DIM),
            StyleMod::Strikethrough => s.add_modifier(Modifier::CROSSED_OUT),
        };
    }
    s
}

pub fn render_markdown(text: &str, theme: &Theme) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_line: Vec<Span<'static>> = Vec::new();
    let mut in_code_block = false;
    let mut in_block_quote = false;
    let mut in_list_item = false;
    let mut list_item_prefix = String::new();
    let mut style_stack: Vec<StyleMod> = Vec::new();

    let parser = Parser::new(text);

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::CodeBlock(_) => in_code_block = true,
                Tag::Emphasis => style_stack.push(StyleMod::Italic),
                Tag::Strong => style_stack.push(StyleMod::Bold),
                Tag::Strikethrough => style_stack.push(StyleMod::Strikethrough),
                Tag::Link { .. } => style_stack.push(StyleMod::Dim),
                Tag::BlockQuote(_) => in_block_quote = true,
                Tag::List(_) => {
                    in_list_item = false;
                }
                Tag::Item => {
                    in_list_item = true;
                    list_item_prefix = "  • ".to_string();
                }
                Tag::Heading { .. } => style_stack.push(StyleMod::Bold),
                _ => {}
            },
            Event::End(end) => match end {
                TagEnd::CodeBlock => {
                    in_code_block = false;
                    lines.push(Line::from(""));
                }
                TagEnd::Emphasis => { style_stack.retain(|m| !matches!(m, StyleMod::Italic)); }
                TagEnd::Strong => { style_stack.retain(|m| !matches!(m, StyleMod::Bold)); }
                TagEnd::Strikethrough => { style_stack.retain(|m| !matches!(m, StyleMod::Strikethrough)); }
                TagEnd::Link => { style_stack.retain(|m| !matches!(m, StyleMod::Dim)); }
                TagEnd::Heading { .. } => {
                    style_stack.retain(|m| !matches!(m, StyleMod::Bold));
                    if !current_line.is_empty() {
                        lines.push(Line::from(std::mem::take(&mut current_line)));
                    }
                    lines.push(Line::from(""));
                }
                TagEnd::BlockQuote(_) => in_block_quote = false,
                TagEnd::List(_) => in_list_item = false,
                TagEnd::Item => {
                    in_list_item = false;
                    if !current_line.is_empty() {
                        lines.push(Line::from(std::mem::take(&mut current_line)));
                    }
                }
                TagEnd::Paragraph => {
                    if !current_line.is_empty() {
                        lines.push(Line::from(std::mem::take(&mut current_line)));
                    }
                    lines.push(Line::from(""));
                }
                _ => {}
            },
            Event::Text(t) => {
                let text = t.into_string();
                if in_code_block {
                    for line in text.lines() {
                        lines.push(Line::from(Span::styled(
                            format!("  {}", line),
                            Style::default().fg(theme.fg("tool_result")).bg(theme.fg("input_bg")),
                        )));
                    }
                    continue;
                }
                let mut base_style = Style::default().fg(theme.fg("assistant"));
                if in_block_quote {
                    base_style = base_style.add_modifier(Modifier::DIM);
                }
                let style = apply_style(base_style, &style_stack);
                if in_list_item && !list_item_prefix.is_empty() {
                    let prefix = std::mem::take(&mut list_item_prefix);
                    current_line.push(Span::styled(prefix, style));
                }
                if in_block_quote {
                    for line in text.lines() {
                        if !current_line.is_empty() {
                            lines.push(Line::from(std::mem::take(&mut current_line)));
                        }
                        current_line.push(Span::styled(format!("│ {}", line), base_style));
                    }
                } else {
                    current_line.push(Span::styled(text, style));
                }
            }
            Event::Code(t) => {
                let text = t.into_string();
                let base = Style::default().fg(theme.fg("accent")).bg(theme.fg("border"));
                let style = apply_style(base, &style_stack);
                current_line.push(Span::styled(text, style));
            }
            Event::SoftBreak => {
                current_line.push(Span::raw(" "));
            }
            Event::HardBreak if !current_line.is_empty() => {
                lines.push(Line::from(std::mem::take(&mut current_line)));
            }
            Event::HardBreak => {}
            _ => {}
        }
    }

    if !current_line.is_empty() {
        lines.push(Line::from(std::mem::take(&mut current_line)));
    }
    lines
}