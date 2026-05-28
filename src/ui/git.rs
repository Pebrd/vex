use crate::diff::{self, DiffHunk};
use crate::git::{self, BranchInfo, CommitInfo, FileStatus};
use crate::theme::Theme;
use anyhow::Result;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(PartialEq)]
#[allow(dead_code)]
pub enum GitMode {
    Files,
    Commits,
    Branches,
}

#[derive(Clone, Default)]
pub struct MultiSelect {
    pub active: bool,
    pub selected: HashSet<usize>,
}

impl MultiSelect {
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(dead_code)]
    pub fn toggle(&mut self) {
        self.active = !self.active;
    }

    pub fn is_selected(&self, idx: usize) -> bool {
        self.selected.contains(&idx)
    }

    pub fn toggle_item(&mut self, idx: usize) {
        if !self.selected.insert(idx) {
            self.selected.remove(&idx);
        }
    }

    pub fn clear(&mut self) {
        self.selected.clear();
        self.active = false;
    }

    pub fn selected_indices(&self) -> Vec<usize> {
        let mut v: Vec<_> = self.selected.iter().copied().collect();
        v.sort();
        v
    }
}

#[allow(dead_code)]
pub struct GitScreen {
    pub repo_path: Option<PathBuf>,
    pub mode: GitMode,
    pub focus: bool,
    pub files: Vec<FileStatus>,
    pub files_list_state: ListState,
    pub commits: Vec<CommitInfo>,
    pub commits_list_state: ListState,
    pub branches: Vec<BranchInfo>,
    pub branches_list_state: ListState,
    pub diff_hunks: Vec<DiffHunk>,
    pub current_branch: String,
    pub loading: bool,
    pub status: String,
    pub multi_select: MultiSelect,
}

#[allow(dead_code)]
impl GitScreen {
    pub fn new() -> Self {
        Self {
            repo_path: None,
            mode: GitMode::Files,
            focus: true,
            files: Vec::new(),
            files_list_state: ListState::default(),
            commits: Vec::new(),
            commits_list_state: ListState::default(),
            branches: Vec::new(),
            branches_list_state: ListState::default(),
            diff_hunks: Vec::new(),
            current_branch: String::new(),
            loading: false,
            status: String::new(),
            multi_select: MultiSelect::new(),
        }
    }

    pub fn set_repo_path(&mut self, path: Option<PathBuf>) {
        self.repo_path = path;
    }

    pub fn refresh(&mut self) {
        let repo_path = match &self.repo_path {
            Some(p) => p.clone(),
            None => {
                self.status = "no project selected".to_string();
                return;
            }
        };
        self.loading = true;
        self.current_branch = git::current_branch_for(&repo_path).unwrap_or_default();
        self.refresh_files(&repo_path);
        self.refresh_commits(&repo_path, 50);
        self.refresh_branches(&repo_path);
        self.loading = false;
    }

    fn repo(&self, path: &Path) -> Result<git2::Repository> {
        git::open_repo(path)
    }

    fn refresh_files(&mut self, repo_path: &Path) {
        match self.repo(repo_path).and_then(|r| git::get_statuses(&r)) {
            Ok(files) => {
                self.files = files;
                if self.files_list_state.selected().is_none() && !self.files.is_empty() {
                    self.files_list_state.select(Some(0));
                }
            }
            Err(e) => self.status = format!("error: {e}"),
        }
    }

    fn refresh_commits(&mut self, repo_path: &Path, max: usize) {
        match self.repo(repo_path).and_then(|r| git::get_commits(&r, max)) {
            Ok(commits) => {
                self.commits = commits;
                if self.commits_list_state.selected().is_none() && !self.commits.is_empty() {
                    self.commits_list_state.select(Some(0));
                }
            }
            Err(e) => self.status = format!("error: {e}"),
        }
    }

    fn refresh_branches(&mut self, repo_path: &Path) {
        match self.repo(repo_path).and_then(|r| git::get_branches(&r)) {
            Ok(branches) => {
                self.branches = branches;
                if self.branches_list_state.selected().is_none() && !self.branches.is_empty() {
                    self.branches_list_state.select(Some(0));
                }
            }
            Err(e) => self.status = format!("error: {e}"),
        }
    }
}

// -- action methods --

