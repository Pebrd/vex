use crate::config::Project;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

pub struct Dashboard {
    pub projects: Vec<Project>,
    pub selected: usize,
}

impl Dashboard {
    pub fn new(projects: Vec<Project>) -> Self {
        Self { projects, selected: 0 }
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect) {
        if self.projects.is_empty() {
            let block = Block::default()
                .borders(Borders::ALL)
                .title(" vex — GitHub Issues TUI ")
                .style(Style::default().fg(Color::Cyan));
            let inner = block.inner(area);

            let welcome = Paragraph::new(vec![
                Line::from(Span::raw("")),
                Line::from(Span::raw("")),
                Line::from(Span::raw("")),
                Line::from(Span::styled("  No project selected", Style::default().fg(Color::White).add_modifier(Modifier::BOLD))),
                Line::from(Span::raw("")),
                Line::from(Span::raw("  Launch vex from a git repository or add one:")),
                Line::from(Span::raw("")),
                Line::from(vec![
                    Span::raw("    "),
                    Span::styled("a", Style::default().fg(Color::Cyan)),
                    Span::raw("  browse for a project directory"),
                ]),
                Line::from(vec![
                    Span::raw("    "),
                    Span::styled("q", Style::default().fg(Color::Cyan)),
                    Span::raw("  quit"),
                ]),
                Line::from(Span::raw("")),
                Line::from(Span::styled("  ─────────────────────────────────", Style::default().fg(Color::DarkGray))),
                Line::from(Span::raw("")),
                Line::from(Span::styled("  Quick capture:", Style::default().fg(Color::DarkGray))),
                Line::from(Span::raw("    vex add \"note title\" --body \"...\" --priority high")),
            ])
            .wrap(Wrap { trim: false });
            frame.render_widget(block, area);
            frame.render_widget(welcome, inner);
            return;
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" Projects ({}) ", self.projects.len()))
            .style(Style::default().fg(Color::Cyan));
        let inner = block.inner(area);

        let items: Vec<ListItem> = self
            .projects
            .iter()
            .map(|p| {
                let content = Line::from(vec![
                    Span::styled(
                        &p.name,
                        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  "),
                    Span::styled(
                        &p.path,
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::raw("  "),
                    Span::styled(
                        format!("{}/{}", p.owner, p.repo),
                        Style::default().fg(Color::Cyan),
                    ),
                ]);
                ListItem::new(content)
            })
            .collect();

        let mut list_state = ListState::default();
        list_state.select(Some(self.selected));

        let list = List::new(items)
            .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
            .highlight_symbol("> ");

        frame.render_widget(block, area);
        frame.render_stateful_widget(list, inner, &mut list_state);
    }
}
