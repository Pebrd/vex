use crate::notes::Note;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

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

    pub fn draw(&mut self, frame: &mut Frame, area: Rect, detail: Option<&Note>) {
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
            .style(Style::default().fg(Color::Cyan));
        let inner = block.inner(layout[0]);
        frame.render_widget(block, layout[0]);

        let items: Vec<ListItem> = self
            .notes
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
                    Span::styled(&n.title, Style::default().fg(Color::White)),
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
                        Style::default().fg(Color::Magenta),
                    ));
                }

                ListItem::new(Line::from(spans))
            })
            .collect();

        self.list_state.select(Some(self.selected));

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        frame.render_stateful_widget(list, inner, &mut self.list_state);

        if let Some(note) = detail {
            self.draw_detail(frame, layout[1], note);
        }
    }

    fn draw_detail(&mut self, frame: &mut Frame, area: Rect, note: &Note) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" Note: {} ", note.title))
            .style(Style::default().fg(Color::Cyan));

        let mut lines = vec![
            Line::from(vec![Span::styled(
                &note.title,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled(
                    &note.status,
                    Style::default().fg(match note.status.as_str() {
                        "open" => Color::Green,
                        _ => Color::Red,
                    }),
                ),
                Span::raw(" | priority: "),
                Span::styled(
                    &note.priority,
                    Style::default().fg(match note.priority.as_str() {
                        "high" => Color::Red,
                        "medium" => Color::Yellow,
                        _ => Color::Blue,
                    }),
                ),
                Span::raw(" | "),
                Span::styled(&note.created_at, Style::default().fg(Color::DarkGray)),
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

        if let Some(body) = &note.body {
            for line in body.lines() {
                lines.push(Line::from(Span::raw(line)));
            }
        }

        let detail = Paragraph::new(lines).block(block);
        frame.render_widget(detail, area);
    }
}
