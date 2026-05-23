use super::{Comment, Issue, PullRequest};
use anyhow::{Context, Result};
use crate::config::{self, Config};
use serde::Deserialize;

pub struct Client {
    token: String,
    http: reqwest::Client,
}

#[derive(Deserialize)]
struct GhIssue {
    number: u64,
    title: String,
    body: Option<String>,
    state: String,
    user: Option<GhUser>,
    labels: Vec<GhLabel>,
    assignees: Vec<GhUser>,
    comments: u64,
    created_at: Option<String>,
    updated_at: Option<String>,
}

#[derive(Deserialize)]
struct GhPr {
    number: u64,
    title: String,
    body: Option<String>,
    state: String,
    user: Option<GhUser>,
    head: Option<GhBranch>,
    base: Option<GhBranch>,
    mergeable: Option<bool>,
    created_at: Option<String>,
    updated_at: Option<String>,
}

#[derive(Deserialize)]
struct GhComment {
    id: u64,
    user: Option<GhUser>,
    body: Option<String>,
    created_at: Option<String>,
}

#[derive(Deserialize)]
struct GhUser {
    login: String,
}

#[derive(Deserialize)]
struct GhLabel {
    name: String,
}

#[derive(Deserialize)]
struct GhBranch {
    label: String,
    r#ref: String,
}

#[derive(Deserialize)]
struct GhCombinedStatus {
    state: String,
}

impl Client {
    pub fn new(config: &Config) -> Self {
        let http = reqwest::Client::builder()
            .user_agent("vex/0.1.0")
            .build()
            .expect("building reqwest client");
        let token = config::resolve_token(config);
        Self {
            token,
            http,
        }
    }

    fn github_get(&self, path: &str) -> reqwest::RequestBuilder {
        self.http
            .get(format!("https://api.github.com{path}"))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
    }

    pub async fn list_issues(
        &self,
        owner: &str,
        repo: &str,
        state: Option<&str>,
    ) -> Result<Vec<Issue>> {
        let path = match state {
            Some(s) => format!("/repos/{owner}/{repo}/issues?state={s}"),
            None => format!("/repos/{owner}/{repo}/issues"),
        };
        let issues: Vec<GhIssue> = self
            .github_get(&path)
            .send()
            .await
            .with_context(|| format!("fetching issues for {owner}/{repo}"))?
            .error_for_status()
            .with_context(|| format!("GitHub API error for {owner}/{repo} issues"))?
            .json()
            .await?;

        Ok(issues
            .into_iter()
            .map(|i| Issue {
                number: i.number,
                title: i.title,
                body: i.body,
                state: i.state,
                author: i.user.map(|u| u.login),
                labels: i.labels.into_iter().map(|l| l.name).collect(),
                assignees: i.assignees.into_iter().map(|a| a.login).collect(),
                comments: i.comments,
                created_at: i.created_at,
                updated_at: i.updated_at,
            })
            .collect())
    }

