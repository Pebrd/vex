use crate::github::Issue;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

pub struct IssuesView {
    pub issues: Vec<Issue>,
    pub selected: usize,
    pub filter_state: Option<String>,
    pub filter_label: Option<String>,
}

impl IssuesView {
    pub fn new(issues: Vec<Issue>) -> Self {
        Self {
            issues,
            selected: 0,
            filter_state: None,
            filter_label: None,
        }
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect, detail_issue: Option<&Issue>) {
        let layout = if detail_issue.is_some() {
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
            .title(" Issues ")
            .style(Style::default().fg(Color::Cyan));
        let inner = block.inner(layout[0]);

        frame.render_widget(block, layout[0]);

        let items: Vec<ListItem> = self
            .issues
            .iter()
            .map(|i| {
                let state_style = match i.state.as_str() {
                    "open" => Style::default().fg(Color::Green),
                    "closed" => Style::default().fg(Color::Red),
                    _ => Style::default().fg(Color::Yellow),
                };

                let mut spans = vec![
                    Span::styled(
                        format!(" #{} ", i.number),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(
                        if i.state == "open" { " " } else { " " },
                        state_style,
                    ),
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

        let mut list_state = ListState::default();
        list_state.select(Some(self.selected));

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        frame.render_stateful_widget(list, inner, &mut list_state);

        if let Some(issue) = detail_issue {
            self.draw_detail(frame, layout[1], issue);
        }
    }

    fn draw_detail(&self, frame: &mut Frame, area: Rect, issue: &Issue) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" Issue #{} ", issue.number))
            .style(Style::default().fg(Color::Cyan));

        let mut lines = vec![
            Line::from(vec![
                Span::styled(
                    &issue.title,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    &issue.state,
                    Style::default().fg(match issue.state.as_str() {
                        "open" => Color::Green,
                        _ => Color::Red,
                    }),
                ),
                Span::raw(" by "),
                Span::styled(
                    issue.author.as_deref().unwrap_or("unknown"),
                    Style::default().fg(Color::Cyan),
                ),
            ]),
            Line::from(Span::raw("")),
        ];

        if let Some(body) = &issue.body {
            for line in body.lines() {
                lines.push(Line::from(Span::raw(line)));
            }
        }

        let detail = Paragraph::new(lines).block(block);
        frame.render_widget(detail, area);
    }
}
