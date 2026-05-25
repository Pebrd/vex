use crate::cache::Cache;
use crate::config::{self, Config};
use crate::git;
use crate::github::{Client, Comment, Issue, PullRequest};
use crate::notes::{self, Note};
use crate::ui::dashboard::Dashboard;
use crate::ui::file_browser::FileBrowser;
use crate::ui::issues::{FocusTarget, IssuesView};
use crate::ui::notes::NotesView;
use crate::ui::prs::PRsView;
use crate::ui::roadmap::RoadmapView;
use crate::ui::stats::StatsView;
use crate::ui::{popup, status_bar};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

#[derive(Debug, Clone, Copy, PartialEq)]
enum SortMode {
    CreatedNewest,
    CreatedOldest,
    Updated,
    Number,
    State,
}
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use crate::ui::centered_rect;
use ratatui::Frame;
use ratatui::Terminal;
use std::collections::HashMap;
use std::io::Stdout;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

#[derive(PartialEq)]
enum Screen {
    Dashboard,
    Issues,
    PullRequests,
    Notes,
    Stats,
    Roadmap,
}

#[derive(PartialEq)]
enum DetailTarget {
    Issue,
    Note,
}

enum BgEvent {
    IssuesReady(Vec<Issue>),
    PRsReady(Vec<PullRequest>),
    CommentsReady(u64, Vec<Comment>),
    #[allow(dead_code)]
    LabelsReady(Vec<String>),
    Error(String),
}

enum InputMode {
    None,
    Search { query: String },
    BrowseProject { browser: FileBrowser },
    Comment { body: String },
    ConfirmMerge { pr_number: u64, selected: u8 },
    CreateNote { title: String, body: String, step: u8 },
    CreatePr { title: String, body: String, step: u8 },
    EditIssue { title: String, body: String, focus: u8, issue_number: u64, labels: Vec<String>, available_labels: Vec<String>, label_idx: usize },
    EditNote { slug: Option<String>, title: String, body: String, focus: u8 },
    LinkNote { input: String },
    CreateRepo { name: String, private: bool, step: u8 },
}

pub struct App {
    config: Config,
    cache: Arc<Cache>,
    client: Arc<Client>,
    cmd_tx: mpsc::UnboundedSender<BgEvent>,
    cmd_rx: mpsc::UnboundedReceiver<BgEvent>,
    screen: Screen,
    input_mode: InputMode,
    dashboard: Dashboard,
    issues_view: IssuesView,
    prs_view: PRsView,
    notes_view: NotesView,
    all_issues: Vec<Issue>,
    all_prs: Vec<PullRequest>,
    selected_issue: Option<Issue>,
    selected_pr: Option<PullRequest>,
    selected_note: Option<Note>,
    issue_comments: HashMap<u64, Vec<Comment>>,
    pr_comments: HashMap<u64, Vec<Comment>>,
    repo_owner: Option<String>,
    repo_name: Option<String>,
    repo_path: Option<PathBuf>,
    status: String,
    loading: bool,
    stats_view: StatsView,
    roadmap_view: RoadmapView,
    should_quit: bool,
    needs_redraw: bool,
    project_notes: Vec<Note>,
    selected_note_idx: usize,
    focus: FocusTarget,
    detail_target: DetailTarget,
    state_filter: String,
    search_matcher: SkimMatcherV2,
    last_search_press: Option<Instant>,
    last_fetched: Option<Instant>,
    sort_mode: SortMode,
    detail_scroll: u16,
    show_help: bool,
    #[allow(dead_code)]
    label_filter: Option<String>,
    #[allow(dead_code)]
    available_labels: Vec<String>,
}

impl App {
    pub async fn new(config: Config, cache: Cache) -> Result<Self> {
        let client = Arc::new(Client::new(&config));
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();

        let repo_path = git::project_root();
        let (repo_owner, repo_name, status) = if let Some(ref path) = repo_path {
            match git::detect(path) {
                Ok(Some(info)) => {
                    let o = info.owner.clone();
                    let r = info.repo.clone();
                    (Some(info.owner), Some(info.repo), format!("{o}/{r}"))
                }
                Ok(None) => (None, None, "no GitHub remote — press 'c' to create".to_string()),
                Err(e) => (None, None, format!("{e}")),
            }
        } else {
            (None, None, "no git repo detected".to_string())
        };

        let projects = config.projects.clone();
        let dashboard = Dashboard::new(projects);

        let project_notes = if let Some(ref path) = repo_path {
            notes::list_notes(path).unwrap_or_default()
        } else {
            Vec::new()
        };
        let notes_view = NotesView::new(project_notes.clone());

        let (owner_str, repo_str) = match (&repo_owner, &repo_name) {
            (Some(o), Some(r)) => (o.clone(), r.clone()),
            _ => (String::new(), String::new()),
        };

        let default_filter = config.default_filter.clone().unwrap_or_else(|| "open".to_string());

        let mut app = Self {
            config,
            cache: Arc::new(cache),
            client,
            cmd_tx,
            cmd_rx,
            screen: Screen::Dashboard,
            input_mode: InputMode::None,
            dashboard,
            issues_view: IssuesView::new(Vec::new()),
            prs_view: PRsView::new(Vec::new()),
            notes_view,
            all_issues: Vec::new(),
            all_prs: Vec::new(),
            selected_issue: None,
            selected_pr: None,
            selected_note: None,
            issue_comments: HashMap::new(),
            pr_comments: HashMap::new(),
            repo_owner,
            repo_name,
            repo_path,
            status,
            loading: false,
            should_quit: false,
            needs_redraw: true,
            stats_view: StatsView::new(&owner_str, &repo_str),
            roadmap_view: RoadmapView::new(&owner_str, &repo_str),
            project_notes,
            selected_note_idx: 0,
            focus: FocusTarget::Issues,
            detail_target: DetailTarget::Issue,
            state_filter: default_filter,
            search_matcher: SkimMatcherV2::default(),
            last_search_press: None,
            last_fetched: None,
            sort_mode: SortMode::CreatedNewest,
            detail_scroll: 0,
            show_help: false,
            label_filter: None,
            available_labels: Vec::new(),
        };

        if app.repo_owner.is_some() && app.repo_name.is_some() {
            app.switch_to_issues().await?;
        } else if let (Some(owner), Some(repo), Some(path)) = (&app.config.last_owner, &app.config.last_repo, &app.config.last_path) {
            app.repo_owner = Some(owner.clone());
            app.repo_name = Some(repo.clone());
            app.repo_path = Some(std::path::PathBuf::from(path));
            app.status = format!("{owner}/{repo} (restored)");
            app.switch_to_issues().await?;
        }

        Ok(app)
    }