#[allow(dead_code)]
impl GitScreen {
    pub fn stage_selected(&mut self) {
        let repo_path = match &self.repo_path {
            Some(p) => p.clone(),
            None => return,
        };
        let idx = self.files_list_state.selected();
        let file = match idx.and_then(|i| self.files.get(i)) {
            Some(f) => f.clone(),
            None => return,
        };
        match git::open_repo(&repo_path).and_then(|r| git::stage_file(&r, &file.path)) {
            Ok(_) => self.refresh_files(&repo_path),
            Err(e) => self.status = format!("error: {e}"),
        }
    }

    pub fn unstage_selected(&mut self) {
        let repo_path = match &self.repo_path {
            Some(p) => p.clone(),
            None => return,
        };
        let idx = self.files_list_state.selected();
        let file = match idx.and_then(|i| self.files.get(i)) {
            Some(f) => f.clone(),
            None => return,
        };
        match git::open_repo(&repo_path).and_then(|r| git::unstage_file(&r, &file.path)) {
            Ok(_) => self.refresh_files(&repo_path),
            Err(e) => self.status = format!("error: {e}"),
        }
    }

    pub fn stage_all(&mut self) {
        let repo_path = match &self.repo_path {
            Some(p) => p.clone(),
            None => return,
        };
        match git::open_repo(&repo_path).and_then(|r| git::stage_all(&r)) {
            Ok(_) => self.refresh_files(&repo_path),
            Err(e) => self.status = format!("error: {e}"),
        }
    }

    pub fn unstage_all(&mut self) {
        let repo_path = match &self.repo_path {
            Some(p) => p.clone(),
            None => return,
        };
        match git::open_repo(&repo_path).and_then(|r| git::unstage_all(&r)) {
            Ok(_) => self.refresh_files(&repo_path),
            Err(e) => self.status = format!("error: {e}"),
        }
    }

    pub fn discard_selected(&mut self) {
        let repo_path = match &self.repo_path {
            Some(p) => p.clone(),
            None => return,
        };
        let idx = self.files_list_state.selected();
        let file = match idx.and_then(|i| self.files.get(i)) {
            Some(f) if !f.staged => f.clone(),
            _ => return,
        };
        match git::open_repo(&repo_path).and_then(|r| git::discard_file(&r, &file.path)) {
            Ok(_) => self.refresh_files(&repo_path),
            Err(e) => self.status = format!("error: {e}"),
        }
    }

    pub fn commit(&mut self, message: &str) {
        let repo_path = match &self.repo_path {
            Some(p) => p.clone(),
            None => return,
        };
        match git::open_repo(&repo_path).and_then(|r| git::create_commit(&r, message)) {
            Ok(_) => {
                self.status = "committed".to_string();
                self.refresh_files(&repo_path);
                self.refresh_commits(&repo_path, 50);
            }
            Err(e) => self.status = format!("error: {e}"),
        }
    }

    pub fn checkout_selected_branch(&mut self) {
        let repo_path = match &self.repo_path {
            Some(p) => p.clone(),
            None => return,
        };
        let idx = self.branches_list_state.selected();
        let branch = match idx.and_then(|i| self.branches.get(i)) {
            Some(b) if !b.is_current => b.clone(),
            _ => return,
        };
        match git::open_repo(&repo_path).and_then(|r| git::checkout_branch(&r, &branch.name)) {
            Ok(_) => {
                self.status = format!("switched to {}", branch.name);
                self.refresh_data(&repo_path);
            }
            Err(e) => self.status = format!("error: {e}"),
        }
    }

    fn refresh_data(&mut self, repo_path: &Path) {
        self.current_branch = git::current_branch_for(repo_path).unwrap_or_default();
        self.refresh_files(repo_path);
        self.refresh_commits(repo_path, 50);
        self.refresh_branches(repo_path);
    }

    pub fn push(&mut self) {
        let repo_path = match &self.repo_path {
            Some(p) => p.clone(),
            None => return,
        };
        match git::open_repo(&repo_path).and_then(|r| git::push_current_branch(&r)) {
            Ok(_) => self.status = "pushed".to_string(),
            Err(e) => self.status = format!("error: {e}"),
        }
    }

    pub fn pull(&mut self) {
        let repo_path = match &self.repo_path {
            Some(p) => p.clone(),
            None => return,
        };
        match git::open_repo(&repo_path).and_then(|r| git::pull(&r)) {
            Ok(_) => {
                self.status = "pulled".to_string();
                self.refresh_data(&repo_path);
            }
            Err(e) => self.status = format!("error: {e}"),
        }
    }

