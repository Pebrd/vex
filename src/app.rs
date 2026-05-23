use crate::cache::Cache;
use crate::config::{self, Config};
use crate::git;
use crate::github::{Client, Issue, PullRequest};
use crate::ui::dashboard::Dashboard;
use crate::ui::issues::IssuesView;
use crate::ui::prs::PRsView;
use crate::ui::{popup, status_bar};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use fuzzy_matcher::FuzzyMatcher;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Frame;
use ratatui::Terminal;
use std::io::Stdout;
use std::time::Duration;

enum Screen {
    Dashboard,
    Issues,
    PullRequests,
}

enum InputMode {
    None,
    Search { query: String },
    CreateIssue { title: String, body: String, step: u8 },
    AddProject { path: String },
    Comment { body: String },
    ConfirmMerge { pr_number: u64, selected: u8 },
}

pub struct App {
    config: Config,
    cache: Cache,
    client: Client,
    screen: Screen,
    input_mode: InputMode,
    dashboard: Dashboard,
    issues_view: IssuesView,
    prs_view: PRsView,
    all_issues: Vec<Issue>,
    all_prs: Vec<PullRequest>,
    selected_issue: Option<Issue>,
    selected_pr: Option<PullRequest>,
    repo_owner: Option<String>,
    repo_name: Option<String>,
    status: String,
    loading: bool,
    should_quit: bool,
}

impl App {
    pub async fn new(config: Config, cache: Cache) -> Result<Self> {
        let client = Client::new(&config);

        let (repo_owner, repo_name, status) = match git::detect_current_dir() {
            Ok(Some(info)) => {
                let s = format!("{}/{}", info.owner, info.repo);
                (Some(info.owner), Some(info.repo), s)
            }
            Ok(None) => (None, None, "no git repo detected".to_string()),
            Err(e) => (None, None, format!("{e}")),
        };

        let projects = config.projects.clone();
        let dashboard = Dashboard::new(projects);

        let mut app = Self {
            config,
            cache,
            client,
            screen: Screen::Dashboard,
            input_mode: InputMode::None,
            dashboard,
            issues_view: IssuesView::new(Vec::new()),
            prs_view: PRsView::new(Vec::new()),
            all_issues: Vec::new(),
            all_prs: Vec::new(),
            selected_issue: None,
            selected_pr: None,
            repo_owner,
            repo_name,
            status,
            loading: false,
            should_quit: false,
        };

        if app.repo_owner.is_some() && app.repo_name.is_some() {
            app.switch_to_issues().await?;
        }

        Ok(app)
    }

    pub async fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events().await?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(frame.area());

        match self.screen {
            Screen::Dashboard => self.dashboard.draw(frame, layout[0]),
            Screen::Issues => self
                .issues_view
                .draw(frame, layout[0], self.selected_issue.as_ref()),
            Screen::PullRequests => {
                self.prs_view
                    .draw(frame, layout[0], self.selected_pr.as_ref())
            }
        }

        let repo_info = self.repo_owner.as_ref().map(|o| {
            format!("{}/{}", o, self.repo_name.as_deref().unwrap_or("?"))
        });

        let status = match &self.input_mode {
            InputMode::Search { query } => format!("/{query}"),
            _ => self.status.clone(),
        };
        status_bar(frame, layout[1], &status, repo_info.as_deref());

