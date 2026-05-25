pub mod client;

pub use client::Client;

#[derive(Debug, Clone)]
pub struct Issue {
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub author: Option<String>,
    pub labels: Vec<String>,
    #[allow(dead_code)]
    pub assignees: Vec<String>,
    #[allow(dead_code)]
    pub comments: u64,
    #[allow(dead_code)]
    pub created_at: Option<String>,
    #[allow(dead_code)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PullRequest {
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub author: Option<String>,
    pub head_branch: Option<String>,
    pub base_branch: Option<String>,
    pub mergeable: Option<String>,
    pub checks_state: Option<String>,
    #[allow(dead_code)]
    pub created_at: Option<String>,
    #[allow(dead_code)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Comment {
    #[allow(dead_code)]
    pub id: u64,
    pub author: Option<String>,
    pub body: Option<String>,
    pub created_at: Option<String>,
}