    pub fn fetch(&mut self) {
        let repo_path = match &self.repo_path {
            Some(p) => p.clone(),
            None => return,
        };
        match git::open_repo(&repo_path).and_then(|r| git::fetch(&r)) {
            Ok(_) => {
                self.status = "fetched".to_string();
                self.refresh_commits(&repo_path, 50);
            }
            Err(e) => self.status = format!("error: {e}"),
        }
    }

    pub fn stash(&mut self) {
        let repo_path = match &self.repo_path {
            Some(p) => p.clone(),
            None => return,
        };
        // stash_push requires &mut Repository
        match git::open_repo(&repo_path).and_then(|mut r| git::stash_push(&mut r, "wip")) {
            Ok(_) => {
                self.status = "stashed".to_string();
                self.refresh_files(&repo_path);
            }
            Err(e) => self.status = format!("error: {e}"),
        }
    }

    pub fn stash_pop(&mut self) {
        let repo_path = match &self.repo_path {
            Some(p) => p.clone(),
            None => return,
        };
        // stash_pop requires &mut Repository
        match git::open_repo(&repo_path).and_then(|mut r| git::stash_pop(&mut r)) {
            Ok(_) => {
                self.status = "stash popped".to_string();
                self.refresh_files(&repo_path);
            }
            Err(e) => self.status = format!("error: {e}"),
        }
    }

    pub fn toggle_multi_select(&mut self) {
        if self.multi_select.active {
            self.multi_select.clear();
        } else {
            self.multi_select.active = true;
            self.status = "multi-select: space to toggle, V to clear, t to stage/unstage selected"
                .to_string();
        }
    }

    pub fn stage_unstage_multi(&mut self) {
        let repo_path = match &self.repo_path {
            Some(p) => p.clone(),
            None => return,
        };
        let indices = self.multi_select.selected_indices();
        if indices.is_empty() {
            return;
        }
        match git::open_repo(&repo_path) {
            Ok(repo) => {
                for idx in indices {
                    if let Some(file) = self.files.get(idx) {
                        if file.staged {
                            let _ = git::unstage_file(&repo, &file.path);
                        } else {
                            let _ = git::stage_file(&repo, &file.path);
                        }
                    }
                }
                self.multi_select.clear();
                self.refresh_files(&repo_path);
            }
            Err(e) => self.status = format!("error: {e}"),
        }
    }

    pub fn delete_branches_multi(&mut self) {
        let repo_path = match &self.repo_path {
            Some(p) => p.clone(),
            None => return,
        };
        let indices = self.multi_select.selected_indices();
        if indices.is_empty() {
            return;
        }
        match git::open_repo(&repo_path) {
            Ok(repo) => {
                let mut deleted = 0u32;
                for idx in indices {
                    if let Some(branch) = self.branches.get(idx) {
                        if !branch.is_current {
                            if git::delete_branch(&repo, &branch.name).is_ok() {
                                deleted += 1;
                            }
                        }
                    }
                }
                self.multi_select.clear();
                self.refresh_branches(&repo_path);
                if deleted > 0 {
                    self.status = format!("deleted {deleted} branch(es)");
                }
            }
            Err(e) => self.status = format!("error: {e}"),
        }
    }
}

// -- draw / render methods --

