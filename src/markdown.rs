use crate::theme::Theme;
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use ratatui::prelude::*;

pub fn render_markdown(text: &str, theme: &Theme) -> Vec<Line<'static>> {
    let parser = Parser::new(text);
    let mut lines = Vec::new();
    let mut current_line: Vec<Span<'static>> = Vec::new();
    let mut in_bold = false;
    let mut in_italic = false;
    let mut in_code_block = false;
    let mut code_block_lines: Vec<String> = Vec::new();
    let flush_line = |lines: &mut Vec<Line<'static>>, spans: &mut Vec<Span<'static>>| {
        if !spans.is_empty() {
            lines.push(Line::from(std::mem::take(spans)));
        } else {
            lines.push(Line::from(""));
        }
    };

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => {}
                Tag::Heading { level: _, .. } => {
                    if !lines.is_empty() {
                        lines.push(Line::from(""));
                    }
                }
                Tag::CodeBlock(_) => {
                    in_code_block = true;
                    code_block_lines.clear();
                }
                Tag::List(_) => {
                    lines.push(Line::from(""));
                }
                Tag::Item => {
                    if !current_line.is_empty() {
                        flush_line(&mut lines, &mut current_line);
                    }
                    current_line.push(Span::styled(
                        format!("  {} ", "•"),
                        Style::default().fg(theme.accent),
                    ));
                }
                Tag::BlockQuote(_) => {
                    lines.push(Line::from(""));
                }
                Tag::Table(_) => {}
                Tag::TableHead => {}
                Tag::TableRow => {}
                Tag::TableCell => {}
                Tag::Strikethrough => {}
                Tag::Emphasis => {
                    in_italic = true;
                }
                Tag::Strong => {
                    in_bold = true;
                }
                Tag::Link {
                    link_type: _,
                    dest_url,
                    title: _,
                    id: _,
                } => {
                    current_line.push(Span::styled(
                        dest_url.to_string(),
                        Style::default().fg(theme.accent).underlined(),
                    ));
                }
                Tag::Image { .. } => {
                    current_line.push(Span::styled(
                        "[image]",
                        Style::default().fg(theme.text_dim).italic(),
                    ));
                }
                _ => {}
            },
            Event::End(tag_end) => match tag_end {
                TagEnd::Paragraph => {
                    flush_line(&mut lines, &mut current_line);
                }
                TagEnd::Heading { .. } => {
                    flush_line(&mut lines, &mut current_line);
                    lines.push(Line::from(""));
                }
                TagEnd::CodeBlock => {
                    if in_code_block {
                        in_code_block = false;
                        lines.push(Line::from(Span::styled(
                            code_block_lines.join("\n"),
                            Style::default().fg(theme.warning).bg(theme.surface),
                        )));
                        code_block_lines.clear();
                        lines.push(Line::from(""));
                    }
                }
                TagEnd::List(_) => {
                    lines.push(Line::from(""));
                }
                TagEnd::Item => {
                    flush_line(&mut lines, &mut current_line);
                }
                TagEnd::BlockQuote(_) => {}
                TagEnd::Table => {}
                TagEnd::TableHead => {}
                TagEnd::TableRow => {}
                TagEnd::TableCell => {}
                TagEnd::Strikethrough => {}
                TagEnd::Emphasis => {
                    in_italic = false;
                }
                TagEnd::Strong => {
                    in_bold = false;
                }
                TagEnd::Link => {}
                TagEnd::Image => {}
                _ => {}
            },
            Event::Text(t) => {
                let text = t.to_string();
                if in_code_block {
                    code_block_lines.push(text);
                } else {
                    let mut s = Style::default().fg(theme.text);
                    if in_bold {
                        s = s.add_modifier(Modifier::BOLD);
                    }
                    if in_italic {
                        s = s.add_modifier(Modifier::ITALIC);
                    }
                    current_line.push(Span::styled(text, s));
                }
            }
            Event::Code(t) => {
                let text = t.to_string();
                current_line.push(Span::styled(
                    text,
                    Style::default().fg(theme.accent).bg(theme.surface),
                ));
            }
            Event::SoftBreak | Event::HardBreak => {
                if !in_code_block {
                    flush_line(&mut lines, &mut current_line);
                } else {
                    code_block_lines.push("\n".to_string());
                }
            }
            Event::Rule => {
                lines.push(Line::from(Span::styled(
                    "─".repeat(50),
                    Style::default().fg(theme.border),
                )));
                lines.push(Line::from(""));
            }
            Event::InlineHtml(html) => {
                current_line.push(Span::raw(html.to_string()));
            }
            Event::InlineMath(math) => {
                current_line.push(Span::styled(
                    format!("${}$", math),
                    Style::default().fg(theme.text_dim).italic(),
                ));
            }
            Event::DisplayMath(math) => {
                lines.push(Line::from(Span::styled(
                    format!("  {}", math),
                    Style::default().fg(theme.text_dim),
                )));
            }
            Event::Html(html) => {
                current_line.push(Span::raw(html.to_string()));
            }
            Event::TaskListMarker(checked) => {
                let marker = if checked { "[x]" } else { "[ ]" };
                current_line.push(Span::styled(marker, Style::default().fg(theme.accent)));
            }
            Event::FootnoteReference(footnote) => {
                current_line.push(Span::styled(
                    format!("[^{}]", footnote),
                    Style::default().fg(theme.text_dim),
                ));
            }
            _ => {}
        }
    }

    if !current_line.is_empty() {
        flush_line(&mut lines, &mut current_line);
    }

    lines
}
