use anyhow::{Context, Result};
use git2::{
    BranchType, FetchOptions, PushOptions, RemoteCallbacks, Repository, Status, StatusOptions,
};
use std::io::Write;
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

// ── git2 operations ──────────────────────────────────────────────────────────
// Most functions below are #[allow(dead_code)] until the Git Screen TUI uses them.

#[allow(dead_code)]
pub fn open_repo(project_dir: &Path) -> Result<Repository> {
    Repository::open(project_dir).context("opening git repository")
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct FileStatus {
    pub path: String,
    pub status: String,
    pub staged: bool,
    pub is_untracked: bool,
}

#[allow(dead_code)]
pub fn get_statuses(repo: &Repository) -> Result<Vec<FileStatus>> {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .renames_head_to_index(true);

    let statuses = repo.statuses(Some(&mut opts))?;
    let mut files: Vec<FileStatus> = Vec::new();

    for entry in statuses.iter() {
        let s = entry.status();
        let path = entry.path().unwrap_or("").to_string();

        if s.intersects(
            Status::INDEX_NEW
                | Status::INDEX_MODIFIED
                | Status::INDEX_DELETED
                | Status::INDEX_RENAMED
                | Status::INDEX_TYPECHANGE,
        ) {
            let ch = if s.intersects(Status::INDEX_NEW) {
                "A"
            } else if s.intersects(Status::INDEX_DELETED) {
                "D"
            } else {
                "M"
            };
            files.push(FileStatus {
                path: path.clone(),
                status: ch.to_string(),
                staged: true,
                is_untracked: false,
            });
        }

        if s.intersects(
            Status::WT_NEW
                | Status::WT_MODIFIED
                | Status::WT_DELETED
                | Status::WT_RENAMED
                | Status::WT_TYPECHANGE,
        ) {
            let ch = if s.intersects(Status::WT_NEW) {
                "?"
            } else if s.intersects(Status::WT_DELETED) {
                "D"
            } else {
                "M"
            };
            let is_untracked = s.intersects(Status::WT_NEW);
            files.push(FileStatus {
                path: path.clone(),
                status: format!(" {ch}"),
                staged: false,
                is_untracked,
            });
        }
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

#[allow(dead_code)]
pub fn stage_file(repo: &Repository, path: &str) -> Result<()> {
    let mut index = repo.index()?;
    index.add_path(Path::new(path))?;
    index.write()?;
    Ok(())
}

#[allow(dead_code)]
pub fn unstage_file(repo: &Repository, path: &str) -> Result<()> {
    let mut index = repo.index()?;
    index.remove_path(Path::new(path))?;
    index.write()?;
    Ok(())
}

#[allow(dead_code)]
pub fn stage_all(repo: &Repository) -> Result<()> {
    let mut index = repo.index()?;
    index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
    index.write()?;
    Ok(())
}

#[allow(dead_code)]
pub fn unstage_all(repo: &Repository) -> Result<()> {
    let head = repo.head()?;
    let tree = head.peel_to_tree()?;
    let mut index = repo.index()?;
    index.read_tree(&tree)?;
    index.write()?;
    Ok(())
}

#[allow(dead_code)]
pub fn discard_file(repo: &Repository, path: &str) -> Result<()> {
    let mut checkout_builder = git2::build::CheckoutBuilder::new();
    checkout_builder.force().path(path);
    repo.checkout_head(Some(&mut checkout_builder))?;
    Ok(())
}

#[allow(dead_code)]
pub fn create_commit(repo: &Repository, message: &str) -> Result<()> {
    let sig = repo.signature()?;
    let mut index = repo.index()?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;

    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());

    if let Some(p) = parent {
        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&p])?;
    } else {
        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[])?;
    }
    Ok(())
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub id: String,
    pub short_id: String,
    pub message: String,
    pub author: String,
    pub time: i64,
    pub is_head: bool,
    pub branch_names: Vec<String>,
}

#[allow(dead_code)]
pub fn get_commits(repo: &Repository, max: usize) -> Result<Vec<CommitInfo>> {
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(git2::Sort::TIME)?;

    let head_id = repo.head().ok().and_then(|h| h.target());
    let branches = repo.branches(Some(BranchType::Local)).ok();
    let branch_map: std::collections::HashMap<String, Vec<String>> = {
        let mut m: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        if let Some(branches) = branches {
            for b in branches.flatten() {
                if let (Some(name), Some(target)) = (b.0.name().ok().flatten(), b.0.get().target())
                {
                    m.entry(target.to_string())
                        .or_default()
                        .push(name.to_string());
                }
            }
        }
        m
    };

    let mut commits = Vec::new();
    for oid in revwalk.take(max) {
        let oid = oid?;
        if let Ok(commit) = repo.find_commit(oid) {
            let id = oid.to_string();
            let short_id = id[..7].to_string();
            let branch_names = branch_map.get(&id).cloned().unwrap_or_default();
            commits.push(CommitInfo {
                id,
                short_id,
                message: commit
                    .message()
                    .unwrap_or("")
                    .lines()
                    .next()
                    .unwrap_or("")
                    .to_string(),
                author: commit.author().name().unwrap_or("unknown").to_string(),
                time: commit.time().seconds(),
                is_head: Some(oid) == head_id,
                branch_names,
            });
        }
    }
    Ok(commits)
}

