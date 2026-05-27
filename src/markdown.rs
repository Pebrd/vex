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
    let mut in_task_item = false;
    let mut link_text: Vec<String> = Vec::new();
    let mut in_link = false;

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
                    if !in_task_item {
                        current_line.push(Span::styled(
                            format!("  {} ", "•"),
                            Style::default().fg(theme.accent),
                        ));
                    }
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
                    dest_url: _,
                    title: _,
                    id: _,
                } => {
                    in_link = true;
                    link_text.clear();
                }
                Tag::Image { .. } => {
                    current_line.push(Span::styled(
                        "[image]",
                        Style::default().fg(theme.text_dim).italic(),
                    ));
                }
                Tag::HtmlBlock => {}
                Tag::FootnoteDefinition(_) => {}
                Tag::DefinitionList => {}
                Tag::DefinitionListTitle => {}
                Tag::DefinitionListDefinition => {}
                Tag::Superscript => {}
                Tag::Subscript => {}
                Tag::MetadataBlock(_) => {}
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
                        for line_text in &code_block_lines {
                            lines.push(Line::from(Span::styled(
                                line_text.clone(),
                                Style::default().fg(theme.warning).bg(theme.surface),
                            )));
                        }
                        code_block_lines.clear();
                        lines.push(Line::from(""));
                    }
                }
                TagEnd::List(_) => {
                    lines.push(Line::from(""));
                }
                TagEnd::Item => {
                    flush_line(&mut lines, &mut current_line);
                    in_task_item = false;
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
                TagEnd::Link => {
                    if in_link {
                        let text = link_text.join("");
                        current_line.push(Span::styled(
                            text,
                            Style::default().fg(theme.accent).underlined(),
                        ));
                        in_link = false;
                    }
                }
                TagEnd::Image => {}
                TagEnd::HtmlBlock => {}
                TagEnd::FootnoteDefinition => {}
                TagEnd::DefinitionList => {}
                TagEnd::DefinitionListTitle => {}
                TagEnd::DefinitionListDefinition => {}
                TagEnd::Superscript => {}
                TagEnd::Subscript => {}
                TagEnd::MetadataBlock(_) => {}
            },
            Event::Text(t) => {
                let text = t.to_string();
                if in_link {
                    link_text.push(text);
                } else if in_code_block {
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
                in_task_item = true;
                let marker = if checked { "[x]" } else { "[ ]" };
                current_line.push(Span::styled(marker, Style::default().fg(theme.accent)));
            }
            Event::FootnoteReference(footnote) => {
                current_line.push(Span::styled(
                    format!("[^{}]", footnote),
                    Style::default().fg(theme.text_dim),
                ));
            }
        }
    }

    if !current_line.is_empty() {
        flush_line(&mut lines, &mut current_line);
    }

    lines
}