#[allow(dead_code)]
impl GitScreen {
    pub fn draw(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)])
            .split(area);

        match self.mode {
            GitMode::Files => self.draw_files_panel(frame, chunks[0], theme),
            GitMode::Commits => self.draw_commits_panel(frame, chunks[0], theme),
            GitMode::Branches => self.draw_branches_panel(frame, chunks[0], theme),
        }

        self.draw_right_panel(frame, chunks[1], theme);
    }

    fn left_block(&self, title: &str, theme: &Theme) -> Block<'static> {
        let border_style = if self.focus {
            Style::default().fg(theme.accent)
        } else {
            Style::default().fg(theme.border)
        };
        Block::default()
            .title(format!(" {title} "))
            .borders(Borders::ALL)
            .border_style(border_style)
    }

    fn right_block(&self, title: &str, theme: &Theme) -> Block<'static> {
        let border_style = if !self.focus {
            Style::default().fg(theme.accent)
        } else {
            Style::default().fg(theme.border)
        };
        Block::default()
            .title(format!(" {title} "))
            .borders(Borders::ALL)
            .border_style(border_style)
    }

    fn draw_files_panel(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let items: Vec<ListItem> = self
            .files
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let (symbol, style) = if f.is_untracked {
                    ("[+]", Style::default().fg(theme.success))
                } else if f.staged {
                    match f.status.trim() {
                        "M" => (
                            "[M]",
                            Style::default()
                                .fg(theme.success)
                                .add_modifier(Modifier::BOLD),
                        ),
                        "A" => ("[A]", Style::default().fg(theme.success)),
                        "D" => ("[D]", Style::default().fg(theme.danger)),
                        _ => (&f.status[..3], Style::default().fg(theme.success)),
                    }
                } else {
                    match f.status.trim() {
                        "M" => ("[M]", Style::default().fg(theme.warning)),
                        "D" => ("[D]", Style::default().fg(theme.danger)),
                        _ => ("[?]", Style::default().fg(theme.warning)),
                    }
                };
                let multi_prefix = if self.multi_select.active {
                    if self.multi_select.is_selected(i) {
                        "● "
                    } else {
                        "○ "
                    }
                } else {
                    ""
                };
                let mut item_style = style;
                if self.multi_select.active && self.multi_select.is_selected(i) {
                    item_style = item_style.bg(theme.selection);
                }
                ListItem::new(Line::from(Span::styled(
                    format!("{multi_prefix}{symbol} {}", f.path),
                    item_style,
                )))
            })
            .collect();

        let list = List::new(items)
            .block(self.left_block("Files", theme))
            .highlight_style(Style::default().fg(Color::Black).bg(theme.selection));

        frame.render_stateful_widget(list, area, &mut self.files_list_state);
    }

    fn draw_commits_panel(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let items: Vec<ListItem> = self
            .commits
            .iter()
            .map(|c| {
                let branch_tag = if c.branch_names.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", c.branch_names.join(", "))
                };
                let label = format!("{} {}{}", c.short_id, c.message, branch_tag);
                let style = if c.is_head {
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.text)
                };
                ListItem::new(Line::from(Span::styled(label, style)))
            })
            .collect();

        let list = List::new(items)
            .block(self.left_block("Commits", theme))
            .highlight_style(Style::default().fg(Color::Black).bg(theme.selection));

        frame.render_stateful_widget(list, area, &mut self.commits_list_state);
    }

    fn draw_branches_panel(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let items: Vec<ListItem> = self
            .branches
            .iter()
            .enumerate()
            .map(|(i, b)| {
                let prefix = if b.is_current { "* " } else { "  " };
                let upstream = b
                    .upstream
                    .as_ref()
                    .map(|u| format!(" → {u}"))
                    .unwrap_or_default();
                let multi_prefix = if self.multi_select.active {
                    if self.multi_select.is_selected(i) {
                        "● "
                    } else {
                        "○ "
                    }
                } else {
                    ""
                };
                let label = format!("{multi_prefix}{prefix}{}{upstream}", b.name);
                let mut style = if b.is_current {
                    Style::default()
                        .fg(theme.success)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.text)
                };
                if self.multi_select.active && self.multi_select.is_selected(i) {
                    style = style.bg(theme.selection);
                }
                ListItem::new(Line::from(Span::styled(label, style)))
            })
            .collect();

        let list = List::new(items)
            .block(self.left_block("Branches", theme))
            .highlight_style(Style::default().fg(Color::Black).bg(theme.selection));

        frame.render_stateful_widget(list, area, &mut self.branches_list_state);
    }

    fn draw_right_panel(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let title = match self.mode {
            GitMode::Files => "Diff (Working Tree)",
            GitMode::Commits => "Diff (Commit)",
            GitMode::Branches => "Branch Details",
        };
        let block = self.right_block(title, theme);

        if self.diff_hunks.is_empty() {
            let text = Paragraph::new("Select an item to view diff")
                .style(Style::default().fg(theme.text_dim))
                .block(block);
            frame.render_widget(text, area);
        } else {
            let inner = block.inner(area);
            let lines = diff::render_side_by_side(&self.diff_hunks, theme, inner.width as usize);
            let text = Paragraph::new(lines)
                .block(block)
                .wrap(Wrap { trim: false });
            frame.render_widget(text, area);
        }
    }

    pub fn load_diff_for_selected(&mut self) {
        let repo_path = match &self.repo_path {
            Some(p) => p.clone(),
            None => return,
        };
        match self.mode {
            GitMode::Files => {
                let idx = self.files_list_state.selected();
                if let Some(file) = idx.and_then(|i| self.files.get(i)) {
                    let repo = match git::open_repo(&repo_path) {
                        Ok(r) => r,
                        Err(_) => return,
                    };
                    let mut opts = git2::DiffOptions::new();
                    opts.pathspec(file.path.clone());
                    if let (Ok(head), Ok(index)) = (repo.head(), repo.index()) {
                        if let Ok(head_tree) = head.peel_to_tree() {
                            let diff = repo.diff_tree_to_index(
                                Some(&head_tree),
                                Some(&index),
                                Some(&mut opts),
                            );
                            if let Ok(diff) = diff {
                                let mut buf: Vec<u8> = Vec::new();
                                let _ =
                                    diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
                                        let prefix = match line.origin() {
                                            '+' => "+",
                                            '-' => "-",
                                            'H' => " ",
                                            _ => " ",
                                        };
                                        let content =
                                            std::str::from_utf8(line.content()).unwrap_or("");
                                        let _ = write!(buf, "{prefix}{content}");
                                        true
                                    });
                                let text = String::from_utf8(buf).unwrap_or_default();
                                self.diff_hunks = diff::parse_diff(&text);
                                return;
                            }
                        }
                    }
                    self.diff_hunks = Vec::new();
                }
            }
            GitMode::Commits => {
                let idx = self.commits_list_state.selected();
                if let Some(commit) = idx.and_then(|i| self.commits.get(i)) {
                    match git::open_repo(&repo_path)
                        .and_then(|r| git::get_commit_diff(&r, &commit.id))
                    {
                        Ok(d) => self.diff_hunks = diff::parse_diff(&d),
                        Err(_) => self.diff_hunks = Vec::new(),
                    }
                }
            }
            GitMode::Branches => {
                self.diff_hunks = Vec::new();
            }
        }
    }
}

