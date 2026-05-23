pub mod dashboard;
pub mod issues;
pub mod popup;
pub mod prs;

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub fn status_bar(frame: &mut Frame, area: Rect, text: &str, repo_info: Option<&str>) {
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(1), Constraint::Length(40)])
        .split(area);

    let status_style = Style::default()
        .bg(Color::DarkGray)
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);

    let msg = Paragraph::new(Line::from(Span::styled(text, status_style)));
    frame.render_widget(msg, layout[0]);

    if let Some(repo) = repo_info {
        let repo_style = Style::default()
            .bg(Color::DarkGray)
            .fg(Color::Cyan)
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

pub fn title_block(title: &str) -> Block {
    Block::default()
        .borders(Borders::ALL)
        .title(format!(" {title} "))
        .style(Style::default().fg(Color::Cyan))
}