    pub async fn list_pull_requests(
        &self,
        owner: &str,
        repo: &str,
        state: Option<&str>,
    ) -> Result<Vec<PullRequest>> {
        let path = match state {
            Some(s) => format!("/repos/{owner}/{repo}/pulls?state={s}"),
            None => format!("/repos/{owner}/{repo}/pulls"),
        };
        let prs: Vec<GhPr> = self
            .github_get(&path)
            .send()
            .await
            .with_context(|| format!("fetching PRs for {owner}/{repo}"))?
            .error_for_status()?
            .json()
            .await?;

        let mut result = Vec::new();
        for pr in prs {
            let checks_state = self.get_checks_state(owner, repo, pr.number).await.ok();

            result.push(PullRequest {
                number: pr.number,
                title: pr.title,
                body: pr.body,
                state: pr.state,
                author: pr.user.map(|u| u.login),
                head_branch: pr.head.map(|b| b.r#ref),
                base_branch: pr.base.map(|b| b.r#ref),
                mergeable: pr.mergeable.map(|m| if m { "mergeable".to_string() } else { "conflict".to_string() }),
                checks_state,
                created_at: pr.created_at,
                updated_at: pr.updated_at,
            });
        }

        Ok(result)
    }

    async fn get_checks_state(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<String> {
        let status: GhCombinedStatus = self
            .github_get(&format!(
                "/repos/{owner}/{repo}/commits/{pr_number}/status"
            ))
            .send()
            .await
            .with_context(|| format!("fetching checks for PR #{pr_number}"))?
            .error_for_status()?
            .json()
            .await?;
        Ok(status.state)
    }

    pub async fn list_comments(
        &self,
        owner: &str,
        repo: &str,
        issue_number: u64,
    ) -> Result<Vec<Comment>> {
        let comments: Vec<GhComment> = self
            .github_get(&format!(
                "/repos/{owner}/{repo}/issues/{issue_number}/comments"
            ))
            .send()
            .await
            .with_context(|| format!("fetching comments for #{issue_number}"))?
            .error_for_status()?
            .json()
            .await?;

        Ok(comments
            .into_iter()
            .map(|c| Comment {
                id: c.id,
                author: c.user.map(|u| u.login),
                body: c.body,
                created_at: c.created_at,
            })
            .collect())
    }

    pub async fn create_issue(
        &self,
        owner: &str,
        repo: &str,
        title: &str,
        body: Option<&str>,
        labels: &[String],
    ) -> Result<Issue> {
        #[derive(serde::Serialize)]
        struct CreateIssue {
            title: String,
            body: Option<String>,
            labels: Vec<String>,
        }

        let payload = CreateIssue {
            title: title.to_string(),
            body: body.map(|s| s.to_string()),
            labels: labels.to_vec(),
        };

        let issue: GhIssue = self
            .http
            .post(format!(
                "https://api.github.com/repos/{owner}/{repo}/issues"
            ))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .json(&payload)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(Issue {
            number: issue.number,
            title: issue.title,
            body: issue.body,
            state: issue.state,
            author: issue.user.map(|u| u.login),
            labels: issue.labels.into_iter().map(|l| l.name).collect(),
            assignees: issue.assignees.into_iter().map(|a| a.login).collect(),
            comments: issue.comments,
            created_at: issue.created_at,
            updated_at: issue.updated_at,
        })
    }

    pub async fn create_comment(
        &self,
        owner: &str,
        repo: &str,
        issue_number: u64,
        body: &str,
    ) -> Result<Comment> {
        #[derive(serde::Serialize)]
        struct CreateComment {
            body: String,
        }

        let comment: GhComment = self
            .http
            .post(format!(
                "https://api.github.com/repos/{owner}/{repo}/issues/{issue_number}/comments"
            ))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .json(&CreateComment {
                body: body.to_string(),
            })
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(Comment {
            id: comment.id,
            author: comment.user.map(|u| u.login),
            body: comment.body,
            created_at: comment.created_at,
        })
    }

    pub async fn update_issue_state(
        &self,
        owner: &str,
        repo: &str,
        issue_number: u64,
        state: &str,
    ) -> Result<Issue> {
        #[derive(serde::Serialize)]
        struct UpdateState {
            state: String,
        }

        let issue: GhIssue = self
            .http
            .patch(format!(
                "https://api.github.com/repos/{owner}/{repo}/issues/{issue_number}"
            ))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .json(&UpdateState {
                state: state.to_string(),
            })
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(Issue {
            number: issue.number,
            title: issue.title,
            body: issue.body,
            state: issue.state,
            author: issue.user.map(|u| u.login),
            labels: issue.labels.into_iter().map(|l| l.name).collect(),
            assignees: issue.assignees.into_iter().map(|a| a.login).collect(),
            comments: issue.comments,
            created_at: issue.created_at,
            updated_at: issue.updated_at,
        })
    }

    pub async fn merge_pr(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
        method: &str,
    ) -> Result<()> {
        #[derive(serde::Serialize)]
        struct MergePr {
            merge_method: String,
        }

        self.http
            .put(format!(
                "https://api.github.com/repos/{owner}/{repo}/pulls/{pr_number}/merge"
            ))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .json(&MergePr {
                merge_method: method.to_string(),
            })
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}