// -- navigation and status keys --

#[allow(dead_code)]
impl GitScreen {
    pub fn status_keys(&self) -> Vec<&str> {
        if self.multi_select.active {
            match self.mode {
                GitMode::Files => vec![
                    "j/k navigate",
                    "space toggle item",
                    "t stage/unstage selected",
                    "V exit multi-select",
                    "q back",
                ],
                GitMode::Commits => vec![
                    "j/k navigate",
                    "enter view diff",
                    "1-3 switch mode",
                    "q back",
                ],
                GitMode::Branches => vec![
                    "j/k navigate",
                    "space toggle item",
                    "d delete selected",
                    "V exit multi-select",
                    "q back",
                ],
            }
        } else {
            match self.mode {
                GitMode::Files => vec![
                    "j/k navigate",
                    "space stage/unstage",
                    "t toggle all",
                    "s commit",
                    "d discard",
                    "enter diff",
                    "1-3 switch mode",
                    "q back",
                ],
                GitMode::Commits => vec![
                    "j/k navigate",
                    "enter view diff",
                    "1-3 switch mode",
                    "q back",
                ],
                GitMode::Branches => vec![
                    "j/k navigate",
                    "enter checkout",
                    "n new branch",
                    "d delete",
                    "V multi-select",
                    "1-3 switch mode",
                    "q back",
                ],
            }
        }
    }

    pub fn navigate_down(&mut self) {
        match self.mode {
            GitMode::Files => {
                let i = self.files_list_state.selected().unwrap_or(0);
                let next = (i + 1).min(self.files.len().saturating_sub(1));
                self.files_list_state.select(Some(next));
            }
            GitMode::Commits => {
                let i = self.commits_list_state.selected().unwrap_or(0);
                let next = (i + 1).min(self.commits.len().saturating_sub(1));
                self.commits_list_state.select(Some(next));
            }
            GitMode::Branches => {
                let i = self.branches_list_state.selected().unwrap_or(0);
                let next = (i + 1).min(self.branches.len().saturating_sub(1));
                self.branches_list_state.select(Some(next));
            }
        }
    }

    pub fn navigate_up(&mut self) {
        match self.mode {
            GitMode::Files => {
                let i = self.files_list_state.selected().unwrap_or(0);
                let prev = i.saturating_sub(1);
                self.files_list_state.select(Some(prev));
            }
            GitMode::Commits => {
                let i = self.commits_list_state.selected().unwrap_or(0);
                let prev = i.saturating_sub(1);
                self.commits_list_state.select(Some(prev));
            }
            GitMode::Branches => {
                let i = self.branches_list_state.selected().unwrap_or(0);
                let prev = i.saturating_sub(1);
                self.branches_list_state.select(Some(prev));
            }
        }
    }

    pub fn toggle_focus(&mut self) {
        self.focus = !self.focus;
    }

    pub fn set_mode(&mut self, mode: GitMode) {
        self.mode = mode;
        self.focus = true;
        self.diff_hunks = Vec::new();
    }
}
