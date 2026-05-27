use crate::github::{Comment, PullRequest};
use crate::theme::Theme;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

pub struct PRsView {
    pub prs: Vec<PullRequest>,
    pub selected: usize,
    #[allow(dead_code)]
    pub filter_state: Option<String>,
    list_state: ListState,
}

impl PRsView {
    pub fn new(prs: Vec<PullRequest>) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            prs,
            selected: 0,
            filter_state: None,
            list_state,
        }
    }

    pub fn draw(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        detail_pr: Option<&PullRequest>,
        comments: Option<&[Comment]>,
        theme: &Theme,
    ) {
        let layout = if detail_pr.is_some() {
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
            .title(" Pull Requests ")
            .style(Style::default().fg(theme.accent));
        let inner = block.inner(layout[0]);

        frame.render_widget(block, layout[0]);

        let items: Vec<ListItem> = self
            .prs
            .iter()
            .map(|pr| {
                let checks_indicator = match pr.checks_state.as_deref() {
                    Some("success") => "  ",
                    Some("failure") => "  ",
                    Some("pending") => "  ",
                    _ => "",
                };

                let merge_indicator = match pr.mergeable.as_deref() {
                    Some("mergeable") => "",
                    Some("conflict") => " ⚠",
                    _ => "",
                };

                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!(" #{} ", pr.number),
                        Style::default().fg(theme.text_dim),
                    ),
                    Span::styled(
                        if pr.state == "open" { " " } else { " " },
                        Style::default().fg(if pr.state == "open" {
                            theme.success
                        } else {
                            theme.danger
                        }),
                    ),
                    Span::raw(&pr.title),
                    Span::styled(checks_indicator, Style::default().fg(theme.success)),
                    Span::styled(merge_indicator, Style::default().fg(theme.danger)),
                ]))
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

        if let (Some(pr), Some(comments)) = (detail_pr, comments) {
            self.draw_detail(frame, layout[1], pr, comments, theme);
        } else if let Some(pr) = detail_pr {
            self.draw_detail(frame, layout[1], pr, &[], theme);
        }
    }

    fn draw_detail(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        pr: &PullRequest,
        comments: &[Comment],
        theme: &Theme,
    ) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" PR #{} ", pr.number))
            .style(Style::default().fg(theme.accent));

        let mut lines = vec![
            Line::from(vec![Span::styled(
                &pr.title,
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::raw(" by "),
                Span::styled(
                    pr.author.as_deref().unwrap_or("unknown"),
                    Style::default().fg(theme.accent),
                ),
                Span::raw(" | "),
                Span::styled(
                    pr.head_branch.as_deref().unwrap_or("?"),
                    Style::default().fg(theme.warning),
                ),
                Span::raw(" → "),
                Span::styled(
                    pr.base_branch.as_deref().unwrap_or("?"),
                    Style::default().fg(theme.warning),
                ),
            ]),
            Line::from(vec![
                Span::raw("State: "),
                Span::styled(
                    &pr.state,
                    Style::default().fg(match pr.state.as_str() {
                        "open" => theme.success,
                        _ => theme.danger,
                    }),
                ),
                Span::raw(" | Mergeable: "),
                Span::styled(
                    pr.mergeable.as_deref().unwrap_or("unknown"),
                    Style::default().fg(match pr.mergeable.as_deref() {
                        Some("mergeable") => theme.success,
                        Some("conflict") => theme.danger,
                        _ => theme.warning,
                    }),
                ),
                Span::raw(" | Checks: "),
                Span::styled(
                    pr.checks_state.as_deref().unwrap_or("?"),
                    Style::default().fg(match pr.checks_state.as_deref() {
                        Some("success") => theme.success,
                        Some("failure") => theme.danger,
                        _ => theme.warning,
                    }),
                ),
            ]),
            Line::from(Span::raw("")),
        ];

        if let Some(body) = &pr.body {
            for line in body.lines() {
                lines.push(Line::from(Span::raw(line)));
            }
        }

        if !comments.is_empty() {
            lines.push(Line::from(Span::raw("")));
            lines.push(Line::from(Span::styled(
                format!("─── {} comments ───", comments.len()),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(Span::raw("")));

            for comment in comments {
                lines.push(Line::from(vec![
                    Span::styled(
                        comment.author.as_deref().unwrap_or("unknown"),
                        Style::default()
                            .fg(theme.accent)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("  {}", comment.created_at.as_deref().unwrap_or("")),
                        Style::default().fg(theme.text_dim),
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

        let detail = Paragraph::new(lines).block(block);
        frame.render_widget(detail, area);
    }
}
