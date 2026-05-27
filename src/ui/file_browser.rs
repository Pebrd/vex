use crate::theme::Theme;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub is_git_repo: bool,
}

pub struct FileBrowser {
    pub current_dir: PathBuf,
    pub entries: Vec<DirEntry>,
    pub selected: usize,
    pub show_hidden: bool,
}

impl FileBrowser {
    pub fn new(path: &Path) -> Self {
        let path = if path.exists() && path.is_dir() {
            path.to_path_buf()
        } else {
            dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
        };

        let mut fb = Self {
            current_dir: path,
            entries: Vec::new(),
            selected: 0,
            show_hidden: false,
        };
        fb.refresh();
        fb
    }

    pub fn navigate(&mut self, dir: &Path) {
        if dir.exists() && dir.is_dir() {
            self.current_dir = dir.to_path_buf();
            self.selected = 0;
            self.refresh();
        }
    }

    pub fn go_up(&mut self) {
        if let Some(parent) = self.current_dir.parent() {
            let prev = self.current_dir.clone();
            self.current_dir = parent.to_path_buf();
            self.selected = 0;
            self.refresh();
            // Try to select the directory we just left
            if let Some(pos) = self.entries.iter().position(|e| e.path == prev) {
                self.selected = pos;
            }
        }
    }

    pub fn refresh(&mut self) {
        self.entries.clear();
        let mut dirs = Vec::new();
        let mut files = Vec::new();

        if let Ok(read) = std::fs::read_dir(&self.current_dir) {
            for entry in read.flatten() {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();

                if !self.show_hidden && name.starts_with('.') {
                    continue;
                }

                let is_dir = path.is_dir();
                let is_git_repo = is_dir && path.join(".git").exists();

                let de = DirEntry {
                    name,
                    path,
                    is_dir,
                    is_git_repo,
                };

                if is_dir {
                    dirs.push(de);
                } else {
                    files.push(de);
                }
            }
        }

        dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        self.entries.extend(dirs);
        self.entries.extend(files);

        if !self.entries.is_empty() {
            self.selected = self.selected.min(self.entries.len() - 1);
        }
    }

    pub fn selected_path(&self) -> Option<PathBuf> {
        self.entries.get(self.selected).map(|e| e.path.clone())
    }

    pub fn selected_is_dir(&self) -> bool {
        self.entries
            .get(self.selected)
            .map(|e| e.is_dir)
            .unwrap_or(false)
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(area);

        let path_str = self.current_dir.to_string_lossy();
        let path_display = Paragraph::new(Line::from(Span::styled(
            &*path_str,
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )));
        frame.render_widget(path_display, layout[0]);

        let items: Vec<ListItem> = self
            .entries
            .iter()
            .map(|e| {
                let icon = if e.is_git_repo {
                    "  "
                } else if e.is_dir {
                    "  "
                } else {
                    "  "
                };
                let name_style = if e.is_dir {
                    Style::default()
                        .fg(theme.warning)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.text)
                };
                let git_indicator = if e.is_git_repo {
                    Span::styled(" ", Style::default().fg(theme.success))
                } else {
                    Span::raw("")
                };
                ListItem::new(Line::from(vec![
                    Span::styled(icon, Style::default().fg(theme.text_dim)),
                    Span::styled(&e.name, name_style),
                    git_indicator,
                ]))
            })
            .collect();

        let mut list_state = ListState::default();
        list_state.select(Some(self.selected));

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(theme.selection)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Select a git project ")
                    .style(Style::default().fg(theme.accent)),
            );

        frame.render_stateful_widget(list, layout[1], &mut list_state);

        let hint = Paragraph::new(Line::from(Span::styled(
            " j/k navigate  enter enter dir / select  esc go up  h toggle hidden  q cancel ",
            Style::default().fg(theme.text_dim),
        )));
        frame.render_widget(hint, layout[2]);
    }
}
