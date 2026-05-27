use anyhow::{Context, Result};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct RepoInfo {
    pub owner: String,
    pub repo: String,
}

#[allow(dead_code)]
pub fn detect_current_dir() -> Result<Option<RepoInfo>> {
    let cwd = std::env::current_dir()?;
    detect(&cwd)
}

pub fn current_branch() -> Option<String> {
    let head_path = Path::new(".git/HEAD");
    if !head_path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(head_path).ok()?;
    let content = content.trim();
    if let Some(ref_str) = content.strip_prefix("ref: refs/heads/") {
        Some(ref_str.to_string())
    } else {
        None
    }
}

pub fn project_root() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    let git_dir = find_git_dir(&cwd)?;
    git_dir.parent().map(|p| p.to_path_buf())
}

pub fn project_root_for(path: &Path) -> Option<PathBuf> {
    let git_dir = find_git_dir(path)?;
    git_dir.parent().map(|p| p.to_path_buf())
}

pub fn detect(path: &Path) -> Result<Option<RepoInfo>> {
    let git_dir = find_git_dir(path);
    let git_dir = match git_dir {
        Some(d) => d,
        None => return Ok(None),
    };

    let config_path = git_dir.join("config");
    let content = std::fs::read_to_string(&config_path)
        .with_context(|| format!("reading {}", config_path.display()))?;

    parse_remote(&content)
}

fn find_git_dir(path: &Path) -> Option<PathBuf> {
    let mut current = Some(path);
    while let Some(dir) = current {
        let candidate = dir.join(".git");
        if candidate.exists() && candidate.is_dir() {
            return Some(candidate);
        }
        if let Some(parent) = dir.parent() {
            if parent == dir {
                break;
            }
            current = Some(parent);
        } else {
            break;
        }
    }
    None
}

use std::path::PathBuf;

#[allow(dead_code)]
pub fn has_git(path: &Path) -> bool {
    find_git_dir(path).is_some()
}

fn parse_remote(content: &str) -> Result<Option<RepoInfo>> {
    for line in content.lines() {
        let line = line.trim();
        if let Some(url) = line.strip_prefix("url = ") {
            let url = url.trim_matches('"');
            if let Some(info) = parse_github_url(url) {
                return Ok(Some(info));
            }
        }
    }
    Ok(None)
}

fn parse_github_url(url: &str) -> Option<RepoInfo> {
    let url = url.strip_suffix(".git").unwrap_or(url);

    if let Some(rest) = url.strip_prefix("https://github.com/") {
        let parts: Vec<&str> = rest.split('/').collect();
        if parts.len() >= 2 {
            return Some(RepoInfo {
                owner: parts[0].to_string(),
                repo: parts[1].to_string(),
            });
        }
    }

    if let Some(rest) = url.strip_prefix("git@github.com:") {
        let parts: Vec<&str> = rest.split('/').collect();
        if parts.len() >= 2 {
            return Some(RepoInfo {
                owner: parts[0].to_string(),
                repo: parts[1].to_string(),
            });
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_https_url() {
        let info = parse_github_url("https://github.com/pebrd/vex.git").unwrap();
        assert_eq!(info.owner, "pebrd");
        assert_eq!(info.repo, "vex");
    }

    #[test]
    fn test_parse_ssh_url() {
        let info = parse_github_url("git@github.com:pebrd/lumo.git").unwrap();
        assert_eq!(info.owner, "pebrd");
        assert_eq!(info.repo, "lumo");
    }
}
