use crate::github::{Comment, Issue};
use crate::notes::Note;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};

pub struct IssuesView {
    pub issues: Vec<Issue>,
    pub selected: usize,
    #[allow(dead_code)]
    pub filter_state: Option<String>,
    #[allow(dead_code)]
    pub filter_label: Option<String>,
    list_state: ListState,
    notes_list_state: ListState,
}

#[derive(PartialEq, Clone, Copy)]
pub enum FocusTarget {
    Issues,
    Notes,
}

#[derive(Clone)]
pub struct EditState {
    pub title: String,
    pub body: String,
    pub field_focus: u8,
    pub issue_number: u64,
    pub note_slug: Option<String>,
    pub labels: Vec<String>,
    pub available_labels: Vec<String>,
    pub label_idx: usize,
}

impl IssuesView {
    pub fn new(issues: Vec<Issue>) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        let mut notes_list_state = ListState::default();
        notes_list_state.select(Some(0));
        Self {
            issues,
            selected: 0,
            filter_state: None,
            filter_label: None,
            list_state,
            notes_list_state,
        }
    }

    pub fn draw(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        detail_issue: Option<&Issue>,
        comments: Option<&[Comment]>,
        notes: &[Note],
        note_selected: usize,
        focus: FocusTarget,
        editing: Option<&EditState>,
        detail_scroll: u16,
    ) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(2, 5), Constraint::Ratio(3, 5)])
            .split(area);

        let left_panels = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
            .split(layout[0]);

        self.draw_issues_list(frame, left_panels[0], focus == FocusTarget::Issues);
        self.draw_notes_list(
            frame,
            left_panels[1],
            notes,
            note_selected,
            focus == FocusTarget::Notes,
        );

        if let Some(editing) = editing {
            self.draw_detail_editing(frame, layout[1], editing);
        } else if let Some(issue) = detail_issue {
            self.draw_detail(
                frame,
                layout[1],
                issue,
                comments.unwrap_or(&[]),
                detail_scroll,
            );
        } else if focus == FocusTarget::Notes {
            if let Some(note) = notes.get(note_selected) {
                self.draw_note_detail(frame, layout[1], note, detail_scroll);
            }
        }
    }

    fn draw_issues_list(&mut self, frame: &mut Frame, area: Rect, is_active: bool) {
        let border_style = if is_active {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Issues ")
            .style(border_style);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let items: Vec<ListItem> = self
            .issues
            .iter()
            .map(|i| {
                let state_style = match i.state.as_str() {
                    "open" => Style::default().fg(Color::Green),
                    "closed" => Style::default().fg(Color::Red),
                    _ => Style::default().fg(Color::Yellow),
                };

                let state_tag = format!(" {} ", if i.state == "open" { "OPEN" } else { "CLOSED" });
                let mut spans = vec![
                    Span::styled(state_tag, state_style.add_modifier(Modifier::REVERSED)),
                    Span::styled(format!(" #{} ", i.number), Style::default().fg(Color::Cyan)),
                    Span::raw(&i.title),
                ];

                if !i.labels.is_empty() {
                    spans.push(Span::raw(" "));
                    for label in &i.labels {
                        spans.push(Span::styled(
                            format!("[{}]", label),
                            Style::default().fg(Color::Magenta),
                        ));
                    }
                }

                ListItem::new(Line::from(spans))
            })
            .collect();

        self.list_state.select(Some(self.selected));

        let highlight = if is_active {
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let list = List::new(items)
            .highlight_style(highlight)
            .highlight_symbol(if is_active { "> " } else { "  " });

        frame.render_stateful_widget(list, inner, &mut self.list_state);
    }

    fn draw_notes_list(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        notes: &[Note],
        selected: usize,
        is_active: bool,
    ) {
        let border_style = if is_active {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Notes ")
            .style(border_style);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let items: Vec<ListItem> = notes
            .iter()
            .map(|n| {
                let priority_style = match n.priority.as_str() {
                    "high" => Style::default().fg(Color::Red),
                    "medium" => Style::default().fg(Color::Yellow),
                    _ => Style::default().fg(Color::Blue),
                };
                let status_icon = if n.status == "open" { " " } else { " " };

                let mut spans = vec![
                    Span::styled(status_icon, priority_style),
                    Span::raw(&n.title),
                    Span::raw(" "),
                    Span::styled(
                        match n.priority.as_str() {
                            "high" => "↑",
                            "medium" => "–",
                            _ => "↓",
                        },
                        priority_style,
                    ),
                ];

                if let Some(num) = n.issue {
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled(
                        format!("#{num}"),
                        Style::default().fg(Color::Magenta),
                    ));
                }

                ListItem::new(Line::from(spans))
            })
            .collect();

        self.notes_list_state.select(Some(selected));

        let highlight = if is_active {
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let list = List::new(items)
            .highlight_style(highlight)
            .highlight_symbol(if is_active { "> " } else { "  " });

        frame.render_stateful_widget(list, inner, &mut self.notes_list_state);
    }

    fn draw_detail_editing(&self, frame: &mut Frame, area: Rect, editing: &EditState) {
        let is_note = editing.note_slug.is_some();
        let label_count = editing.labels.len();
        let title_text = if is_note {
            " Editing Note ".to_string()
        } else if editing.issue_number == 0 {
            if label_count > 0 {
                format!(
                    " Creating Issue ({} label{}) ",
                    label_count,
                    if label_count == 1 { "" } else { "s" }
                )
            } else {
                " Creating Issue ".to_string()
            }
        } else if label_count > 0 {
            format!(
                " Editing Issue #{} ({} label{}) ",
                editing.issue_number,
                label_count,
                if label_count == 1 { "" } else { "s" }
            )
        } else {
            format!(" Editing Issue #{} ", editing.issue_number)
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .title(title_text)
            .style(Style::default().fg(Color::Yellow));
        let inner = block.inner(area);

        let has_label_tags = !editing.available_labels.is_empty()
            && !editing.labels.is_empty()
            && editing.field_focus != 2;
        let mut constraints = vec![Constraint::Length(3), Constraint::Min(1)];
        if has_label_tags {
            constraints.push(Constraint::Length(1));
        }
        constraints.push(Constraint::Length(1));

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner);

        let title_style = if editing.field_focus == 0 {
            Style::default().bg(Color::DarkGray).fg(Color::White)
        } else {
            Style::default().fg(Color::White)
        };
        let title_ptr = if editing.field_focus == 0 {
            "▶ "
        } else {
            "  "
        };
        let title_block = Block::default()
            .title(format!("{title_ptr}Title"))
            .borders(Borders::ALL)
            .style(title_style);
        let mut title_spans: Vec<Span> = vec![Span::raw(&editing.title)];
        if editing.field_focus == 0 {
            title_spans.push(Span::styled("|", Style::default().fg(Color::Cyan)));
        }
        let title_text = Paragraph::new(Line::from(title_spans)).block(title_block);
        frame.render_widget(title_text, chunks[0]);

        if editing.field_focus == 2 {
            let label_style = Style::default().bg(Color::DarkGray).fg(Color::White);
            let label_ptr = "▶ ";
            let title = format!(
                "{label_ptr}Labels ({}/{} selected)",
                editing.labels.len(),
                editing.available_labels.len()
            );
            let label_block = Block::default()
                .title(title)
                .borders(Borders::ALL)
                .style(label_style);
            let label_inner = label_block.inner(chunks[1]);
            frame.render_widget(label_block, chunks[1]);

            let items: Vec<ListItem> = editing
                .available_labels
                .iter()
                .enumerate()
                .map(|(idx, name)| {
                    let checked = if editing.labels.contains(name) {
                        "✓ "
                    } else {
                        "  "
                    };
                    let is_sel = idx == editing.label_idx;
                    let style = if is_sel {
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    ListItem::new(Line::from(vec![
                        Span::styled(checked, Style::default().fg(Color::Green)),
                        Span::styled(name, style),
                    ]))
                })
                .collect();

            let mut list_state = ListState::default();
            list_state.select(Some(editing.label_idx));
            let list = List::new(items)
                .highlight_style(Style::default().bg(Color::DarkGray))
                .highlight_symbol("> ");
            frame.render_stateful_widget(list, label_inner, &mut list_state);
        } else {
            let body_style = if editing.field_focus == 1 {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else {
                Style::default().fg(Color::White)
            };
            let body_ptr = if editing.field_focus == 1 {
                "▶ "
            } else {
                "  "
            };
            let body_block = Block::default()
                .title(format!("{body_ptr}Body"))
                .borders(Borders::ALL)
                .style(body_style);
            let mut body_lines: Vec<Line> = editing
                .body
                .lines()
                .map(|l| Line::from(Span::raw(l)))
                .collect();
            if editing.field_focus == 1 {
                if let Some(last) = body_lines.last_mut() {
                    last.push_span(Span::styled("|", Style::default().fg(Color::Cyan)));
                } else {
                    body_lines.push(Line::from(Span::styled(
                        "|",
                        Style::default().fg(Color::Cyan),
                    )));
                }
            }
            let body_text = Paragraph::new(Text::from(body_lines))
                .block(body_block)
                .wrap(Wrap { trim: false });
            frame.render_widget(body_text, chunks[1]);
        }

        let mut help_idx = 2;
        if has_label_tags {
            help_idx = 3;
            let tags: Vec<Span> = editing
                .labels
                .iter()
                .flat_map(|l| {
                    vec![
                        Span::styled(
                            format!(" {l} "),
                            Style::default().fg(Color::Black).bg(Color::Cyan),
                        ),
                        Span::raw(" "),
                    ]
                })
                .collect();
            let tags_line =
                Paragraph::new(Line::from(tags)).block(Block::default().borders(Borders::NONE));
            frame.render_widget(tags_line, chunks[2]);
        }

        let help = Paragraph::new(Line::from(vec![
            Span::styled("Tab", Style::default().fg(Color::Cyan)),
            Span::raw(" switch  "),
            Span::styled("Space", Style::default().fg(Color::Cyan)),
            Span::raw(" toggle label  "),
            Span::styled("Ctrl+S", Style::default().fg(Color::Cyan)),
            Span::raw(" save  "),
            Span::styled("Esc", Style::default().fg(Color::Cyan)),
            Span::raw(" cancel"),
        ]));
        frame.render_widget(help, chunks[help_idx]);

        frame.render_widget(block, area);
    }

    fn draw_detail(
        &self,
        frame: &mut Frame,
        area: Rect,
        issue: &Issue,
        comments: &[Comment],
        scroll: u16,
    ) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" Issue #{} ", issue.number))
            .style(Style::default().fg(Color::Cyan));

        let mut lines = vec![
            Line::from(vec![Span::styled(
                &issue.title,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from({
                let state_color = match issue.state.as_str() {
                    "open" => Color::Green,
                    _ => Color::Red,
                };
                let state_style = Style::default().fg(state_color);
                let mut spans = vec![
                    Span::styled("●", state_style),
                    Span::styled(
                        format!(" {} ", issue.state.to_uppercase()),
                        state_style.add_modifier(Modifier::BOLD),
                    ),
                ];
                for label in &issue.labels {
                    spans.push(Span::styled(
                        format!("[{}]", label),
                        Style::default().fg(Color::Magenta),
                    ));
                    spans.push(Span::raw(" "));
                }
                spans.push(Span::raw("by "));
                spans.push(Span::styled(
                    issue.author.as_deref().unwrap_or("unknown"),
                    Style::default().fg(Color::Cyan),
                ));
                spans
            }),
            Line::from(Span::raw("")),
        ];

        if let Some(body) = &issue.body {
            for line in body.lines() {
                lines.push(Line::from(Span::raw(line)));
            }
        }

        if !comments.is_empty() {
            lines.push(Line::from(Span::raw("")));
            lines.push(Line::from(Span::styled(
                format!("─── {} comments ───", comments.len()),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(Span::raw("")));

            for comment in comments {
                lines.push(Line::from(vec![
                    Span::styled(
                        comment.author.as_deref().unwrap_or("unknown"),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("  {}", comment.created_at.as_deref().unwrap_or("")),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
                if let Some(body) = &comment.body {
                    for line in body.lines() {
                        lines.push(Line::from(Span::raw(format!("  {line}"))));
                    }
                }
                lines.push(Line::from(Span::raw("")));
            }
        }

        let detail = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0));
        frame.render_widget(detail, area);
    }

    fn draw_note_detail(&self, frame: &mut Frame, area: Rect, note: &Note, scroll: u16) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" Note: {} ", note.title))
            .style(Style::default().fg(Color::Cyan));

        let mut lines = vec![
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    &note.status,
                    Style::default().fg(match note.status.as_str() {
                        "open" => Color::Green,
                        _ => Color::Red,
                    }),
                ),
                Span::raw(" | "),
                Span::styled("Priority: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    &note.priority,
                    Style::default().fg(match note.priority.as_str() {
                        "high" => Color::Red,
                        "medium" => Color::Yellow,
                        _ => Color::Blue,
                    }),
                ),
            ]),
            Line::from(vec![
                Span::styled("Created: ", Style::default().fg(Color::DarkGray)),
                Span::styled(&note.created_at, Style::default().fg(Color::White)),
            ]),
            Line::from(Span::raw("")),
        ];

        if let Some(num) = note.issue {
            lines.push(Line::from(vec![
                Span::styled("Linked to issue: ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("#{num}"), Style::default().fg(Color::Magenta)),
            ]));
            lines.push(Line::from(Span::raw("")));
        }

        if let Some(ref body) = note.body {
            for line in body.lines() {
                lines.push(Line::from(Span::raw(line)));
            }
        }

        let detail = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0));
        frame.render_widget(detail, area);
    }
}
