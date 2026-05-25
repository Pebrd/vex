use crate::github::{Issue, PullRequest};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

pub struct StatsView {
    pub owner: String,
    pub repo: String,
    pub total_items: Vec<String>,
    pub selected: usize,
    list_state: ListState,
}

impl StatsView {
    pub fn new(owner: &str, repo: &str) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            owner: owner.to_string(),
            repo: repo.to_string(),
            total_items: Vec::new(),
            selected: 0,
            list_state,
        }
    }

    pub fn update(&mut self, issues: &[Issue], prs: &[PullRequest]) {
        let open_issues = issues.iter().filter(|i| i.state == "open").count();
        let closed_issues = issues.iter().filter(|i| i.state == "closed").count();
        let open_prs = prs.iter().filter(|p| p.state == "open").count();
        let closed_prs = prs.iter().filter(|p| p.state == "closed").count();
        let merged_prs = prs.iter().filter(|p| p.state == "merged" || p.state == "closed").count() - closed_prs;

        self.total_items = vec![
            format!("Issues (open): {open_issues}"),
            format!("Issues (closed): {closed_issues}"),
            format!("Issues (total): {}", issues.len()),
            String::new(),
            format!("PRs (open): {open_prs}"),
            format!("PRs (closed): {closed_prs}"),
            format!("PRs (merged): {merged_prs}"),
            format!("PRs (total): {}", prs.len()),
        ];
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let title = Paragraph::new(Line::from(vec![
            Span::styled(
                format!(" {}/{} — Statistics ", self.owner, self.repo),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
        ]));
        frame.render_widget(title, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan));
        let inner = block.inner(area);

        let items: Vec<ListItem> = self
            .total_items
            .iter()
            .map(|s| {
                if s.is_empty() {
                    ListItem::new(Line::from(""))
                } else {
                    let color = if s.contains("open") {
                        Color::Green
                    } else if s.contains("closed") || s.contains("merged") {
                        Color::Yellow
                    } else {
                        Color::White
                    };
                    ListItem::new(Line::from(Span::styled(s, Style::default().fg(color))))
                }
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
            .highlight_symbol("> ");

        self.list_state.select(Some(self.selected));
        frame.render_stateful_widget(list, inner, &mut self.list_state);
    }
}
