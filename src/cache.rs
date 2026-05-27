use crate::config::data_dir;
use crate::github::{Comment, Issue, PullRequest};
use anyhow::Result;
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct Cache {
    conn: Mutex<Connection>,
    #[allow(dead_code)]
    path: PathBuf,
}

fn serialize_str_list(items: &[String]) -> String {
    serde_json::to_string(items).unwrap_or_default()
}

fn deserialize_str_list(data: &str) -> Vec<String> {
    serde_json::from_str(data).unwrap_or_default()
}

impl Cache {
    pub fn new() -> Result<Self> {
        let dir = data_dir()?;
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("cache.db");

        let conn = Connection::open(&path)?;
        let cache = Self {
            conn: Mutex::new(conn),
            path,
        };
        cache.initialize()?;
        Ok(cache)
    }

    fn initialize(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS issues (
                id INTEGER PRIMARY KEY,
                owner TEXT NOT NULL,
                repo TEXT NOT NULL,
                number INTEGER NOT NULL,
                title TEXT NOT NULL,
                body TEXT,
                state TEXT NOT NULL,
                author TEXT,
                labels TEXT,
                assignees TEXT,
                created_at TEXT,
                updated_at TEXT,
                fetched_at TEXT NOT NULL DEFAULT (datetime('now')),
                UNIQUE(owner, repo, number)
            );

            CREATE TABLE IF NOT EXISTS pull_requests (
                id INTEGER PRIMARY KEY,
                owner TEXT NOT NULL,
                repo TEXT NOT NULL,
                number INTEGER NOT NULL,
                title TEXT NOT NULL,
                body TEXT,
                state TEXT NOT NULL,
                author TEXT,
                head_branch TEXT,
                base_branch TEXT,
                mergeable TEXT,
                checks_state TEXT,
                created_at TEXT,
                updated_at TEXT,
                fetched_at TEXT NOT NULL DEFAULT (datetime('now')),
                UNIQUE(owner, repo, number)
            );

            CREATE TABLE IF NOT EXISTS comments (
                id INTEGER PRIMARY KEY,
                owner TEXT NOT NULL,
                repo TEXT NOT NULL,
                issue_number INTEGER,
                pr_number INTEGER,
                author TEXT,
                body TEXT,
                created_at TEXT,
                fetched_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE INDEX IF NOT EXISTS idx_issues_repo ON issues(owner, repo);
            CREATE INDEX IF NOT EXISTS idx_prs_repo ON pull_requests(owner, repo);
            ",
        )?;
        Ok(())
    }

    pub fn get_issues(&self, owner: &str, repo: &str) -> Option<Vec<Issue>> {
        let conn = self.conn.lock().ok()?;
        let mut stmt = conn
            .prepare(
                "SELECT number, title, body, state, author, labels, assignees, comments, created_at, updated_at
                 FROM issues WHERE owner=?1 AND repo=?2",
            )
            .ok()?;
        let rows = stmt
            .query_map(rusqlite::params![owner, repo], |row| {
                let labels_str: String = row.get(5)?;
                let assignees_str: String = row.get(6)?;
                let comments_count: i64 = row.get(7)?;
                Ok(Issue {
                    number: row.get::<_, i64>(0)? as u64,
                    title: row.get(1)?,
                    body: row.get(2)?,
                    state: row.get(3)?,
                    author: row.get(4)?,
                    labels: deserialize_str_list(&labels_str),
                    assignees: deserialize_str_list(&assignees_str),
                    comments: comments_count as u64,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            })
            .ok()?;
        let issues: Vec<Issue> = rows.filter_map(|r| r.ok()).collect();
        if issues.is_empty() {
            None
        } else {
            Some(issues)
        }
    }

    pub fn set_issues(&self, owner: &str, repo: &str, issues: &[Issue]) {
        let conn = self.conn.lock().ok();
        let conn = match conn {
            Some(c) => c,
            None => return,
        };
        for issue in issues {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO issues (owner, repo, number, title, body, state, author, labels, assignees, comments, created_at, updated_at, fetched_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, datetime('now'))",
                rusqlite::params![
                    owner,
                    repo,
                    issue.number as i64,
                    issue.title,
                    issue.body,
                    issue.state,
                    issue.author,
                    serialize_str_list(&issue.labels),
                    serialize_str_list(&issue.assignees),
                    issue.comments as i64,
                    issue.created_at,
                    issue.updated_at,
                ],
            );
        }
    }

    pub fn get_prs(&self, owner: &str, repo: &str) -> Option<Vec<PullRequest>> {
        let conn = self.conn.lock().ok()?;
        let mut stmt = conn
            .prepare(
                "SELECT number, title, body, state, author, head_branch, base_branch, mergeable, checks_state, created_at, updated_at
                 FROM pull_requests WHERE owner=?1 AND repo=?2",
            )
            .ok()?;
        let rows = stmt
            .query_map(rusqlite::params![owner, repo], |row| {
                Ok(PullRequest {
                    number: row.get::<_, i64>(0)? as u64,
                    title: row.get(1)?,
                    body: row.get(2)?,
                    state: row.get(3)?,
                    author: row.get(4)?,
                    head_branch: row.get(5)?,
                    base_branch: row.get(6)?,
                    mergeable: row.get(7)?,
                    checks_state: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })
            .ok()?;
        let prs: Vec<PullRequest> = rows.filter_map(|r| r.ok()).collect();
        if prs.is_empty() { None } else { Some(prs) }
    }

    pub fn set_prs(&self, owner: &str, repo: &str, prs: &[PullRequest]) {
        let conn = self.conn.lock().ok();
        let conn = match conn {
            Some(c) => c,
            None => return,
        };
        for pr in prs {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO pull_requests (owner, repo, number, title, body, state, author, head_branch, base_branch, mergeable, checks_state, created_at, updated_at, fetched_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, datetime('now'))",
                rusqlite::params![
                    owner,
                    repo,
                    pr.number as i64,
                    pr.title,
                    pr.body,
                    pr.state,
                    pr.author,
                    pr.head_branch,
                    pr.base_branch,
                    pr.mergeable,
                    pr.checks_state,
                    pr.created_at,
                    pr.updated_at,
                ],
            );
        }
    }

    pub fn get_comments(&self, owner: &str, repo: &str, issue_number: u64) -> Option<Vec<Comment>> {
        let conn = self.conn.lock().ok()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, author, body, created_at
                 FROM comments WHERE owner=?1 AND repo=?2 AND issue_number=?3",
            )
            .ok()?;
        let rows = stmt
            .query_map(rusqlite::params![owner, repo, issue_number as i64], |row| {
                Ok(Comment {
                    id: row.get::<_, i64>(0)? as u64,
                    author: row.get(1)?,
                    body: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })
            .ok()?;
        let comments: Vec<Comment> = rows.filter_map(|r| r.ok()).collect();
        if comments.is_empty() {
            None
        } else {
            Some(comments)
        }
    }

    pub fn set_comments(&self, owner: &str, repo: &str, issue_number: u64, comments: &[Comment]) {
        let conn = self.conn.lock().ok();
        let conn = match conn {
            Some(c) => c,
            None => return,
        };
        for comment in comments {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO comments (id, owner, repo, issue_number, author, body, created_at, fetched_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, datetime('now'))",
                rusqlite::params![
                    comment.id as i64,
                    owner,
                    repo,
                    issue_number as i64,
                    comment.author,
                    comment.body,
                    comment.created_at,
                ],
            );
        }
    }

    #[allow(dead_code)]
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}
