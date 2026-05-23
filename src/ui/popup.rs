use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

pub fn input_dialog(frame: &mut Frame, area: Rect, title: &str, value: &str, help: &str) {
    let popup = crate::ui::centered_rect(60, 20, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {title} "))
        .style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let input = Paragraph::new(Span::raw(value))
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(input, inner);

    let hint = Paragraph::new(Line::from(Span::styled(
        help,
        Style::default().fg(Color::DarkGray),
    )));
    let hint_area = Rect::new(inner.x, inner.bottom(), inner.width, 1);
    frame.render_widget(hint, hint_area);

    let cursor_x = inner.x + 1 + value.len() as u16;
    let cursor_y = inner.y + 1;
    let _ = frame.set_cursor_position((cursor_x.min(inner.right().saturating_sub(1)), cursor_y));
}

pub fn confirm_dialog(frame: &mut Frame, area: Rect, title: &str, message: &str) {
    let popup = crate::ui::centered_rect(50, 20, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {title} "))
        .style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup);
    let msg = Paragraph::new(Line::from(Span::styled(
        message,
        Style::default().fg(Color::White),
    )));

    let hint = Paragraph::new(Line::from(Span::styled(
        "[y]es  [n]o  [esc] cancel",
        Style::default().fg(Color::DarkGray),
    )));

    let layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Min(1),
            ratatui::layout::Constraint::Length(1),
        ])
        .split(inner);

    frame.render_widget(block, popup);
    frame.render_widget(msg, layout[0]);
    frame.render_widget(hint, layout[1]);
}

pub fn merge_dialog(frame: &mut Frame, area: Rect, selected: usize) {
    let popup = crate::ui::centered_rect(50, 30, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Merge PR ")
        .style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup);
    let methods = [
        ("1", "merge commit"),
        ("2", "squash"),
        ("3", "rebase"),
    ];

    let items: Vec<Line> = methods
        .iter()
        .enumerate()
        .map(|(i, (key, label))| {
            let style = if i == selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            Line::from(Span::styled(format!(" [{key}] {label}"), style))
        })
        .collect();

    let mut lines = items;
    lines.push(Line::from(Span::raw("")));
    lines.push(Line::from(Span::styled(
        " esc to cancel ",
        Style::default().fg(Color::DarkGray),
    )));

    let paragraph = Paragraph::new(lines);
    frame.render_widget(block, popup);
    frame.render_widget(paragraph, inner);
}