        match &self.input_mode {
            InputMode::None => {}
            InputMode::Search { .. } => {}
            InputMode::CreateIssue { title, body, step } => {
                let (prompt, value, help) = match step {
                    0 => ("Issue title", title.as_str(), "enter to confirm, esc to cancel"),
                    _ => ("Issue body (optional)", body.as_str(), "enter to submit, esc to skip body"),
                };
                popup::input_dialog(frame, frame.area(), prompt, value, help);
            }
            InputMode::AddProject { path } => {
                popup::input_dialog(frame, frame.area(), "Add project path", path, "enter to confirm, esc to cancel");
            }
            InputMode::Comment { body } => {
                popup::input_dialog(frame, frame.area(), "Comment", body, "enter to post, esc to cancel");
            }
            InputMode::ConfirmMerge { selected, .. } => {
                popup::merge_dialog(frame, frame.area(), *selected as usize);
            }
        }
    }

    async fn handle_events(&mut self) -> Result<()> {
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => self.handle_key(key).await?,
                _ => {}
            }
        }
        Ok(())
    }

    async fn handle_key(&mut self, key: event::KeyEvent) -> Result<()> {
        match &self.input_mode {
            InputMode::None => match self.screen {
                Screen::Dashboard => self.handle_dashboard_key(key).await?,
                Screen::Issues => self.handle_issues_key(key).await?,
                Screen::PullRequests => self.handle_prs_key(key).await?,
            },
            InputMode::ConfirmMerge { .. } => self.handle_merge_key(key).await?,
            InputMode::Search { .. }
            | InputMode::CreateIssue { .. }
            | InputMode::AddProject { .. }
            | InputMode::Comment { .. } => self.handle_input_key(key).await?,
        }
        Ok(())
    }

    async fn handle_input_key(&mut self, key: event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                let was_search = matches!(self.input_mode, InputMode::Search { .. });
                self.input_mode = InputMode::None;
                if was_search {
                    self.restore_filter();
                }
            }
            KeyCode::Enter => {
                let mode = std::mem::replace(&mut self.input_mode, InputMode::None);
                match mode {
                    InputMode::Search { query } => {
                        self.status = format!("filtered: \"{query}\"");
                    }
                    InputMode::CreateIssue { title, body, step } => match step {
                        0 => {
                            if !title.is_empty() {
                                self.input_mode = InputMode::CreateIssue {
                                    title,
                                    body: String::new(),
                                    step: 1,
                                };
                            } else {
                                self.status = "title cannot be empty".to_string();
                                self.input_mode = InputMode::None;
                            }
                        }
                        _ => {
                            self.create_issue(&title, Some(&body)).await?;
                        }
                    },
                    InputMode::AddProject { path } => {
                        self.add_project(&path).await?;
                    }
                    InputMode::Comment { body } => {
                        self.add_comment(&body).await?;
                    }
                    InputMode::None | InputMode::ConfirmMerge { .. } => {}
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
            self.apply_search_filter();
        }

        Ok(())
    }

    fn active_input_mut(&mut self) -> Option<&mut String> {
        match &mut self.input_mode {
            InputMode::Search { query } => Some(query),
            InputMode::CreateIssue { title, step: 0, .. } => Some(title),
            InputMode::CreateIssue { body, step: 1, .. } => Some(body),
            InputMode::AddProject { path } => Some(path),
            InputMode::Comment { body } => Some(body),
            InputMode::None | InputMode::ConfirmMerge { .. } => None,
            InputMode::CreateIssue { .. } => None,
        }
    }

    // Dashboard key handlers
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
                    self.status = format!("{}/{}", project.owner, project.repo);
                    self.switch_to_issues().await?;
                }
            }
            KeyCode::Char('a') => {
                self.input_mode = InputMode::AddProject {
                    path: String::new(),
                };
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

    // Issues key handlers
    async fn handle_issues_key(&mut self, key: event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('q') => {
                self.repo_owner = None;
                self.repo_name = None;
                self.status = String::new();
                self.screen = Screen::Dashboard;
            }
            KeyCode::Char('Q') => self.should_quit = true,
            KeyCode::Char('j') | KeyCode::Down => {
                self.issues_view.selected = (self.issues_view.selected + 1)
                    .min(self.issues_view.issues.len().saturating_sub(1));
                self.selected_issue = self
                    .issues_view
                    .issues
                    .get(self.issues_view.selected)
                    .cloned();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.issues_view.selected = self.issues_view.selected.saturating_sub(1);
                self.selected_issue = self
                    .issues_view
                    .issues
                    .get(self.issues_view.selected)
                    .cloned();
            }
            KeyCode::Enter => {
                self.selected_issue = self
                    .issues_view
                    .issues
                    .get(self.issues_view.selected)
                    .cloned();
            }
            KeyCode::Char('/') => {
                self.input_mode = InputMode::Search {
                    query: String::new(),
                };
            }
            KeyCode::Char('c') => {
                self.input_mode = InputMode::CreateIssue {
                    title: String::new(),
                    body: String::new(),
                    step: 0,
                };
            }
            KeyCode::Char('x') => {
                self.toggle_issue_state().await?;
            }
            KeyCode::Char('o') => {
                if self.selected_issue.is_some() {
                    self.input_mode = InputMode::Comment {
                        body: String::new(),
                    };
                }
            }
            KeyCode::Char('p') => {
                self.switch_to_prs().await?;
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
            KeyCode::Char('q') => {
                self.repo_owner = None;
                self.repo_name = None;
                self.status = String::new();
                self.screen = Screen::Dashboard;
            }
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
            KeyCode::Char('/') => {
                self.input_mode = InputMode::Search {
                    query: String::new(),
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
            KeyCode::Char('i') => {
                self.switch_to_issues().await?;
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

    // --- Actions ---

    async fn create_issue(&mut self, title: &str, body: Option<&str>) -> Result<()> {
        if let (Some(owner), Some(repo)) = (&self.repo_owner, &self.repo_name) {
            self.status = format!("creating issue...");
            match self
                .client
                .create_issue(owner, repo, title, body, &[])
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

    async fn toggle_issue_state(&mut self) -> Result<()> {
        if let Some(issue) = &self.selected_issue {
            if let (Some(owner), Some(repo)) = (&self.repo_owner, &self.repo_name) {
                let new_state = if issue.state == "open" {
                    "closed"
                } else {
                    "open"
                };
                self.status = format!("{} issue #{}...", new_state, issue.number);
                match self
                    .client
                    .update_issue_state(owner, repo, issue.number, new_state)
                    .await
                {
                    Ok(updated) => {
                        let new_state = updated.state.clone();
                        self.status = format!("issue #{} {}", updated.number, new_state);
                        self.refresh_issues().await?;
                    }
                    Err(e) => {
                        self.status = format!("error: {e}");
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
                match self
                    .client
                    .create_comment(owner, repo, issue.number, &body)
                    .await
                {
                    Ok(_) => self.status = "comment posted".to_string(),
                    Err(e) => self.status = format!("error: {e}"),
                }
            } else if let Some(ref pr) = self.selected_pr {
                match self
                    .client
                    .create_comment(owner, repo, pr.number, &body)
                    .await
                {
                    Ok(_) => self.status = "comment posted".to_string(),
                    Err(e) => self.status = format!("error: {e}"),
                }
            }
        }
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

    async fn add_project(&mut self, path: &str) -> Result<()> {
        let expanded = if path.starts_with("~/") {
            let home = dirs::home_dir().map(|h| h.to_string_lossy().to_string()).unwrap_or_default();
            path.replacen("~", &home, 1)
        } else {
            path.to_string()
        };
        let p = std::path::Path::new(&expanded);

        if !p.exists() {
            self.status = format!("path does not exist: {path}");
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
            Err(e) => {
                self.status = format!("error: {e}");
            }
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

        let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();
        match self.screen {
            Screen::Issues => {
                let filtered: Vec<Issue> = self
                    .all_issues
                    .iter()
                    .filter(|i| matcher.fuzzy_match(&i.title, &query).unwrap_or(0) > 0)
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
                    .filter(|pr| matcher.fuzzy_match(&pr.title, &query).unwrap_or(0) > 0)
                    .cloned()
                    .collect();
                self.prs_view.prs = filtered;
                self.prs_view.selected = 0;
                self.selected_pr = self.prs_view.prs.first().cloned();
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
        if let (Some(owner), Some(repo)) = (&self.repo_owner, &self.repo_name) {
            self.loading = true;
            match self.client.list_issues(owner, repo, Some("open")).await {
                Ok(issues) => {
                    let count = issues.len();
                    self.all_issues = issues.clone();
                    self.issues_view = IssuesView::new(issues);
                    self.status = format!("{owner}/{repo} — {count} issues");
                }
                Err(e) => self.status = format!("error: {e}"),
            }
            self.loading = false;
        }
        self.selected_issue = None;
        self.screen = Screen::Issues;
        Ok(())
    }

    async fn switch_to_prs(&mut self) -> Result<()> {
        if let (Some(owner), Some(repo)) = (&self.repo_owner, &self.repo_name) {
            self.loading = true;
            match self.client.list_pull_requests(owner, repo, Some("open")).await {
                Ok(prs) => {
                    let count = prs.len();
                    self.all_prs = prs.clone();
                    self.prs_view = PRsView::new(prs);
                    self.status = format!("{owner}/{repo} — {count} PRs");
                }
                Err(e) => self.status = format!("error: {e}"),
            }
            self.loading = false;
        }
        self.selected_pr = None;
        self.screen = Screen::PullRequests;
        Ok(())
    }

    async fn refresh_issues(&mut self) -> Result<()> {
        if let (Some(owner), Some(repo)) = (&self.repo_owner, &self.repo_name) {
            self.status = format!("refreshing {owner}/{repo} issues...");
            match self.client.list_issues(owner, repo, None).await {
                Ok(issues) => {
                    let count = issues.len();
                    self.all_issues = issues.clone();
                    self.issues_view = IssuesView::new(issues);
                    self.status = format!("{owner}/{repo} — {count} issues (refreshed)");
                }
                Err(e) => self.status = format!("error: {e}"),
            }
        }
        Ok(())
    }

    async fn refresh_prs(&mut self) -> Result<()> {
        if let (Some(owner), Some(repo)) = (&self.repo_owner, &self.repo_name) {
            self.status = format!("refreshing {owner}/{repo} PRs...");
            match self.client.list_pull_requests(owner, repo, None).await {
                Ok(prs) => {
                    let count = prs.len();
                    self.all_prs = prs.clone();
                    self.prs_view = PRsView::new(prs);
                    self.status = format!("{owner}/{repo} — {count} PRs (refreshed)");
                }
                Err(e) => self.status = format!("error: {e}"),
            }
        }
        Ok(())
    }
}
