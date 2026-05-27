pub mod dashboard;
pub mod file_browser;
pub mod git;
pub mod issues;
pub mod notes;
pub mod popup;
pub mod prs;
pub mod roadmap;
pub mod stats;

use crate::theme::Theme;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

pub fn keybinds_bar(frame: &mut Frame, area: Rect, keys: &str, theme: &Theme) {
    let style = Style::default()
        .bg(theme.surface)
        .fg(theme.text)
        .add_modifier(Modifier::BOLD);
    let msg = Paragraph::new(Line::from(Span::styled(keys, style)));
    frame.render_widget(msg, area);
}

pub fn status_bar(
    frame: &mut Frame,
    area: Rect,
    text: &str,
    repo_info: Option<&str>,
    theme: &Theme,
) {
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(1), Constraint::Length(40)])
        .split(area);

    let status_style = Style::default()
        .bg(theme.surface)
        .fg(theme.text)
        .add_modifier(Modifier::BOLD);

    let msg = Paragraph::new(Line::from(Span::styled(text, status_style)));
    frame.render_widget(msg, layout[0]);

    if let Some(repo) = repo_info {
        let repo_style = Style::default()
            .bg(theme.surface)
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD);
        let repo_text = Paragraph::new(Line::from(Span::styled(repo, repo_style)));
        frame.render_widget(repo_text, layout[1]);
    }
}

pub fn centered_rect(percent_x: u16, percent_y: u16, within: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Ratio(u32::from((100 - percent_y) / 200), 1),
            Constraint::Ratio(u32::from(percent_y), 100),
            Constraint::Ratio(u32::from((100 - percent_y) / 200), 1),
        ])
        .split(within);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(u32::from((100 - percent_x) / 200), 1),
            Constraint::Ratio(u32::from(percent_x), 100),
            Constraint::Ratio(u32::from((100 - percent_x) / 200), 1),
        ])
        .split(popup_layout[1])[1]
}

#[allow(dead_code)]
pub fn title_block(title: &str, theme: &Theme) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title(format!(" {title} "))
        .style(Style::default().fg(theme.accent))
}
