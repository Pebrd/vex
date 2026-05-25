use crate::github::Issue;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;
use std::collections::BTreeMap;

pub struct RoadmapView {
    pub owner: String,
    pub repo: String,
    pub groups: Vec<(String, Vec<Issue>)>,
    pub selected_group: usize,
    pub selected_item: usize,
    list_states: Vec<ListState>,
}

impl RoadmapView {
    pub fn new(owner: &str, repo: &str) -> Self {
        Self {
            owner: owner.to_string(),
            repo: repo.to_string(),
            groups: Vec::new(),
            selected_group: 0,
            selected_item: 0,
            list_states: Vec::new(),
        }
    }

    pub fn update(&mut self, issues: &[Issue]) {
        let mut map: BTreeMap<String, Vec<Issue>> = BTreeMap::new();
        for issue in issues {
            if issue.state != "open" {
                continue;
            }
            if issue.labels.is_empty() {
                map.entry("no label".to_string()).or_default().push(issue.clone());
            } else {
                for label in &issue.labels {
                    map.entry(label.clone()).or_default().push(issue.clone());
                }
            }
        }
        self.groups = map.into_iter().collect();
        self.list_states = self.groups.iter().map(|_| {
            let mut s = ListState::default();
            s.select(Some(0));
            s
        }).collect();
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let title = Paragraph::new(Line::from(vec![
            Span::styled(
                format!(" {}/{} — Roadmap ", self.owner, self.repo),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
        ]));
        frame.render_widget(title, area);

        if self.groups.is_empty() {
            let empty = Paragraph::new("no open issues with labels")
                .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(empty, area);
            return;
        }

        let constraints: Vec<Constraint> = self
            .groups
            .iter()
            .map(|_| Constraint::Ratio(1, self.groups.len() as u32))
            .collect();

        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(constraints)
            .split(area);

        for (idx, (label, issues)) in self.groups.iter().enumerate() {
            let is_selected = idx == self.selected_group;
            let border_color = if is_selected { Color::Green } else { Color::DarkGray };
            let title_text = format!(" {label} ({})", issues.len());

            let block = Block::default()
                .borders(Borders::ALL)
                .title(title_text)
                .border_style(Style::default().fg(border_color));

            let items: Vec<ListItem> = issues.iter().map(|issue| {
                ListItem::new(Line::from(Span::styled(
                    format!("#{} {}", issue.number, issue.title),
                    Style::default().fg(Color::White),
                )))
            }).collect();

            let list = List::new(items)
                .block(block)
                .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD));

            if let Some(state) = self.list_states.get_mut(idx) {
                if is_selected {
                    state.select(Some(self.selected_item.min(issues.len().saturating_sub(1))));
                }
                frame.render_stateful_widget(list, columns[idx], state);
            }
        }
    }
}
