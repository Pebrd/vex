use anyhow::Result;
use crate::config::data_dir;
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct Cache {
    conn: Mutex<Connection>,
    path: PathBuf,
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

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}