    pub async fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        while !self.should_quit {
            if self.needs_redraw {
                terminal.draw(|frame| self.draw(frame))?;
                self.needs_redraw = false;
            }
            self.drain_bg_events();
            self.handle_events().await?;
        }
        self.config.last_owner = self.repo_owner.clone();
        self.config.last_repo = self.repo_name.clone();
        self.config.last_path = self.repo_path.as_ref().map(|p| p.to_string_lossy().to_string());
        let _ = config::save(&self.config);
        Ok(())
    }

    fn mark_dirty(&mut self) {
        self.needs_redraw = true;
    }

    fn apply_label_filter(&self) -> Vec<Issue> {
        match &self.label_filter {
            Some(label) => self.all_issues.iter().filter(|i| i.labels.contains(label)).cloned().collect(),
            None => self.all_issues.clone(),
        }
    }

    fn apply_sort(&mut self) {
        match self.sort_mode {
            SortMode::CreatedNewest => {
                self.issues_view.issues.sort_by(|a, b| b.created_at.cmp(&a.created_at));
                self.all_issues.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            }
            SortMode::CreatedOldest => {
                self.issues_view.issues.sort_by(|a, b| a.created_at.cmp(&b.created_at));
                self.all_issues.sort_by(|a, b| a.created_at.cmp(&b.created_at));
            }
            SortMode::Updated => {
                self.issues_view.issues.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
                self.all_issues.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
            }
            SortMode::Number => {
                self.issues_view.issues.sort_by(|a, b| a.number.cmp(&b.number));
                self.all_issues.sort_by(|a, b| a.number.cmp(&b.number));
            }
            SortMode::State => {
                self.issues_view.issues.sort_by(|a, b| a.state.cmp(&b.state));
                self.all_issues.sort_by(|a, b| a.state.cmp(&b.state));
            }
        }
        self.issues_view.selected = self.issues_view.selected.min(self.issues_view.issues.len().saturating_sub(1));
        self.selected_issue = self.issues_view.issues.get(self.issues_view.selected).cloned();
    }

    fn go_to_dashboard(&mut self) {
        self.repo_owner = None;
        self.repo_name = None;
        self.repo_path = None;
        self.selected_issue = None;
        self.selected_pr = None;
        self.selected_note = None;
        self.status = String::new();
        self.screen = Screen::Dashboard;
    }

    fn drain_bg_events(&mut self) {
        while let Ok(event) = self.cmd_rx.try_recv() {
            match event {
                BgEvent::IssuesReady(issues) => {
                    self.all_issues = issues;
                    let count = self.all_issues.len();
                    if !matches!(&self.input_mode, InputMode::Search { .. }) {
                        self.issues_view = IssuesView::new(self.apply_label_filter());
                        self.apply_sort();
                        self.issues_view.issues = self.apply_label_filter();
                    }
                    if let Some(ref owner) = self.repo_owner {
                        if let Some(ref repo) = self.repo_name {
                            self.status = format!("{owner}/{repo} — {count} issues ({})", self.state_filter);
                        }
                    }
                    self.last_fetched = Some(Instant::now());
                    self.loading = false;
                    self.mark_dirty();
                }
                BgEvent::PRsReady(prs) => {
                    self.all_prs = prs;
                    let count = self.all_prs.len();
                    if !matches!(&self.input_mode, InputMode::Search { .. }) {
                        self.prs_view = PRsView::new(self.all_prs.clone());
                    }
                    if let Some(ref owner) = self.repo_owner {
                        if let Some(ref repo) = self.repo_name {
                            if self.screen == Screen::PullRequests {
                                self.status = format!("{owner}/{repo} — {count} PRs");
                            }
                        }
                    }
                    self.last_fetched = Some(Instant::now());
                    self.loading = false;
                    self.mark_dirty();
                }
                BgEvent::CommentsReady(number, comments) => {
                    self.issue_comments.insert(number, comments);
                    self.mark_dirty();
                }
                BgEvent::LabelsReady(labels) => {
                    let labels_for_edit = labels;
                    if self.repo_owner.is_some() && self.repo_name.is_some() {
                        self.input_mode = InputMode::EditIssue {
                            title: String::new(),
                            body: String::new(),
                            focus: 0,
                            issue_number: 0,
                            labels: Vec::new(),
                            available_labels: labels_for_edit,
                            label_idx: 0,
                        };
                    }
                    self.status = "creating issue (Tab switch field, Ctrl+S save)".to_string();
                    self.mark_dirty();
                }
                BgEvent::Error(e) => {
                    self.status = e;
                    self.loading = false;
                    self.mark_dirty();
                }
            }
        }
    }

    fn draw(&mut self, frame: &mut Frame) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1), Constraint::Length(1)])
            .split(frame.area());

        frame.render_widget(Clear, layout[0]);

        if self.loading {
            let spinner = Paragraph::new(" Loading... ")
                .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
            frame.render_widget(spinner, layout[0]);
            return;
        }

        match self.screen {
            Screen::Dashboard => self.dashboard.draw(frame, layout[0]),
            Screen::Issues => self.draw_issues(frame, layout[0]),
            Screen::PullRequests => {
                let comments = self.selected_pr.as_ref().and_then(|pr| self.pr_comments.get(&pr.number));
                self.prs_view
                    .draw(frame, layout[0], self.selected_pr.as_ref(), comments.map(|v| v.as_slice()));
            }
            Screen::Notes => self
                .notes_view
                .draw(frame, layout[0], self.selected_note.as_ref()),
            Screen::Stats => self.stats_view.draw(frame, layout[0]),
            Screen::Roadmap => self.roadmap_view.draw(frame, layout[0]),
        }

        let repo_info = self.repo_owner.as_ref().map(|o| {
            format!("{}/{}", o, self.repo_name.as_deref().unwrap_or("?"))
        });

        let fetch_info = self.last_fetched.map(|t| {
            let secs = t.elapsed().as_secs();
            if secs < 60 {
                format!(" [{}s ago]", secs)
            } else {
                format!(" [{}m ago]", secs / 60)
            }
        }).unwrap_or_default();

        let status = match &self.input_mode {
            InputMode::Search { query } => format!("/{query}"),
            InputMode::LinkNote { input } => format!("Link to issue #: {input}"),
            _ => format!("{}{}", self.status, fetch_info),
        };
        status_bar(frame, layout[1], &status, repo_info.as_deref());

        let (screen_str, input_str) = match &self.input_mode {
            InputMode::None => (
                match self.screen {
                    Screen::Dashboard => "dashboard",
                    Screen::Issues => "issues",
                    Screen::PullRequests => "prs",
                    Screen::Notes => "notes",
                    Screen::Stats => "stats",
                    Screen::Roadmap => "roadmap",
                },
                "none",
            ),
            InputMode::EditIssue { .. } | InputMode::EditNote { .. } => ("issues", "edit"),
            _ => ("", ""),
        };
        crate::ui::keybinds_bar(frame, layout[2], screen_str, input_str);

        match &self.input_mode {
            InputMode::None => {}
            InputMode::Search { .. } => {}
            InputMode::BrowseProject { browser } => {
                browser.draw(frame, frame.area());
            }
            InputMode::CreateNote { title, body, step } => {
                let (prompt, value, help) = match step {
                    0 => ("Note title", title.as_str(), "enter to confirm, esc to cancel"),
                    _ => ("Note body (optional)", body.as_str(), "enter to submit, esc to skip"),
                };
                popup::input_dialog(frame, frame.area(), prompt, value, help);
            }
            InputMode::Comment { body } => {
                popup::input_dialog(frame, frame.area(), "Comment", body, "enter to post, esc to cancel");
            }
            InputMode::ConfirmMerge { selected, .. } => {
                popup::merge_dialog(frame, frame.area(), *selected as usize);
            }
            InputMode::CreatePr { title, body, step } => {
                let (prompt, value, help) = match step {
                    0 => ("PR title", title.as_str(), "enter to confirm, esc to cancel"),
                    _ => ("PR body (optional)", body.as_str(), "enter to submit, esc to skip body"),
                };
                popup::input_dialog(frame, frame.area(), prompt, value, help);
            }
            InputMode::LinkNote { input } => {
                popup::input_dialog(frame, frame.area(), "Link note to issue #", input, "enter to link, esc to cancel");
            }
            InputMode::CreateRepo { name, private: _, step } => {
                let (prompt, help) = match step {
                    0 => ("Repo name", "enter to confirm, esc to cancel"),
                    _ => ("", ""),
                };
                popup::input_dialog(frame, frame.area(), prompt, name, help);
            }
            InputMode::EditIssue { .. } | InputMode::EditNote { .. } => {}
        }

        if self.show_help {
            self.draw_help(frame);
        }
    }

    fn draw_help(&self, frame: &mut Frame) {
        let screen_help = match self.screen {
            Screen::Dashboard => vec![
                "j/k         navigate projects",
                "Enter       open project",
                "a           add project",
                "d           delete project",
                "s           stats screen",
                "t           roadmap screen",
                "q           quit",
            ],
            Screen::Issues => vec![
                "j/k         navigate issues",
                "Enter       open detail",
                "Tab         switch focus (issues/notes)",
                "/           search",
                "f           cycle filter (open/all/closed)",
                "S           cycle sort order",
                "l           cycle label filter",
                "c           create issue",
                "e           edit issue",
                "x           toggle open/closed",
                "o           add comment",
                "n           new linked note",
                "L           link note to issue",
                "d           delete linked note",
                "O           open in browser",
                "p           pull requests screen",
                "s           stats screen",
                "t           roadmap screen",
                "r           refresh",
                "Ctrl+d/u    scroll detail",
                "mouse wheel scroll detail",
                "g           dashboard",
            ],
            Screen::PullRequests => vec![
                "j/k         navigate PRs",
                "Enter       open detail",
                "/           search",
                "o           add comment",
                "O           open in browser",
                "m           merge PR",
                "c           create PR",
                "i           issues screen",
                "s           stats screen",
                "t           roadmap screen",
                "r           refresh",
                "g           dashboard",
            ],
            Screen::Notes => vec![
                "j/k         navigate notes",
                "Enter       open detail",
                "/           search",
                "n           create note",
                "E           edit note",
                "x           toggle open/closed",
                "d           delete note",
                "g           dashboard",
            ],
            Screen::Stats => vec![
                "j/k         navigate",
                "q           back",
                "g           dashboard",
            ],
            Screen::Roadmap => vec![
                "h/j/k/l     navigate",
                "q           back",
                "g           dashboard",
            ],
        };

        let width = 52usize;
        let mut lines = vec![
            Line::from(Span::styled(
                format!(" {:^width$} ", " Help ", width = width - 2),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::raw("")),
        ];
        for line in screen_help {
            let padded = format!(" {:<width$}", line, width = width - 1);
            lines.push(Line::from(Span::raw(padded)));
        }
        lines.push(Line::from(Span::raw("")));
        lines.push(Line::from(Span::styled(
            " Press any key to close ",
            Style::default().fg(Color::DarkGray),
        )));

        let block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan))
            .title(" Keybindings ");
        let para = Paragraph::new(lines).block(block);
        let area = centered_rect(60, 50, frame.area());
        frame.render_widget(Clear, area);
        frame.render_widget(para, area);
    }

    fn draw_issues(&mut self, frame: &mut Frame, area: Rect) {
        let editing = match &self.input_mode {
            InputMode::EditIssue { title, body, focus, issue_number, labels, available_labels, label_idx } => {
                Some(crate::ui::issues::EditState {
                    title: title.clone(),
                    body: body.clone(),
                    field_focus: *focus,
                    issue_number: *issue_number,
                    note_slug: None,
                    labels: labels.clone(),
                    available_labels: available_labels.clone(),
                    label_idx: *label_idx,
                })
            }
            InputMode::EditNote { slug: _, title, body, focus } => {
                Some(crate::ui::issues::EditState {
                    title: title.clone(),
                    body: body.clone(),
                    field_focus: *focus,
                    issue_number: 0,
                    note_slug: Some(String::new()),
                    labels: Vec::new(),
                    available_labels: Vec::new(),
                    label_idx: 0,
                })
            }
            _ => None,
        };
        let comments = self.selected_issue.as_ref().and_then(|i| self.issue_comments.get(&i.number));
        let detail_issue = if self.detail_target == DetailTarget::Issue {
            self.selected_issue.as_ref()
        } else {
            None
        };
        self.issues_view.draw(
            frame,
            area,
            detail_issue,
            comments.map(|v| v.as_slice()),
            &self.project_notes,
            self.selected_note_idx,
            self.focus,
            editing.as_ref(),
            self.detail_scroll,
        );
    }

    fn spawn_load_issues(&self) {
        if let (Some(owner), Some(repo)) = (self.repo_owner.clone(), self.repo_name.clone()) {
            if let Some(cached) = self.cache.get_issues(&owner, &repo) {
                let filtered: Vec<Issue> = if self.state_filter == "all" {
                    cached
                } else {
                    cached.into_iter().filter(|i| i.state == self.state_filter).collect()
                };
                if !filtered.is_empty() {
                    let _ = self.cmd_tx.send(BgEvent::IssuesReady(filtered));
                    return;
                }
            }

            let filter = if self.state_filter == "all" { None } else { Some(self.state_filter.clone()) };
            let tx = self.cmd_tx.clone();
            let client = self.client.clone();
            let cache = self.cache.clone();
            let owner_c = owner.clone();
            let repo_c = repo.clone();
            tokio::spawn(async move {
                match client.list_issues(&owner, &repo, filter.as_deref()).await {
                    Ok(issues) => {
                        cache.set_issues(&owner_c, &repo_c, &issues);
                        let _ = tx.send(BgEvent::IssuesReady(issues));
                    }
                    Err(e) => { let _ = tx.send(BgEvent::Error(format!("{e}"))); }
                }
            });
        }
    }

    fn spawn_load_prs(&self) {
        if let (Some(owner), Some(repo)) = (self.repo_owner.clone(), self.repo_name.clone()) {
            if let Some(cached) = self.cache.get_prs(&owner, &repo) {
                let _ = self.cmd_tx.send(BgEvent::PRsReady(cached));
                return;
            }

            let tx = self.cmd_tx.clone();
            let client = self.client.clone();
            let cache = self.cache.clone();
            let owner_c = owner.clone();
            let repo_c = repo.clone();
            tokio::spawn(async move {
                match client.list_pull_requests(&owner, &repo, Some("open")).await {
                    Ok(prs) => {
                        cache.set_prs(&owner_c, &repo_c, &prs);
                        let _ = tx.send(BgEvent::PRsReady(prs));
                    }
                    Err(e) => { let _ = tx.send(BgEvent::Error(format!("{e}"))); }
                }
            });
        }
    }

    fn spawn_load_comments(&self, number: u64) {
        if let (Some(owner), Some(repo)) = (self.repo_owner.clone(), self.repo_name.clone()) {
            if let Some(cached) = self.cache.get_comments(&owner, &repo, number) {
                let _ = self.cmd_tx.send(BgEvent::CommentsReady(number, cached));
                return;
            }

            let tx = self.cmd_tx.clone();
            let client = self.client.clone();
            let cache = self.cache.clone();
            let owner_c = owner.clone();
            let repo_c = repo.clone();
            tokio::spawn(async move {
                match client.list_comments(&owner, &repo, number).await {
                    Ok(comments) => {
                        cache.set_comments(&owner_c, &repo_c, number, &comments);
                        let _ = tx.send(BgEvent::CommentsReady(number, comments));
                    }
                    Err(e) => { let _ = tx.send(BgEvent::Error(format!("{e}"))); }
                }
            });
        }
    }

    #[allow(dead_code)]
    fn spawn_load_labels(&self) {
        if let (Some(owner), Some(repo)) = (self.repo_owner.clone(), self.repo_name.clone()) {
            let tx = self.cmd_tx.clone();
            let client = self.client.clone();
            tokio::spawn(async move {
                match client.list_labels(&owner, &repo).await {
                    Ok(labels) => { let _ = tx.send(BgEvent::LabelsReady(labels)); }
                    Err(e) => { let _ = tx.send(BgEvent::Error(format!("{e}"))); }
                }
            });
        }
    }

    async fn handle_events(&mut self) -> Result<()> {
        if event::poll(Duration::from_millis(16))? {
            match event::read()? {
                Event::Key(key) => { self.handle_key(key).await?; }
                Event::Mouse(mouse) => { self.handle_mouse(mouse).await?; }
                Event::Resize(_, _) => {}
                _ => {}
            }
            self.mark_dirty();

            // debounced search: apply filter only after 80ms idle
            if matches!(&self.input_mode, InputMode::Search { .. }) {
                if let Some(t) = self.last_search_press {
                    if t.elapsed() >= Duration::from_millis(80) {
                        self.apply_search_filter();
                        self.last_search_press = None;
                    }
                }
            }
        }
        Ok(())
    }

    fn screen_area(&self) -> Result<Rect> {
        let (cols, rows) = crossterm::terminal::size()?;
        Ok(Rect::new(0, 0, cols, rows.saturating_sub(2)))
    }

    async fn handle_mouse(&mut self, mouse: MouseEvent) -> Result<()> {
        if !self.config.mouse_enabled {
            return Ok(());
        }
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if !matches!(self.input_mode, InputMode::None) {
                    return Ok(());
                }
                match self.screen {
                    Screen::Dashboard => self.mouse_click_dashboard(mouse.column, mouse.row).await?,
                    Screen::Issues => self.mouse_click_issues(mouse.column, mouse.row).await?,
                    Screen::PullRequests => self.mouse_click_prs(mouse.column, mouse.row).await?,
                    Screen::Notes => self.mouse_click_notes(mouse.column, mouse.row).await?,
                    Screen::Stats => self.mouse_click_stats(mouse.column, mouse.row),
                    Screen::Roadmap => self.mouse_click_roadmap(mouse.column, mouse.row),
                }
            }
            MouseEventKind::ScrollUp => {
                self.mouse_scroll_up().await?;
            }
            MouseEventKind::ScrollDown => {
                self.mouse_scroll_down().await?;
            }
            _ => {}
        }
        Ok(())
    }

    fn item_at_row(&self, area: Rect, row: u16) -> Option<usize> {
        let inner_y = area.y + 1;
        if row < inner_y {
            return None;
        }
        let idx = (row - inner_y) as usize;
        if row >= area.y + area.height - 1 {
            return None;
        }
        Some(idx)
    }

    async fn mouse_click_dashboard(&mut self, _col: u16, row: u16) -> Result<()> {
        if self.dashboard.projects.is_empty() {
            return Ok(());
        }
        let area = self.screen_area()?;
        let Some(idx) = self.item_at_row(area, row) else { return Ok(()) };
        if idx >= self.dashboard.projects.len() {
            return Ok(());
        }
        self.dashboard.selected = idx;
        if let Some(project) = self.dashboard.projects.get(self.dashboard.selected) {
            self.repo_owner = Some(project.owner.clone());
            self.repo_name = Some(project.repo.clone());
            self.repo_path = git::project_root_for(std::path::Path::new(&project.path));
            self.stats_view = StatsView::new(&project.owner, &project.repo);
            self.roadmap_view = RoadmapView::new(&project.owner, &project.repo);
            self.status = format!("{}/{}", project.owner, project.repo);
            self.switch_to_issues().await?;
        }
        Ok(())
    }

    async fn mouse_click_issues(&mut self, col: u16, row: u16) -> Result<()> {
        let area = self.screen_area()?;
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(2, 5), Constraint::Ratio(3, 5)])
            .split(area);
        let left_panels = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
            .split(layout[0]);

        // Issues list (top-left)
        if col >= left_panels[0].x && col < left_panels[0].x + left_panels[0].width
            && row >= left_panels[0].y && row < left_panels[0].y + left_panels[0].height
        {
            if let Some(idx) = self.item_at_row(left_panels[0], row) {
                if idx < self.issues_view.issues.len() {
                    self.focus = FocusTarget::Issues;
                    self.detail_target = DetailTarget::Issue;
                    self.issues_view.selected = idx;
                    self.selected_issue = self.issues_view.issues.get(idx).cloned();
                    if let Some(ref issue) = self.selected_issue {
                        if !self.issue_comments.contains_key(&issue.number) {
                            self.load_issue_comments(issue.number).await?;
                        }
                    }
                }
            }
            return Ok(());
        }

        // Notes list (bottom-left)
        if col >= left_panels[1].x && col < left_panels[1].x + left_panels[1].width
            && row >= left_panels[1].y && row < left_panels[1].y + left_panels[1].height
        {
            if let Some(idx) = self.item_at_row(left_panels[1], row) {
                if idx < self.project_notes.len() {
                    self.focus = FocusTarget::Notes;
                    self.detail_target = DetailTarget::Note;
                    self.selected_note_idx = idx;
                }
            }
            return Ok(());
        }

        // Detail panel (right side) — like pressing Enter
        if col >= layout[1].x && col < layout[1].x + layout[1].width
            && row >= layout[1].y && row < layout[1].y + layout[1].height
        {
            match self.focus {
                FocusTarget::Issues => {
                    self.selected_issue = self.issues_view.issues.get(self.issues_view.selected).cloned();
                    self.detail_target = DetailTarget::Issue;
                    if let Some(ref issue) = self.selected_issue {
                        if !self.issue_comments.contains_key(&issue.number) {
                            self.load_issue_comments(issue.number).await?;
                        }
                    }
                }
                FocusTarget::Notes => {
                    self.detail_target = DetailTarget::Note;
                }
            }
        }
        Ok(())
    }

    async fn mouse_click_prs(&mut self, _col: u16, row: u16) -> Result<()> {
        let area = self.screen_area()?;
        let layout = if self.selected_pr.is_some() {
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
        if let Some(idx) = self.item_at_row(layout[0], row) {
            if idx < self.prs_view.prs.len() {
                self.prs_view.selected = idx;
                self.selected_pr = self.prs_view.prs.get(idx).cloned();
                if let Some(ref pr) = self.selected_pr {
                    if !self.pr_comments.contains_key(&pr.number) {
                        self.load_pr_comments(pr.number).await?;
                    }
                }
            }
        }
        Ok(())
    }

    async fn mouse_click_notes(&mut self, _col: u16, row: u16) -> Result<()> {
        let area = self.screen_area()?;
        let layout = if self.selected_note.is_some() {
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
        if let Some(idx) = self.item_at_row(layout[0], row) {
            if idx < self.notes_view.notes.len() {
                self.notes_view.selected = idx;
                self.selected_note = self.notes_view.notes.get(idx).cloned();
            }
        }
        Ok(())
    }

    fn mouse_click_stats(&mut self, _col: u16, row: u16) {
        let area = match self.screen_area() {
            Ok(a) => a,
            _ => return,
        };
        let title_row = area.y;  // title takes 1 line at top
        if row <= title_row {
            return;
        }
        let idx = (row - title_row - 1) as usize;
        if idx < self.stats_view.total_items.len() {
            self.stats_view.selected = idx;
        }
    }

    fn mouse_click_roadmap(&mut self, col: u16, row: u16) {
        let area = match self.screen_area() {
            Ok(a) => a,
            _ => return,
        };
        if self.roadmap_view.groups.is_empty() {
            return;
        }
        let title_height = 1u16; // title paragraph at top
        let list_area_y = area.y + title_height;
        if row < list_area_y {
            return;
        }
        let n = self.roadmap_view.groups.len() as u16;
        let col_width = area.width / n;
        let group_idx = ((col - area.x) / col_width) as usize;
        if group_idx >= self.roadmap_view.groups.len() {
            return;
        }
        self.roadmap_view.selected_group = group_idx;
        let row_idx = (row - list_area_y) as usize;
        let max = self.roadmap_view.groups[group_idx].1.len().saturating_sub(1);
        self.roadmap_view.selected_item = row_idx.min(max);
    }

    async fn mouse_scroll_up(&mut self) -> Result<()> {
        match self.screen {
            Screen::Dashboard => {
                self.dashboard.selected = self.dashboard.selected.saturating_sub(1);
            }
            Screen::Issues => {
                if self.selected_issue.is_some() {
                    self.detail_scroll = self.detail_scroll.saturating_sub(3);
                } else {
                    match self.focus {
                        FocusTarget::Issues => {
                            self.issues_view.selected = self.issues_view.selected.saturating_sub(1);
                            self.selected_issue = self.issues_view.issues.get(self.issues_view.selected).cloned();
                            self.detail_target = DetailTarget::Issue;
                        }
                        FocusTarget::Notes => {
                            self.selected_note_idx = self.selected_note_idx.saturating_sub(1);
                            self.detail_target = DetailTarget::Note;
                        }
                    }
                }
            }
            Screen::PullRequests => {
                self.prs_view.selected = self.prs_view.selected.saturating_sub(1);
                self.selected_pr = self.prs_view.prs.get(self.prs_view.selected).cloned();
            }
            Screen::Notes => {
                self.notes_view.selected = self.notes_view.selected.saturating_sub(1);
                self.selected_note = self.notes_view.notes.get(self.notes_view.selected).cloned();
            }
            Screen::Stats => {
                self.stats_view.selected = self.stats_view.selected.saturating_sub(1);
            }
            Screen::Roadmap => {
                let group_len = self.roadmap_view.groups.get(self.roadmap_view.selected_group).map(|(_, items)| items.len()).unwrap_or(0);
                self.roadmap_view.selected_item = self.roadmap_view.selected_item.saturating_sub(1).min(group_len.saturating_sub(1));
            }
        }
        Ok(())
    }

    async fn mouse_scroll_down(&mut self) -> Result<()> {
        match self.screen {
            Screen::Dashboard => {
                self.dashboard.selected = (self.dashboard.selected + 1)
                    .min(self.dashboard.projects.len().saturating_sub(1));
            }
            Screen::Issues => {
                if self.selected_issue.is_some() {
                    self.detail_scroll = self.detail_scroll.saturating_add(3);
                } else {
                    match self.focus {
                        FocusTarget::Issues => {
                            self.issues_view.selected = (self.issues_view.selected + 1)
                                .min(self.issues_view.issues.len().saturating_sub(1));
                            self.selected_issue = self.issues_view.issues.get(self.issues_view.selected).cloned();
                            self.detail_target = DetailTarget::Issue;
                        },
                        FocusTarget::Notes => {
                        self.selected_note_idx = (self.selected_note_idx + 1)
                            .min(self.project_notes.len().saturating_sub(1));
                        self.detail_target = DetailTarget::Note;
                    }
                }
                }
            }
            Screen::PullRequests => {
                self.prs_view.selected = (self.prs_view.selected + 1)
                    .min(self.prs_view.prs.len().saturating_sub(1));
                self.selected_pr = self.prs_view.prs.get(self.prs_view.selected).cloned();
            }
            Screen::Notes => {
                self.notes_view.selected = (self.notes_view.selected + 1)
                    .min(self.notes_view.notes.len().saturating_sub(1));
                self.selected_note = self.notes_view.notes.get(self.notes_view.selected).cloned();
            }
            Screen::Stats => {
                self.stats_view.selected = (self.stats_view.selected + 1)
                    .min(self.stats_view.total_items.len().saturating_sub(1));
            }
            Screen::Roadmap => {
                let group_len = self.roadmap_view.groups.get(self.roadmap_view.selected_group).map(|(_, items)| items.len()).unwrap_or(0);
                self.roadmap_view.selected_item = (self.roadmap_view.selected_item + 1)
                    .min(group_len.saturating_sub(1));
            }
        }
        Ok(())
    }

    async fn handle_key(&mut self, key: event::KeyEvent) -> Result<()> {
        if self.show_help {
            self.show_help = false;
            return Ok(());
        }
        if key.code == KeyCode::Char('?') {
            self.show_help = true;
            return Ok(());
        }
        match &self.input_mode {
            InputMode::EditIssue { .. } | InputMode::EditNote { .. } => self.handle_edit_key(key).await?,
            _ => match &self.input_mode {
                InputMode::None => match self.screen {
                    Screen::Dashboard => self.handle_dashboard_key(key).await?,
                    Screen::Issues => self.handle_issues_key(key).await?,
                    Screen::PullRequests => self.handle_prs_key(key).await?,
                    Screen::Notes => self.handle_notes_key(key).await?,
                    Screen::Stats => self.handle_stats_key(key).await?,
                    Screen::Roadmap => self.handle_roadmap_key(key).await?,
                },
                InputMode::ConfirmMerge { .. } => self.handle_merge_key(key).await?,
                InputMode::BrowseProject { .. } => self.handle_browse_key(key).await?,
                InputMode::LinkNote { .. } => self.handle_link_key(key).await?,
                InputMode::Search { .. }
                | InputMode::CreateNote { .. }
                | InputMode::CreatePr { .. }
                | InputMode::Comment { .. } => self.handle_input_key(key).await?,
                InputMode::CreateRepo { .. } => self.handle_create_repo_key(key).await?,
                InputMode::EditIssue { .. } | InputMode::EditNote { .. } => unreachable!(),
            },
        }
        Ok(())
    }

    async fn handle_edit_key(&mut self, key: event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::None;
                self.status = "edit cancelled".to_string();
            }
            KeyCode::Tab => {
                match &mut self.input_mode {
                    InputMode::EditIssue { focus, .. } => {
                        *focus = (*focus + 1) % 3;
                    }
                    InputMode::EditNote { focus, .. } => {
                        *focus = (*focus + 1) % 2;
                    }
                    _ => {}
                }
            }
            KeyCode::Enter => {
                let is_title_focused = matches!(
                    &self.input_mode,
                    InputMode::EditIssue { focus: 0, .. } | InputMode::EditNote { focus: 0, .. }
                );
                if is_title_focused {
                    match &mut self.input_mode {
                        InputMode::EditIssue { focus, .. }
                        | InputMode::EditNote { focus, .. } => {
                            *focus = 1;
                        }
                        _ => {}
                    }
                } else {
                    match &mut self.input_mode {
                        InputMode::EditIssue { body, .. }
                        | InputMode::EditNote { body, .. } => {
                            body.push('\n');
                        }
                        _ => {}
                    }
                }
            }
            KeyCode::Backspace => {
                match &mut self.input_mode {
                    InputMode::EditIssue { title, focus: 0, .. }
                    | InputMode::EditNote { title, focus: 0, .. } => {
                        title.pop();
                    }
                    InputMode::EditIssue { body, focus: 1, .. }
                    | InputMode::EditNote { body, focus: 1, .. } => {
                        body.pop();
                    }
                    _ => {}
                }
            }
            KeyCode::Down => {
                if let InputMode::EditIssue { focus: 2, label_idx, available_labels, .. } = &mut self.input_mode {
                    *label_idx = (*label_idx + 1).min(available_labels.len().saturating_sub(1));
                }
            }
            KeyCode::Up => {
                if let InputMode::EditIssue { focus: 2, label_idx, .. } = &mut self.input_mode {
                    *label_idx = label_idx.saturating_sub(1);
                }
            }
            KeyCode::Char('j') => {
                match &mut self.input_mode {
                    InputMode::EditIssue { focus: 2, label_idx, available_labels, .. } => {
                        *label_idx = (*label_idx + 1).min(available_labels.len().saturating_sub(1));
                    }
                    InputMode::EditIssue { title, focus: 0, .. }
                    | InputMode::EditNote { title, focus: 0, .. } => {
                        title.push('j');
                    }
                    InputMode::EditIssue { body, focus: 1, .. }
                    | InputMode::EditNote { body, focus: 1, .. } => {
                        body.push('j');
                    }
                    _ => {}
                }
            }
            KeyCode::Char('k') => {
                match &mut self.input_mode {
                    InputMode::EditIssue { focus: 2, label_idx, .. } => {
                        *label_idx = label_idx.saturating_sub(1);
                    }
                    InputMode::EditIssue { title, focus: 0, .. }
                    | InputMode::EditNote { title, focus: 0, .. } => {
                        title.push('k');
                    }
                    InputMode::EditIssue { body, focus: 1, .. }
                    | InputMode::EditNote { body, focus: 1, .. } => {
                        body.push('k');
                    }
                    _ => {}
                }
            }
            KeyCode::Char(' ') => {
                match &mut self.input_mode {
                    InputMode::EditIssue { focus: 2, label_idx, labels, available_labels, .. } => {
                        if let Some(name) = available_labels.get(*label_idx).cloned() {
                            if let Some(pos) = labels.iter().position(|l| l == &name) {
                                labels.remove(pos);
                            } else {
                                labels.push(name);
                            }
                        }
                    }
                    InputMode::EditIssue { title, focus: 0, .. } | InputMode::EditNote { title, focus: 0, .. } => {
                        title.push(' ');
                    }
                    InputMode::EditIssue { body, focus: 1, .. } | InputMode::EditNote { body, focus: 1, .. } => {
                        body.push(' ');
                    }
                    _ => {}
                }
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let mode = std::mem::replace(&mut self.input_mode, InputMode::None);
                match mode {
                    InputMode::EditIssue { title, body, issue_number, labels, .. } => {
                        let body_opt = if body.is_empty() { None } else { Some(body.as_str()) };
                        if issue_number == 0 {
                            self.create_issue(&title, body_opt, &labels).await?;
                        } else {
                            self.update_issue(issue_number, &title, body_opt, &labels).await?;
                        }
                    }
                    InputMode::EditNote { slug, title, body, .. } => {
                        if let Some(s) = slug {
                            self.update_note_local(&s, &title, if body.is_empty() { None } else { Some(&body) }).await?;
                        } else {
                            self.create_note_local(&title, if body.is_empty() { None } else { Some(&body) }).await?;
                        }
                        self.detail_target = DetailTarget::Note;
                        self.focus = FocusTarget::Notes;
                        self.selected_note_idx = 0;
                    }
                    _ => {
                        self.input_mode = InputMode::None;
                    }
                }
            }
            KeyCode::Char(c) => {
                match &mut self.input_mode {
                    InputMode::EditIssue { title, focus: 0, .. }
                    | InputMode::EditNote { title, focus: 0, .. } => {
                        title.push(c);
                    }
                    InputMode::EditIssue { body, focus: 1, .. }
                    | InputMode::EditNote { body, focus: 1, .. } => {
                        body.push(c);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_input_key(&mut self, key: event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                let was_search = matches!(self.input_mode, InputMode::Search { .. });
                self.input_mode = InputMode::None;
                self.last_search_press = None;
                if was_search {
                    self.restore_filter();
                }
            }
            KeyCode::Enter => {
                let mode = std::mem::replace(&mut self.input_mode, InputMode::None);
                self.last_search_press = None;
                match mode {
                    InputMode::Search { query } => {
                        self.status = format!("filtered: \"{query}\"");
                    }
                    InputMode::Comment { body } => {
                        self.add_comment(&body).await?;
                    }
                    InputMode::CreateNote { title, body: _, step: 0 } => {
                        if !title.is_empty() {
                            self.input_mode = InputMode::CreateNote {
                                title,
                                body: String::new(),
                                step: 1,
                            };
                        } else {
                            self.status = "title cannot be empty".to_string();
                            self.input_mode = InputMode::None;
                        }
                    }
                    InputMode::CreateNote { title, body, .. } => {
                        self.create_note_local(&title, Some(&body)).await?;
                    }
                    InputMode::CreatePr { title, body: _, step: 0 } => {
                        if !title.is_empty() {
                            self.input_mode = InputMode::CreatePr {
                                title,
                                body: String::new(),
                                step: 1,
                            };
                        } else {
                            self.status = "title cannot be empty".to_string();
                            self.input_mode = InputMode::None;
                        }
                    }
                    InputMode::CreatePr { title, body, .. } => {
                        self.create_pr(&title, Some(&body)).await?;
                    }
                    InputMode::None | InputMode::ConfirmMerge { .. } | InputMode::BrowseProject { .. } => {}
                    InputMode::CreateRepo { .. } | InputMode::LinkNote { .. } | InputMode::EditIssue { .. } | InputMode::EditNote { .. } => unreachable!(),
                }
            }
            KeyCode::Backspace => {
                if let Some(active) = self.active_input_mut() {
                    active.pop();
                }
            }
            KeyCode::Char(c) => {
                if let Some(active) = self.active_input_mut() {
                    active.push(c);
                }
            }
            _ => {}
        }

        if matches!(&self.input_mode, InputMode::Search { .. }) {
            self.last_search_press = Some(Instant::now());
        }

        Ok(())
    }

    fn active_input_mut(&mut self) -> Option<&mut String> {
        match &mut self.input_mode {
            InputMode::Search { query } => Some(query),
            InputMode::CreateNote { title, step: 0, .. } => Some(title),
            InputMode::CreateNote { body, step: 1, .. } => Some(body),
            InputMode::Comment { body } => Some(body),
            InputMode::CreatePr { title, step: 0, .. } => Some(title),
            InputMode::CreatePr { body, step: 1, .. } => Some(body),
            InputMode::CreateRepo { name, step: 0, .. } => Some(name),
            InputMode::None | InputMode::ConfirmMerge { .. } | InputMode::BrowseProject { .. } => None,
            InputMode::CreateNote { .. } => None,
            InputMode::CreatePr { .. } | InputMode::LinkNote { .. } | InputMode::EditIssue { .. } | InputMode::EditNote { .. } => None,
            InputMode::CreateRepo { .. } => None,
        }
    }

    // --- Dashboard key handlers ---
    async fn handle_dashboard_key(&mut self, key: event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('j') | KeyCode::Down => {
                self.dashboard.selected = (self.dashboard.selected + 1)
                    .min(self.dashboard.projects.len().saturating_sub(1));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.dashboard.selected = self.dashboard.selected.saturating_sub(1);
            }
            KeyCode::Enter => {
                if let Some(project) = self.dashboard.projects.get(self.dashboard.selected) {
                    self.repo_owner = Some(project.owner.clone());
                    self.repo_name = Some(project.repo.clone());
                    self.repo_path = git::project_root_for(std::path::Path::new(&project.path));
                    self.stats_view = StatsView::new(&project.owner, &project.repo);
                    self.roadmap_view = RoadmapView::new(&project.owner, &project.repo);
                    self.status = format!("{}/{}", project.owner, project.repo);
                    self.switch_to_issues().await?;
                }
            }
            KeyCode::Char('a') => {
                let start = std::env::current_dir().unwrap_or_else(|_| dirs::home_dir().unwrap_or_default());
                let browser = FileBrowser::new(&start);
                self.input_mode = InputMode::BrowseProject { browser };
            }
            KeyCode::Char('n') => {
                self.switch_to_notes().await?;
            }
            KeyCode::Char('s') => {
                self.show_stats().await?;
            }
            KeyCode::Char('t') => {
                self.show_roadmap().await?;
            }
            KeyCode::Char('c') => {
                if self.repo_path.is_some() && self.repo_owner.is_none() {
                    self.input_mode = InputMode::CreateRepo {
                        name: self.repo_path.as_ref()
                            .and_then(|p| p.file_name())
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default(),
                        private: false,
                        step: 0,
                    };
                }
            }
            KeyCode::Char('d') => {
                if !self.dashboard.projects.is_empty() {
                    let idx = self.dashboard.selected;
                    self.dashboard.projects.remove(idx);
                    self.dashboard.selected = self
                        .dashboard
                        .selected
                        .min(self.dashboard.projects.len().saturating_sub(1));
                    self.config.projects = self.dashboard.projects.clone();
                    config::save(&self.config)?;
                    self.status = "project deleted".to_string();
                }
            }
            _ => {}
        }
        Ok(())
    }

    // --- Issues key handlers ---
    async fn handle_issues_key(&mut self, key: event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('q') => self.go_to_dashboard(),
            KeyCode::Char('g') => self.go_to_dashboard(),
            KeyCode::Char('Q') => self.should_quit = true,
            KeyCode::Tab => {
                self.focus = match self.focus {
                    FocusTarget::Issues => FocusTarget::Notes,
                    FocusTarget::Notes => FocusTarget::Issues,
                };
                self.status = match self.focus {
                    FocusTarget::Issues => "focus: issues".to_string(),
                    FocusTarget::Notes => "focus: notes".to_string(),
                };
            }
            KeyCode::Char('j') | KeyCode::Down => {
                match self.focus {
                    FocusTarget::Issues => {
                        self.issues_view.selected = (self.issues_view.selected + 1)
                            .min(self.issues_view.issues.len().saturating_sub(1));
                        self.detail_scroll = 0;
                        self.selected_issue = self
                            .issues_view
                            .issues
                            .get(self.issues_view.selected)
                            .cloned();
                        self.detail_target = DetailTarget::Issue;
                    }
                    FocusTarget::Notes => {
                        self.selected_note_idx = (self.selected_note_idx + 1)
                            .min(self.project_notes.len().saturating_sub(1));
                        self.detail_target = DetailTarget::Note;
                    }
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                match self.focus {
                    FocusTarget::Issues => {
                        self.issues_view.selected = self.issues_view.selected.saturating_sub(1);
                        self.detail_scroll = 0;
                        self.selected_issue = self
                            .issues_view
                            .issues
                            .get(self.issues_view.selected)
                            .cloned();
                        self.detail_target = DetailTarget::Issue;
                    }
                    FocusTarget::Notes => {
                        self.selected_note_idx = self.selected_note_idx.saturating_sub(1);
                        self.detail_target = DetailTarget::Note;
                    }
                }
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.selected_issue.is_some() {
                    self.detail_scroll = self.detail_scroll.saturating_add(5);
                    self.mark_dirty();
                }
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.selected_issue.is_some() {
                    self.detail_scroll = self.detail_scroll.saturating_sub(5);
                    self.mark_dirty();
                }
            }
            KeyCode::Enter => {
                match self.focus {
                    FocusTarget::Issues => {
                        self.detail_scroll = 0;
                        self.selected_issue = self
                            .issues_view
                            .issues
                            .get(self.issues_view.selected)
                            .cloned();
                        self.detail_target = DetailTarget::Issue;
                        if let Some(ref issue) = self.selected_issue {
                            if !self.issue_comments.contains_key(&issue.number) {
                                self.load_issue_comments(issue.number).await?;
                            }
                        }
                    }
                    FocusTarget::Notes => {
                        self.detail_target = DetailTarget::Note;
                    }
                }
            }
            KeyCode::Char('/') => {
                self.input_mode = InputMode::Search {
                    query: String::new(),
                };
            }
            KeyCode::Char('c') => {
                let labels = if self.repo_owner.is_some() && self.repo_name.is_some() {
                    self.client
                        .list_labels(self.repo_owner.as_deref().unwrap(), self.repo_name.as_deref().unwrap())
                        .await
                        .unwrap_or_default()
                } else {
                    Vec::new()
                };
                self.input_mode = InputMode::EditIssue {
                    title: String::new(),
                    body: String::new(),
                    focus: 0,
                    issue_number: 0,
                    labels: Vec::new(),
                    available_labels: labels,
                    label_idx: 0,
                };
                self.status = "creating issue (Tab switch, Ctrl+S save)".to_string();
            }
            KeyCode::Char('x') => {
                match self.focus {
                    FocusTarget::Issues => {
                        self.toggle_issue_state().await?;
                    }
                    FocusTarget::Notes => {
                        self.toggle_note_state().await?;
                    }
                }
            }
            KeyCode::Char('e') => {
                if self.focus == FocusTarget::Issues {
                    if let Some(ref issue) = self.selected_issue {
                        let labels = if self.repo_owner.is_some() && self.repo_name.is_some() {
                            self.client
                                .list_labels(self.repo_owner.as_deref().unwrap(), self.repo_name.as_deref().unwrap())
                                .await
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        };
                        self.input_mode = InputMode::EditIssue {
                            title: issue.title.clone(),
                            body: issue.body.clone().unwrap_or_default(),
                            focus: 0,
                            issue_number: issue.number,
                            labels: issue.labels.clone(),
                            available_labels: labels,
                            label_idx: 0,
                        };
                        self.status = "editing issue (Tab to switch field, Ctrl+S save)".to_string();
                    }
                }
            }
            KeyCode::Char('o') => {
                if self.focus == FocusTarget::Issues && self.selected_issue.is_some() {
                    self.input_mode = InputMode::Comment {
                        body: String::new(),
                    };
                }
            }
            KeyCode::Char('O') => {
                if let (Some(owner), Some(repo), Some(ref issue)) = (&self.repo_owner, &self.repo_name, self.selected_issue.clone()) {
                    let url = format!("https://github.com/{owner}/{repo}/issues/{}", issue.number);
                    let _ = webbrowser::open(&url);
                    self.status = format!("opened: {url}");
                }
            }
            KeyCode::Char('n') => {
                self.detail_target = DetailTarget::Note;
                self.focus = FocusTarget::Notes;
                self.input_mode =             InputMode::EditNote {
                    slug: None,
                    title: String::new(),
                    body: String::new(),
                    focus: 0,
                };
                self.status = "new note (Tab switch, Ctrl+S save)".to_string();
            }
            KeyCode::Char('L') => {
                if self.focus == FocusTarget::Notes && !self.project_notes.is_empty() {
                    self.input_mode = InputMode::LinkNote {
                        input: String::new(),
                    };
                }
            }
            KeyCode::Char('d') => {
                if self.focus == FocusTarget::Notes {
                    self.delete_selected_note().await?;
                }
            }
            KeyCode::Char('p') => {
                self.switch_to_prs().await?;
            }
            KeyCode::Char('s') => {
                self.show_stats().await?;
            }
            KeyCode::Char('t') => {
                self.show_roadmap().await?;
            }
            KeyCode::Char('f') => {
                self.state_filter = match self.state_filter.as_str() {
                    "open" => "all".to_string(),
                    "all" => "closed".to_string(),
                    _ => "open".to_string(),
                };
                self.config.default_filter = Some(self.state_filter.clone());
                let _ = config::save(&self.config);
                self.status = format!("filter: {}", self.state_filter);
                self.refresh_issues().await?;
            }
            KeyCode::Char('l') => {
                if self.available_labels.is_empty() {
                    if let (Some(owner), Some(repo)) = (&self.repo_owner, &self.repo_name) {
                        self.available_labels = self.client.list_labels(owner, repo).await.unwrap_or_default();
                    }
                }
                if self.available_labels.is_empty() {
                    self.status = "no labels available".to_string();
                } else {
                    let current = self.label_filter.as_deref().unwrap_or("");
                    let idx = self.available_labels.iter().position(|l| l == current).map(|i| i + 1).unwrap_or(0);
                    if idx >= self.available_labels.len() {
                        self.label_filter = None;
                        self.status = "label filter: none".to_string();
                    } else {
                        self.label_filter = Some(self.available_labels[idx].clone());
                        self.status = format!("label filter: {}", self.available_labels[idx]);
                    }
                    self.issues_view.issues = self.apply_label_filter();
                    self.issues_view.selected = self.issues_view.selected.min(self.issues_view.issues.len().saturating_sub(1));
                    self.selected_issue = self.issues_view.issues.get(self.issues_view.selected).cloned();
                }
            }
            KeyCode::Char('S') => {
                self.sort_mode = match self.sort_mode {
                    SortMode::CreatedNewest => { self.status = "sort: created (oldest)".to_string(); SortMode::CreatedOldest }
                    SortMode::CreatedOldest => { self.status = "sort: updated".to_string(); SortMode::Updated }
                    SortMode::Updated => { self.status = "sort: number".to_string(); SortMode::Number }
                    SortMode::Number => { self.status = "sort: state".to_string(); SortMode::State }
                    SortMode::State => { self.status = "sort: created (newest)".to_string(); SortMode::CreatedNewest }
                };
                self.apply_sort();
                self.mark_dirty();
            }
            KeyCode::Char('r') => {
                self.refresh_issues().await?;
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_prs_key(&mut self, key: event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('q') => self.go_to_dashboard(),
            KeyCode::Char('g') => self.go_to_dashboard(),
            KeyCode::Char('Q') => self.should_quit = true,
            KeyCode::Char('j') | KeyCode::Down => {
                self.prs_view.selected = (self.prs_view.selected + 1)
                    .min(self.prs_view.prs.len().saturating_sub(1));
                self.selected_pr = self.prs_view.prs.get(self.prs_view.selected).cloned();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.prs_view.selected = self.prs_view.selected.saturating_sub(1);
                self.selected_pr = self.prs_view.prs.get(self.prs_view.selected).cloned();
            }
            KeyCode::Enter => {
                self.selected_pr = self.prs_view.prs.get(self.prs_view.selected).cloned();
                if let Some(ref pr) = self.selected_pr {
                    if !self.pr_comments.contains_key(&pr.number) {
                        self.load_pr_comments(pr.number).await?;
                    }
                }
            }
            KeyCode::Char('/') => {
                self.input_mode = InputMode::Search {
                    query: String::new(),
                };
            }
            KeyCode::Char('c') => {
                self.input_mode = InputMode::CreatePr {
                    title: String::new(),
                    body: String::new(),
                    step: 0,
                };
            }
            KeyCode::Char('m') => {
                if let Some(pr) = &self.selected_pr {
                    if pr.state == "open" {
                        self.input_mode = InputMode::ConfirmMerge {
                            pr_number: pr.number,
                            selected: 0,
                        };
                    }
                }
            }
            KeyCode::Char('o') => {
                if self.selected_pr.is_some() {
                    self.input_mode = InputMode::Comment {
                        body: String::new(),
                    };
                }
            }
            KeyCode::Char('O') => {
                if let (Some(owner), Some(repo), Some(ref pr)) = (&self.repo_owner, &self.repo_name, self.selected_pr.clone()) {
                    let url = format!("https://github.com/{owner}/{repo}/pull/{}", pr.number);
                    let _ = webbrowser::open(&url);
                    self.status = format!("opened: {url}");
                }
            }
            KeyCode::Char('i') => {
                self.switch_to_issues().await?;
            }
            KeyCode::Char('s') => {
                self.show_stats().await?;
            }
            KeyCode::Char('t') => {
                self.show_roadmap().await?;
            }
            KeyCode::Char('r') => {
                self.refresh_prs().await?;
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_merge_key(&mut self, key: event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('1') => {
                if let InputMode::ConfirmMerge { pr_number, .. } =
                    std::mem::replace(&mut self.input_mode, InputMode::None)
                {
                    self.merge_pr(pr_number, "merge").await?;
                }
            }
            KeyCode::Char('2') => {
                if let InputMode::ConfirmMerge { pr_number, .. } =
                    std::mem::replace(&mut self.input_mode, InputMode::None)
                {
                    self.merge_pr(pr_number, "squash").await?;
                }
            }
            KeyCode::Char('3') => {
                if let InputMode::ConfirmMerge { pr_number, .. } =
                    std::mem::replace(&mut self.input_mode, InputMode::None)
                {
                    self.merge_pr(pr_number, "rebase").await?;
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if let InputMode::ConfirmMerge { ref mut selected, .. } = self.input_mode {
                    *selected = (*selected + 1).min(2);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if let InputMode::ConfirmMerge { ref mut selected, .. } = self.input_mode {
                    *selected = selected.saturating_sub(1);
                }
            }
            KeyCode::Esc => {
                self.input_mode = InputMode::None;
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_browse_key(&mut self, key: event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if let InputMode::BrowseProject { ref mut browser } = self.input_mode {
                    browser.selected = (browser.selected + 1).min(browser.entries.len().saturating_sub(1));
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if let InputMode::BrowseProject { ref mut browser } = self.input_mode {
                    browser.selected = browser.selected.saturating_sub(1);
                }
            }
            KeyCode::Enter => {
                if let InputMode::BrowseProject { ref mut browser } = self.input_mode {
                    if browser.selected_is_dir() {
                        if let Some(path) = browser.selected_path() {
                            browser.navigate(&path);
                        }
                    } else {
                        let path = browser.selected_path();
                        let mode = std::mem::replace(&mut self.input_mode, InputMode::None);
                        if let InputMode::BrowseProject { browser: _ } = mode {
                            if let Some(p) = path {
                                let dir = if p.is_dir() { p } else { p.parent().map(|x| x.to_path_buf()).unwrap_or(p) };
                                self.add_project(&dir.to_string_lossy()).await?;
                            }
                        }
                    }
                }
            }
            KeyCode::Char('h') => {
                if let InputMode::BrowseProject { ref mut browser } = self.input_mode {
                    browser.show_hidden = !browser.show_hidden;
                    browser.refresh();
                }
            }
            KeyCode::Esc => {
                if let InputMode::BrowseProject { ref mut browser } = self.input_mode {
                    browser.go_up();
                    if browser.current_dir.parent().is_none() {
                        self.input_mode = InputMode::None;
                    }
                }
            }
            KeyCode::Char('q') => {
                self.input_mode = InputMode::None;
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_notes_key(&mut self, key: event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('q') => {
                self.go_to_dashboard();
                self.selected_note = None;
            }
            KeyCode::Char('g') => {
                self.go_to_dashboard();
                self.selected_note = None;
            }
            KeyCode::Char('Q') => self.should_quit = true,
            KeyCode::Char('j') | KeyCode::Down => {
                self.notes_view.selected = (self.notes_view.selected + 1)
                    .min(self.notes_view.notes.len().saturating_sub(1));
                self.selected_note = self.notes_view.notes.get(self.notes_view.selected).cloned();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.notes_view.selected = self.notes_view.selected.saturating_sub(1);
                self.selected_note = self.notes_view.notes.get(self.notes_view.selected).cloned();
            }
            KeyCode::Enter => {
                self.selected_note = self.notes_view.notes.get(self.notes_view.selected).cloned();
            }
            KeyCode::Char('n') => {
                self.input_mode = InputMode::CreateNote {
                    title: String::new(),
                    body: String::new(),
                    step: 0,
                };
            }
            KeyCode::Char('x') => {
                self.toggle_note_standalone_state().await?;
            }
            KeyCode::Char('E') => {
                if let Some(note) = self.selected_note.as_ref() {
                    self.input_mode = InputMode::EditNote {
                        slug: Some(note.slug.clone()),
                        title: note.title.clone(),
                        body: note.body.clone().unwrap_or_default(),
                        focus: 0,
                    };
                    self.status = "editing note (Tab switch, Ctrl+S save, Esc cancel)".to_string();
                }
            }
            KeyCode::Char('/') => {
                self.input_mode = InputMode::Search {
                    query: String::new(),
                };
            }
            KeyCode::Char('d') => {
                self.delete_standalone_note().await?;
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_link_key(&mut self, key: event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::None;
                self.status = "link cancelled".to_string();
            }
            KeyCode::Enter => {
                let mode = std::mem::replace(&mut self.input_mode, InputMode::None);
                if let InputMode::LinkNote { input } = mode {
                    if let Ok(num) = input.trim().parse::<u64>() {
                        self.link_selected_note(num).await?;
                    } else if input.trim().is_empty() {
                        self.status = "link cancelled".to_string();
                    } else {
                        self.status = "invalid issue number".to_string();
                    }
                }
            }
            KeyCode::Backspace => {
                if let InputMode::LinkNote { ref mut input } = self.input_mode {
                    input.pop();
                }
            }
            KeyCode::Char(c) => {
                if let InputMode::LinkNote { ref mut input } = self.input_mode {
                    input.push(c);
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_create_repo_key(&mut self, key: event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::None;
                self.status = "canceled".to_string();
            }
            KeyCode::Enter => {
                let mode = std::mem::replace(&mut self.input_mode, InputMode::None);
                if let InputMode::CreateRepo { name, private, .. } = mode {
                    if name.trim().is_empty() {
                        self.status = "name cannot be empty".to_string();
                        self.input_mode = InputMode::CreateRepo { name, private, step: 0 };
                        return Ok(());
                    }
                    self.create_github_repo(&name, private).await?;
                }
            }
            KeyCode::Backspace => {
                if let InputMode::CreateRepo { ref mut name, .. } = self.input_mode {
                    name.pop();
                }
            }
            KeyCode::Char(c) => {
                if let InputMode::CreateRepo { ref mut name, .. } = self.input_mode {
                    name.push(c);
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn create_github_repo(&mut self, name: &str, private: bool) -> Result<()> {
        self.status = format!("creating repo {name}...");
        match self.client.create_repo(name, private, Some("created from vex")).await {
            Ok(repo) => {
                let full_name = repo["full_name"].as_str().unwrap_or(name);
                let _html_url = repo["html_url"].as_str().unwrap_or("");
                self.status = format!("created {full_name}");

                if let Some(ref path) = self.repo_path.clone() {
                    let origin = format!("https://github.com/{full_name}.git");
                    let _ = std::process::Command::new("git")
                        .args(["remote", "add", "origin", &origin])
                        .current_dir(path)
                        .output();
                    let _ = std::process::Command::new("git")
                        .args(["push", "-u", "origin", "HEAD"])
                        .current_dir(path)
                        .output();
                    let _ = std::process::Command::new("git")
                        .args(["fetch", "origin"])
                        .current_dir(path)
                        .output();
                }

                self.repo_owner = repo["owner"]["login"].as_str().map(|s| s.to_string());
                self.repo_name = repo["name"].as_str().map(|s| s.to_string());
                if self.repo_owner.is_some() && self.repo_name.is_some() {
                    self.switch_to_issues().await?;
                }
            }
            Err(e) => {
                self.status = format!("error creating repo: {e}");
            }
        }
        Ok(())
    }

    // --- Actions ---

    async fn create_issue(&mut self, title: &str, body: Option<&str>, labels: &[String]) -> Result<()> {
        if let (Some(owner), Some(repo)) = (&self.repo_owner, &self.repo_name) {
            self.status = format!("creating issue...");
            match self
                .client
                .create_issue(owner, repo, title, body, labels)
                .await
            {
                Ok(issue) => {
                    self.status = format!("created issue #{}", issue.number);
                    self.refresh_issues().await?;
                }
                Err(e) => {
                    self.status = format!("error creating issue: {e}");
                }
            }
        }
        Ok(())
    }

    async fn update_issue(&mut self, number: u64, title: &str, body: Option<&str>, labels: &[String]) -> Result<()> {
        // optimistic local update
        for issue in self.all_issues.iter_mut() {
            if issue.number == number {
                issue.title = title.to_string();
                issue.body = body.map(|s| s.to_string());
                issue.labels = labels.to_vec();
            }
        }
        for issue in self.issues_view.issues.iter_mut() {
            if issue.number == number {
                issue.title = title.to_string();
                issue.body = body.map(|s| s.to_string());
                issue.labels = labels.to_vec();
            }
        }
        if let Some(ref mut sel) = self.selected_issue {
            if sel.number == number {
                sel.title = title.to_string();
                sel.body = body.map(|s| s.to_string());
                sel.labels = labels.to_vec();
            }
        }

        if let (Some(owner), Some(repo)) = (&self.repo_owner, &self.repo_name) {
            self.status = format!("updating issue #{number}...");
            match self.client.update_issue(owner, repo, number, title, body, labels).await {
                Ok(_) => {
                    self.status = format!("updated issue #{number}");
                    self.refresh_issues().await?;
                }
                Err(e) => {
                    self.status = format!("error: {e}");
                    self.refresh_issues().await?;
                }
            }
        }
        Ok(())
    }

    async fn toggle_issue_state(&mut self) -> Result<()> {
        let issue_num = self.selected_issue.as_ref().map(|i| i.number);
        let new_state = if self.selected_issue.as_ref().is_some_and(|i| i.state == "open") { "closed" } else { "open" };

        // optimistic local update
        if let Some(num) = issue_num {
            for issue in self.all_issues.iter_mut() {
                if issue.number == num {
                    issue.state = new_state.to_string();
                }
            }
            for issue in self.issues_view.issues.iter_mut() {
                if issue.number == num {
                    issue.state = new_state.to_string();
                }
            }
            if let Some(ref mut sel) = self.selected_issue {
                sel.state = new_state.to_string();
            }
        }

        if let (Some(owner), Some(repo)) = (&self.repo_owner, &self.repo_name) {
            if let Some(num) = issue_num {
                self.status = format!("{} issue #{num}...", new_state);
                match self.client.update_issue_state(owner, repo, num, new_state).await {
                    Ok(_) => {
                        self.status = format!("issue #{num} {new_state}");
                        self.refresh_issues().await?;
                    }
                    Err(e) => {
                        self.status = format!("error: {e}");
                        self.refresh_issues().await?;
                    }
                }
            }
        }
        Ok(())
    }

    async fn add_comment(&mut self, body: &str) -> Result<()> {
        let body = body.trim().to_string();
        if body.is_empty() {
            self.status = "comment cannot be empty".to_string();
            return Ok(());
        }

        if let (Some(owner), Some(repo)) = (&self.repo_owner, &self.repo_name) {
            if let Some(ref issue) = self.selected_issue {
                match self.client.create_comment(owner, repo, issue.number, &body).await {
                    Ok(_) => self.status = "comment posted".to_string(),
                    Err(e) => self.status = format!("error: {e}"),
                }
            } else if let Some(ref pr) = self.selected_pr {
                match self.client.create_comment(owner, repo, pr.number, &body).await {
                    Ok(_) => self.status = "comment posted".to_string(),
                    Err(e) => self.status = format!("error: {e}"),
                }
            }
        }
        Ok(())
    }

    async fn load_issue_comments(&mut self, number: u64) -> Result<()> {
        self.spawn_load_comments(number);
        Ok(())
    }

    async fn load_pr_comments(&mut self, number: u64) -> Result<()> {
        self.spawn_load_comments(number);
        Ok(())
    }

    fn resolve_project_path(&mut self) {
        if self.repo_path.as_ref().map_or(true, |p| !p.exists()) {
            if let (Some(ref owner), Some(ref repo)) = (self.repo_owner.clone(), self.repo_name.clone()) {
                let found = self.config.projects.iter().find(|p| p.owner == *owner && p.repo == *repo);
                if let Some(project) = found {
                    let path = std::path::Path::new(&project.path);
                    if path.exists() {
                        self.repo_path = Some(path.to_path_buf());
                        return;
                    }
                }
            }
            self.repo_path = None;
        }
    }

    // --- Note operations ---

    async fn create_note_local(&mut self, title: &str, body: Option<&str>) -> Result<()> {
        self.resolve_project_path();
        let issue = if self.detail_target == DetailTarget::Issue {
            self.selected_issue.as_ref().map(|i| i.number)
        } else {
            None
        };

        if let Some(ref path) = self.repo_path {
            match notes::create_note(path, title, body, "medium", issue) {
                Ok(note) => {
                    self.status = format!("created note: {}", note.title);
                    self.refresh_project_notes().await?;
                }
                Err(e) => self.status = format!("error creating note: {e}"),
            }
        } else {
            self.status = "no project directory".to_string();
        }
        Ok(())
    }

    async fn update_note_local(&mut self, slug: &str, title: &str, body: Option<&str>) -> Result<()> {
        self.resolve_project_path();
        if let Some(ref path) = self.repo_path {
            match notes::update_note(path, slug, title, body, "medium", "open", None) {
                Ok(_) => {
                    self.status = format!("updated note: {title}");
                    self.refresh_project_notes().await?;
                }
                Err(e) => self.status = format!("error updating note: {e}"),
            }
        } else {
            self.status = "no project directory".to_string();
        }
        Ok(())
    }

    async fn link_selected_note(&mut self, issue_number: u64) -> Result<()> {
        self.resolve_project_path();
        if let Some(ref path) = self.repo_path {
            if let Some(note) = self.project_notes.get(self.selected_note_idx) {
                match notes::update_note(
                    path,
                    &note.slug,
                    &note.title,
                    note.body.as_deref(),
                    &note.priority,
                    &note.status,
                    Some(issue_number),
                ) {
                    Ok(_) => {
                        self.status = format!("linked note to issue #{issue_number}");
                        self.refresh_project_notes().await?;
                    }
                    Err(e) => self.status = format!("error linking note: {e}"),
                }
            }
        }
        Ok(())
    }

    async fn delete_selected_note(&mut self) -> Result<()> {
        self.resolve_project_path();
        if let Some(ref path) = self.repo_path {
            if let Some(note) = self.project_notes.get(self.selected_note_idx) {
                let slug = note.slug.clone();
                match notes::delete_note(path, &slug) {
                    Ok(_) => {
                        self.status = format!("deleted note: {}", note.title);
                        self.selected_note_idx = 0;
                        self.refresh_project_notes().await?;
                    }
                    Err(e) => self.status = format!("error deleting note: {e}"),
                }
            }
        }
        Ok(())
    }

    async fn toggle_note_state(&mut self) -> Result<()> {
        self.resolve_project_path();
        if let Some(ref path) = self.repo_path {
            if let Some(note) = self.project_notes.get(self.selected_note_idx).cloned() {
                let new_status = if note.status == "open" { "closed" } else { "open" };
                match notes::update_note(
                    path,
                    &note.slug,
                    &note.title,
                    note.body.as_deref(),
                    &note.priority,
                    new_status,
                    note.issue,
                ) {
                    Ok(_) => {
                        self.status = format!("note: {}", if new_status == "open" { "reopened" } else { "closed" });
                        self.refresh_project_notes().await?;
                    }
                    Err(e) => self.status = format!("error: {e}"),
                }
            }
        }
        Ok(())
    }

    async fn toggle_note_standalone_state(&mut self) -> Result<()> {
        self.resolve_project_path();
        if let Some(ref path) = self.repo_path {
            if let Some(note) = &self.selected_note {
                let new_status = if note.status == "open" { "closed" } else { "open" };
                match notes::update_note(
                    path,
                    &note.slug,
                    &note.title,
                    note.body.as_deref(),
                    &note.priority,
                    new_status,
                    note.issue,
                ) {
                    Ok(_) => {
                        self.status = format!("note: {}", if new_status == "open" { "reopened" } else { "closed" });
                        self.refresh_notes().await?;
                    }
                    Err(e) => self.status = format!("error: {e}"),
                }
            }
        }
        Ok(())
    }

    async fn delete_standalone_note(&mut self) -> Result<()> {
        self.resolve_project_path();
        if let Some(ref path) = self.repo_path {
            if let Some(note) = &self.selected_note {
                let slug = note.slug.clone();
                match notes::delete_note(path, &slug) {
                    Ok(_) => {
                        self.status = format!("deleted note: {}", note.title);
                        self.selected_note = None;
                        self.refresh_notes().await?;
                    }
                    Err(e) => self.status = format!("error: {e}"),
                }
            }
        }
        Ok(())
    }

    async fn refresh_project_notes(&mut self) -> Result<()> {
        self.resolve_project_path();
        if let Some(ref path) = self.repo_path {
            match notes::list_notes(path) {
                Ok(notes) => {
                    let count = notes.len();
                    self.project_notes = notes;
                    self.selected_note_idx = self.selected_note_idx.min(count.saturating_sub(1));
                    if self.screen == Screen::Issues {
                        self.status = format!("{count} notes");
                    }
                    self.notes_view = NotesView::new(self.project_notes.clone());
                }
                Err(e) => self.status = format!("error: {e}"),
            }
        }
        Ok(())
    }

    async fn refresh_notes(&mut self) -> Result<()> {
        self.resolve_project_path();
        if let Some(ref path) = self.repo_path {
            match notes::list_notes(path) {
                Ok(notes) => {
                    let count = notes.len();
                    self.project_notes = notes.clone();
                    self.notes_view = NotesView::new(notes);
                    self.selected_note = self.notes_view.notes.first().cloned();
                    if self.screen == Screen::Notes {
                        self.status = format!("{count} notes");
                    }
                }
                Err(e) => self.status = format!("error: {e}"),
            }
        }
        Ok(())
    }

    async fn switch_to_notes(&mut self) -> Result<()> {
        self.refresh_notes().await?;
        self.screen = Screen::Notes;
        Ok(())
    }

    async fn merge_pr(&mut self, pr_number: u64, method: &str) -> Result<()> {
        if let (Some(owner), Some(repo)) = (&self.repo_owner, &self.repo_name) {
            self.status = format!("merging PR #{pr_number} ({method})...");
            match self.client.merge_pr(owner, repo, pr_number, method).await {
                Ok(_) => {
                    self.status = format!("PR #{pr_number} merged ({method})");
                    self.refresh_prs().await?;
                }
                Err(e) => self.status = format!("error: {e}"),
            }
        }
        Ok(())
    }

    async fn add_project(&mut self, path_str: &str) -> Result<()> {
        let expanded = if path_str.starts_with("~/") {
            let home = dirs::home_dir().map(|h| h.to_string_lossy().to_string()).unwrap_or_default();
            path_str.replacen("~", &home, 1)
        } else {
            path_str.to_string()
        };
        let p = std::path::Path::new(&expanded);

        if !p.exists() {
            self.status = format!("path does not exist: {path_str}");
            return Ok(());
        }

        match git::detect(p) {
            Ok(Some(info)) => {
                let name = p
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| info.repo.clone());

                let project = config::Project {
                    name,
                    path: p.to_string_lossy().to_string(),
                    owner: info.owner,
                    repo: info.repo,
                };

                self.dashboard.projects.push(project.clone());
                self.config.projects = self.dashboard.projects.clone();
                config::save(&self.config)?;
                self.status = format!("added {}/{}", project.owner, project.repo);
            }
            Ok(None) => {
                self.status = "no GitHub repo found at that path".to_string();
            }
            Err(e) => self.status = format!("error: {e}"),
        }
        Ok(())
    }

    // --- Search ---

    fn apply_search_filter(&mut self) {
        let query = match &self.input_mode {
            InputMode::Search { query } => query.clone(),
            _ => return,
        };

        if query.is_empty() {
            self.restore_filter();
            return;
        }

        match self.screen {
            Screen::Issues => {
                let filtered: Vec<Issue> = self
                    .all_issues
                    .iter()
                    .filter(|i| self.search_matcher.fuzzy_match(&i.title, &query).unwrap_or(0) > 0)
                    .cloned()
                    .collect();
                self.issues_view.issues = filtered;
                self.issues_view.selected = 0;
                self.selected_issue = self.issues_view.issues.first().cloned();
            }
            Screen::PullRequests => {
                let filtered: Vec<PullRequest> = self
                    .all_prs
                    .iter()
                    .filter(|pr| self.search_matcher.fuzzy_match(&pr.title, &query).unwrap_or(0) > 0)
                    .cloned()
                    .collect();
                self.prs_view.prs = filtered;
                self.prs_view.selected = 0;
                self.selected_pr = self.prs_view.prs.first().cloned();
            }
            Screen::Notes => {
                let filtered: Vec<Note> = self
                    .project_notes
                    .iter()
                    .filter(|n| {
                        self.search_matcher.fuzzy_match(&n.title, &query).unwrap_or(0) > 0
                            || n.body.as_ref().is_some_and(|b| {
                                self.search_matcher.fuzzy_match(b, &query).unwrap_or(0) > 0
                            })
                    })
                    .cloned()
                    .collect();
                self.notes_view.notes = filtered;
                self.notes_view.selected = 0;
                self.selected_note = self.notes_view.notes.first().cloned();
            }
            _ => {}
        }
    }

    fn restore_filter(&mut self) {
        self.issues_view.issues = self.all_issues.clone();
        self.prs_view.prs = self.all_prs.clone();
        self.issues_view.selected = self
            .issues_view
            .selected
            .min(self.issues_view.issues.len().saturating_sub(1));
        self.prs_view.selected = self
            .prs_view
            .selected
            .min(self.prs_view.prs.len().saturating_sub(1));
        self.notes_view.notes = self.project_notes.clone();
        self.notes_view.selected = self
            .notes_view
            .selected
            .min(self.notes_view.notes.len().saturating_sub(1));
        self.selected_issue = self
            .issues_view
            .issues
            .get(self.issues_view.selected)
            .cloned();
        self.selected_pr = self
            .prs_view
            .prs
            .get(self.prs_view.selected)
            .cloned();
    }

    // --- Screen switching ---

    async fn switch_to_issues(&mut self) -> Result<()> {
        if self.repo_owner.is_some() && self.repo_name.is_some() {
            self.loading = true;
            self.all_issues.clear();
            self.spawn_load_issues();
            self.status = "loading issues...".to_string();
        }
        self.selected_issue = None;
        self.focus = FocusTarget::Issues;
        self.detail_target = DetailTarget::Issue;
        self.refresh_project_notes().await?;
        self.screen = Screen::Issues;
        Ok(())
    }

    async fn switch_to_prs(&mut self) -> Result<()> {
        if self.repo_owner.is_some() && self.repo_name.is_some() {
            self.loading = true;
            self.all_prs.clear();
            self.spawn_load_prs();
            self.status = "loading PRs...".to_string();
        }
        self.selected_pr = None;
        self.screen = Screen::PullRequests;
        Ok(())
    }

    async fn refresh_issues(&mut self) -> Result<()> {
        if self.repo_owner.is_some() && self.repo_name.is_some() {
            self.status = "refreshing...".to_string();
            self.loading = true;
            self.spawn_load_issues();
        }
        Ok(())
    }

    async fn refresh_prs(&mut self) -> Result<()> {
        if self.repo_owner.is_some() && self.repo_name.is_some() {
            self.status = "refreshing...".to_string();
            self.loading = true;
            self.spawn_load_prs();
        }
        Ok(())
    }

    async fn handle_stats_key(&mut self, key: event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.screen = Screen::Dashboard;
            }
            KeyCode::Char('g') => self.go_to_dashboard(),
            KeyCode::Char('j') | KeyCode::Down => {
                self.stats_view.selected = (self.stats_view.selected + 1)
                    .min(self.stats_view.total_items.len().saturating_sub(1));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.stats_view.selected = self.stats_view.selected.saturating_sub(1);
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_roadmap_key(&mut self, key: event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.screen = Screen::Dashboard;
            }
            KeyCode::Char('g') => self.go_to_dashboard(),
            KeyCode::Char('j') | KeyCode::Down => {
                let group_len = self.roadmap_view.groups.get(self.roadmap_view.selected_group).map(|(_, items)| items.len()).unwrap_or(0);
                self.roadmap_view.selected_item = (self.roadmap_view.selected_item + 1)
                    .min(group_len.saturating_sub(1));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.roadmap_view.selected_item = self.roadmap_view.selected_item.saturating_sub(1);
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.roadmap_view.selected_group = self.roadmap_view.selected_group.saturating_sub(1);
                self.roadmap_view.selected_item = 0;
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.roadmap_view.selected_group = (self.roadmap_view.selected_group + 1)
                    .min(self.roadmap_view.groups.len().saturating_sub(1));
                self.roadmap_view.selected_item = 0;
            }
            _ => {}
        }
        Ok(())
    }

    async fn show_stats(&mut self) -> Result<()> {
        if let (Some(owner), Some(repo)) = (&self.repo_owner, &self.repo_name) {
            self.stats_view = StatsView::new(owner, repo);
            self.stats_view.update(&self.all_issues, &self.all_prs);
        }
        self.screen = Screen::Stats;
        Ok(())
    }

    async fn show_roadmap(&mut self) -> Result<()> {
        if let (Some(owner), Some(repo)) = (&self.repo_owner, &self.repo_name) {
            self.roadmap_view = RoadmapView::new(owner, repo);
            self.roadmap_view.update(&self.all_issues);
        }
        self.screen = Screen::Roadmap;
        Ok(())
    }

    async fn create_pr(&mut self, title: &str, body: Option<&str>) -> Result<()> {
        if let (Some(owner), Some(repo)) = (&self.repo_owner, &self.repo_name) {
            let head = crate::git::current_branch();
            let head = head.as_deref().unwrap_or("HEAD");
            let base = "main";

            self.status = format!("creating PR...");
            match self.client.create_pr(owner, repo, title, body, head, base).await {
                Ok(pr) => {
                    self.status = format!("created PR #{}", pr.number);
                    self.refresh_prs().await?;
                }
                Err(e) => {
                    self.status = format!("error creating PR: {e}");
                }
            }
        }
        Ok(())
    }
}