#[allow(dead_code)]
pub fn get_commit_diff(repo: &Repository, commit_id: &str) -> Result<String> {
    let oid = commit_id.parse::<git2::Oid>()?;
    let commit = repo.find_commit(oid)?;
    let tree = commit.tree()?;
    let parent_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());
    let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?;

    let mut buf: Vec<u8> = Vec::new();
    diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        let prefix = match line.origin() {
            '+' => "+",
            '-' => "-",
            ' ' => " ",
            _ => " ",
        };
        let content = std::str::from_utf8(line.content()).unwrap_or("");
        let _ = write!(buf, "{prefix}{content}");
        true
    })?;
    Ok(String::from_utf8(buf).unwrap_or_default())
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub name: String,
    pub is_current: bool,
    pub upstream: Option<String>,
}

#[allow(dead_code)]
pub fn get_branches(repo: &Repository) -> Result<Vec<BranchInfo>> {
    let current = current_branch();
    let mut branches = Vec::new();
    for b in repo.branches(Some(BranchType::Local))? {
        let (branch, _) = b?;
        let name = branch.name()?.unwrap_or("").to_string();
        let upstream = branch
            .upstream()
            .ok()
            .and_then(|u| u.name().ok().flatten().map(|s| s.to_string()));
        let is_current = Some(name.as_str()) == current.as_deref();
        branches.push(BranchInfo {
            name,
            is_current,
            upstream,
        });
    }
    branches.sort_by(|a, b| b.is_current.cmp(&a.is_current).then(a.name.cmp(&b.name)));
    Ok(branches)
}

#[allow(dead_code)]
pub fn checkout_branch(repo: &Repository, name: &str) -> Result<()> {
    let branch_ref = format!("refs/heads/{name}");
    let object = repo.revparse_single(&branch_ref)?;
    repo.checkout_tree(&object, None)?;
    repo.set_head(&branch_ref)?;
    Ok(())
}

#[allow(dead_code)]
pub fn create_branch(repo: &Repository, name: &str) -> Result<()> {
    let head = repo.head()?;
    let commit = head.peel_to_commit()?;
    repo.branch(name, &commit, false)?;
    Ok(())
}

#[allow(dead_code)]
pub fn delete_branch(repo: &Repository, name: &str) -> Result<()> {
    let mut branch = repo.find_branch(name, BranchType::Local)?;
    branch.delete()?;
    Ok(())
}

#[allow(dead_code)]
fn default_remote_callbacks() -> RemoteCallbacks<'static> {
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_url, username_from_url, _allowed_types| {
        git2::Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"))
    });
    callbacks
}

#[allow(dead_code)]
pub fn push_current_branch(repo: &Repository) -> Result<()> {
    let head = repo.head()?;
    let branch_name = head.shorthand().unwrap_or("HEAD").to_string();
    let mut remote = repo.find_remote("origin")?;
    let refspec = format!("refs/heads/{branch_name}:refs/heads/{branch_name}");

    let callbacks = default_remote_callbacks();
    let mut push_opts = PushOptions::new();
    push_opts.remote_callbacks(callbacks);

    remote.push(&[&refspec], Some(&mut push_opts))?;
    Ok(())
}

#[allow(dead_code)]
pub fn fetch(repo: &Repository) -> Result<()> {
    let mut remote = repo.find_remote("origin")?;
    let callbacks = default_remote_callbacks();
    let mut fetch_opts = FetchOptions::new();
    fetch_opts.remote_callbacks(callbacks);
    remote.fetch(
        &["refs/heads/*:refs/remotes/origin/*"],
        Some(&mut fetch_opts),
        None,
    )?;
    Ok(())
}

#[allow(dead_code)]
pub fn pull(repo: &Repository) -> Result<()> {
    fetch(repo)?;
    let head = repo.head()?;
    let branch_name = head.shorthand().unwrap_or("HEAD").to_string();
    let remote_branch = format!("origin/{branch_name}");
    let remote_oid = repo.refname_to_id(&format!("refs/remotes/{remote_branch}"))?;
    let remote_commit = repo.find_commit(remote_oid)?;
    let head_commit = head.peel_to_commit()?;

    if head_commit.id() != remote_commit.id() {
        let sig = repo.signature()?;
        let merge_base_oid = repo.merge_base(head_commit.id(), remote_commit.id())?;
        let merge_base_commit = repo.find_commit(merge_base_oid)?;
        let head_tree = head_commit.tree()?;
        let remote_tree = remote_commit.tree()?;
        let base_tree = merge_base_commit.tree()?;
        let mut merged = repo.merge_trees(&base_tree, &head_tree, &remote_tree, None)?;
        let merged_tree_id = merged.write_tree_to(repo)?;
        let merged_tree = repo.find_tree(merged_tree_id)?;
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            &format!("Merge branch '{remote_branch}'"),
            &merged_tree,
            &[&head_commit, &remote_commit],
        )?;
    }
    Ok(())
}

#[allow(dead_code)]
pub fn stash_push(repo: &mut Repository, message: &str) -> Result<()> {
    let sig = repo.signature()?;
    repo.stash_save(&sig, message, Some(git2::StashFlags::DEFAULT))?;
    Ok(())
}

#[allow(dead_code)]
pub fn stash_pop(repo: &mut Repository) -> Result<()> {
    repo.stash_pop(0, None)?;
    Ok(())
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
