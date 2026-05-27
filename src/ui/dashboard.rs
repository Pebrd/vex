use crate::config::Project;
use crate::theme::Theme;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};

pub struct Dashboard {
    pub projects: Vec<Project>,
    pub selected: usize,
    list_state: ListState,
}

impl Dashboard {
    pub fn new(projects: Vec<Project>) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            projects,
            selected: 0,
            list_state,
        }
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if self.projects.is_empty() {
            let block = Block::default()
                .borders(Borders::ALL)
                .title(" vex — GitHub Issues TUI ")
                .style(Style::default().fg(theme.accent));
            let inner = block.inner(area);

            let welcome = Paragraph::new(vec![
                Line::from(Span::raw("")),
                Line::from(Span::raw("")),
                Line::from(Span::raw("")),
                Line::from(Span::styled(
                    "  No project selected",
                    Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::raw("")),
                Line::from(Span::raw("  Launch vex from a git repository or add one:")),
                Line::from(Span::raw("")),
                Line::from(vec![
                    Span::raw("    "),
                    Span::styled("a", Style::default().fg(theme.accent)),
                    Span::raw("  browse for a project directory"),
                ]),
                Line::from(vec![
                    Span::raw("    "),
                    Span::styled("q", Style::default().fg(theme.accent)),
                    Span::raw("  quit"),
                ]),
                Line::from(Span::raw("")),
                Line::from(Span::styled(
                    "  ─────────────────────────────────",
                    Style::default().fg(theme.text_dim),
                )),
                Line::from(Span::raw("")),
                Line::from(Span::styled(
                    "  Quick capture:",
                    Style::default().fg(theme.text_dim),
                )),
                Line::from(Span::raw(
                    "    vex add \"note title\" --body \"...\" --priority high",
                )),
            ])
            .wrap(Wrap { trim: false });
            frame.render_widget(block, area);
            frame.render_widget(welcome, inner);
            return;
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" Projects ({}) ", self.projects.len()))
            .style(Style::default().fg(theme.accent));
        let inner = block.inner(area);

        let items: Vec<ListItem> = self
            .projects
            .iter()
            .map(|p| {
                let exists = std::path::Path::new(&p.path).exists();
                let warning = if !exists {
                    Span::styled(
                        "⚠ ",
                        Style::default()
                            .fg(theme.danger)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::raw("  ")
                };
                let name_style = if !exists {
                    Style::default()
                        .fg(theme.danger)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                        .fg(theme.success)
                        .add_modifier(Modifier::BOLD)
                };
                let content = Line::from(vec![
                    warning,
                    Span::styled(&p.name, name_style),
                    Span::raw("  "),
                    Span::styled(&p.path, Style::default().fg(theme.text_dim)),
                    Span::raw("  "),
                    Span::styled(
                        format!("{}/{}", p.owner, p.repo),
                        Style::default().fg(theme.accent),
                    ),
                ]);
                ListItem::new(content)
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

        frame.render_widget(block, area);
        frame.render_stateful_widget(list, inner, &mut self.list_state);
    }
}
