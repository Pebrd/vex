use crate::notes::Note;
use crate::theme::Theme;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};

pub struct NotesView {
    pub notes: Vec<Note>,
    pub selected: usize,
    list_state: ListState,
}

impl NotesView {
    pub fn new(notes: Vec<Note>) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            notes,
            selected: 0,
            list_state,
        }
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect, detail: Option<&Note>, theme: &Theme) {
        let layout = if detail.is_some() {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Ratio(2, 5), Constraint::Ratio(3, 5)])
                .split(area)
        } else {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(1), Constraint::Length(0)])
                .split(area)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Local Notes ")
            .style(Style::default().fg(theme.accent));
        let inner = block.inner(layout[0]);
        frame.render_widget(block, layout[0]);

        let items: Vec<ListItem> = self
            .notes
            .iter()
            .map(|n| {
                let priority_style = match n.priority.as_str() {
                    "high" => Style::default().fg(theme.danger),
                    "medium" => Style::default().fg(theme.warning),
                    _ => Style::default().fg(theme.accent),
                };
                let status_icon = if n.status == "open" { " " } else { " " };

                let mut spans = vec![
                    Span::styled(status_icon, priority_style),
                    Span::styled(&n.title, Style::default().fg(theme.text)),
                    Span::raw(" "),
                    Span::styled(
                        match n.priority.as_str() {
                            "high" => " ↑",
                            "medium" => " –",
                            _ => " ↓",
                        },
                        priority_style,
                    ),
                ];

                if let Some(num) = n.issue {
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled(
                        format!("#{num}"),
                        Style::default().fg(theme.accent),
                    ));
                }

                ListItem::new(Line::from(spans))
            })
            .collect();

        self.list_state.select(Some(self.selected));

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(theme.selection)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        frame.render_stateful_widget(list, inner, &mut self.list_state);

        if let Some(note) = detail {
            self.draw_detail(frame, layout[1], note, theme);
        }
    }

    fn draw_detail(&mut self, frame: &mut Frame, area: Rect, note: &Note, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" Note: {} ", note.title))
            .style(Style::default().fg(theme.accent));

        let mut lines = vec![
            Line::from(vec![Span::styled(
                &note.title,
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled(
                    &note.status,
                    Style::default().fg(match note.status.as_str() {
                        "open" => theme.success,
                        _ => theme.danger,
                    }),
                ),
                Span::raw(" | priority: "),
                Span::styled(
                    &note.priority,
                    Style::default().fg(match note.priority.as_str() {
                        "high" => theme.danger,
                        "medium" => theme.warning,
                        _ => theme.accent,
                    }),
                ),
                Span::raw(" | "),
                Span::styled(&note.created_at, Style::default().fg(theme.text_dim)),
            ]),
            Line::from(Span::raw("")),
        ];

        if let Some(num) = note.issue {
            lines.push(Line::from(vec![
                Span::styled("Linked to issue: ", Style::default().fg(theme.text_dim)),
                Span::styled(format!("#{num}"), Style::default().fg(theme.accent)),
            ]));
            lines.push(Line::from(Span::raw("")));
        }

        if let Some(body) = &note.body {
            for line in body.lines() {
                lines.push(Line::from(Span::raw(line)));
            }
        }

        let detail = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false });
        frame.render_widget(detail, area);
    }
}
