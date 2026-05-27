# Review: CHANGES REQUIRED ❌
**Files:** `src/git.rs` | **Date:** 2026-05-27 | **VAULT ref:** 2026-05-27-002-review-git2-operations.md

## TL;DR
**Status:** BLOCKED
**Blockers:** 2 — `unstage_file` stages deletions instead of resetting to HEAD; `pull` doesn't update working tree/index after merge
**Detail needed:** yes

## Strengths
- All 25 required items (functions + structs + imports) present
- API corrections from plan spec to real git2 API properly resolved (discard_file, stash, pull, borrow fixes)
- Builds with 0 errors, 0 new clippy warnings, 2/2 tests passing, fmt clean
- Consistent error propagation via `anyhow::Result` — no panics or unwraps in core logic
- Clear section divider between existing URL parsing code and new git2 operations
- Good use of `#[allow(dead_code)]` to avoid warnings until TUI caller lands
- `default_remote_callbacks()` properly isolated as a helper

## Blockers

### 1. `unstage_file` stages deletions instead of resetting index to HEAD
**Location:** `src/git.rs:214-219`
**Problem:** `index.remove_path(path)` removes the index entry entirely. For a tracked file that was modified and staged, this stages a **deletion** of the file (it's in HEAD's tree but no longer in the index). `git status` after calling `unstage_file` shows "D" (staged deletion) + potentially "??" (untracked) — completely wrong. This only accidentally works correctly for newly-added files that don't exist in HEAD.
**Fix:** Replace with `repo.reset_default(Some(&head_tree.as_object()), [path])` to reset the index entry to match HEAD. This correctly handles all cases: modified→staged (resets to HEAD), newly added (removes from index since absent in HEAD), staged deletion (restores to HEAD).

### 2. `pull` creates merge commit but leaves working tree and index in pre-merge state
**Location:** `src/git.rs:443-472`
**Problem:** `merge_trees()` returns an in-memory `Index` representing the merged result, but neither the on-disk index nor the working tree are updated. After `pull()`, HEAD points to the new merge commit, but `git status` shows all files changed by the merge as "staged for commit" (index != HEAD), and the working tree still reflects the pre-merge state. Running `git checkout -- .` would silently discard the merge.
**Fix:** Either (a) write the merged index to disk and checkout the working tree via `checkout_index(Some(&mut merged), None)` before committing, or (b) use `repo.checkout_head()` after committing to sync the working tree. The merge commit should come last, after the index and working tree are consistent.

## Observations (non-blocking)

1. **`pull` always creates merge commits, never fast-forwards** — this deviates from standard `git pull` behavior. Consider checking if a fast-forward is possible first (i.e., if HEAD is an ancestor of the remote branch) and using `repo.reset()` + `repo.checkout_head()` instead. Acceptable if the intent is `pull --no-ff`.

2. **`get_statuses` duplicates entries for files with both staged and unstaged changes** — a file modified in both the index and working tree produces two `FileStatus` entries with different `staged` values. The TUI caller must handle deduplication. Either document this or collapse into a single entry with combined status (e.g., `"MM"` like git porcelain).

3. **`PathBuf` import at line 78 is separated from other `std` imports (lines 5-6)** — minor style inconsistency. Should be grouped with `use std::path::Path` at line 6.

4. **`get_commits` rebuilds the branch map on every call** — iterates all local branches each time, O(branches + commits). If the TUI refreshes frequently, consider caching the branch map or making it a parameter. Minor for now since the function is dead code until Task 3.

5. **`write!` result silently discarded in `get_commit_diff` callback** — `let _ = write!(buf, ...)` swallows potential I/O errors. Harmless for `Vec<u8>` (writes never fail), but a `write_all` + `unwrap()` pattern would be clearer.
