use crate::config::Project;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState, Paragraph};
use ratatui::Frame;

pub struct Dashboard {
    pub projects: Vec<Project>,
    pub selected: usize,
    pub filter: String,
}

impl Dashboard {
    pub fn new(projects: Vec<Project>) -> Self {
        Self {
            projects,
            selected: 0,
            filter: String::new(),
        }
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(1),
        ])
            .split(area);

        let title = Paragraph::new(Line::from(Span::styled(
            " vex — projects ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
        frame.render_widget(title, layout[0]);

        let items: Vec<ListItem> = self
            .projects
            .iter()
            .map(|p| {
                let content = Line::from(vec![
                    Span::styled(
                        &p.name,
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  "),
                    Span::styled(&p.path, Style::default().fg(Color::DarkGray)),
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
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        frame.render_stateful_widget(list, layout[1], &mut list_state);


    }
}
