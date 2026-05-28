# vex

Terminal UI for GitHub issues and pull requests.

## Features

- Dashboard with project management
- Browse and filter issues / PRs from any repo
- Create issues with labels
- Edit issue title, body, and labels inline
- Close / reopen issues
- Add comments
- Create and merge pull requests
- File-based local notes (`.vex/*.md`) with priority, status, and issue linking
- Inline comments on Enter
- Fuzzy search (`/`)
- Filter by state (`f` cycles open → all → closed)
- Stats overview (`s`)
- Roadmap grouped by label (`t`)
- Quick capture CLI: `vex add <title> [--body "..."] [--priority high]`
- Caches API responses offline
- Works with any GitHub repo
- Open terminal in project directory (`Ctrl+t`)
- Launch configured CLI tool inside terminal (`Ctrl+e`, e.g. opencode, code, gh)
- Settings screen to select default CLI (`Ctrl+g`, auto-detects tools from PATH)
- Dashboard warnings for projects with missing paths
- Edit project directory from Dashboard (`e` key → file browser)
- Open project in file explorer (`Ctrl+o`) or browser (`O`)
- Git screen (`g`): stage/unstage files, commit, view log, branches, push/pull/fetch/stash
- Side-by-side diff viewer in Git screen and PR view (`Enter`)
- Color themes with 11 presets (`Ctrl+g`), configurable via `config.toml`
- Markdown rendering in issue bodies, comments, and notes
- Multi-select for bulk operations: close/reopen issues, stage/unstage files, delete notes/branches (`V`)
- Notes with YAML front-matter (title, priority, status, issue link)

## Installation

```bash
cargo install --git https://github.com/Pebrd/vex.git
```

Or build from source:

```bash
git clone https://github.com/Pebrd/vex.git
cd vex
cargo install --path .
```

## Usage

```bash
vex
```

vex will auto-detect the GitHub repo if you're in a git directory with a GitHub remote. Otherwise, press `a` to add a project from the dashboard.

### Keybindings

#### Global

| Key | Action |
|---|---|
| `?` | Toggle help screen |
| `Q` | Quit |
| `g` | Go to dashboard |
| `Ctrl+t` | Open terminal in project directory |
| `Ctrl+e` | Launch configured CLI tool |
| `Ctrl+g` | Settings screen (themes, CLI tool) |
| `0`-`9` | Quick-switch to project N (dashboard) |

#### Dashboard

| Key | Action |
|---|---|
| `j`/`k` | Navigate projects |
| `Enter` | Open selected project |
| `a` | Add project (opens file browser) |
| `d` | Delete project |
| `e` | Edit project path (opens file browser) |
| `n` | Notes screen |
| `s` | Stats view |
| `t` | Roadmap view |
| `Ctrl+o` | Open project in file explorer |

#### Issues Screen

| Key | Action |
|---|---|
| `j`/`k` | Navigate issues |
| `Enter` | Open detail (loads comments) |
| `Tab` | Toggle focus (issues ↔ notes) |
| `/` | Fuzzy search |
| `f` | Cycle filter (open → all → closed) |
| `S` | Cycle sort order |
| `l` | Cycle label filter |
| `c` | Create issue |
| `e` | Edit issue (title, body, labels) |
| `x` | Toggle open/closed |
| `o` | Add comment |
| `n` | New linked note |
| `L` | Link note to issue |
| `d` | Delete linked note |
| `O` | Open in browser |
| `V` | Enter multi-select mode |
| `r` | Refresh |
| `p` | Switch to PRs view |
| `Ctrl+d` / `Ctrl+u` | Scroll detail |
| `mouse wheel` | Scroll detail |

#### Pull Requests Screen

| Key | Action |
|---|---|
| `j`/`k` | Navigate PRs |
| `Enter` | View side-by-side diff |
| `/` | Search |
| `o` | Add comment |
| `O` | Open in browser |
| `m` | Merge PR (1=squash, 2=rebase, 3=merge) |
| `c` | Create PR |
| `r` | Refresh |
| `i` | Switch to Issues view |

#### Notes Screen

| Key | Action |
|---|---|
| `j`/`k` | Navigate notes |
| `Enter` | Open detail |
| `/` | Search |
| `n` | Create note |
| `E` | Edit note |
| `x` | Toggle open/closed |
| `d` | Delete note |
| `V` | Enter multi-select mode |
| `L` | Link note to issue (detail view) |

#### Git Screen

| Key | Action |
|---|---|
| `j`/`k` | Navigate list |
| `Tab` | Toggle focus (files/commits/branches ↔ diff) |
| `1` | Files mode |
| `2` | Commits mode |
| `3` | Branches mode |
| `Enter` | View diff (commits) / checkout branch |
| `space` | Stage/unstage selected file |
| `t` | Stage/unstage all |
| `s` | Open commit modal |
| `d` | Discard file (unstaged) / delete branch |
| `p` | Pull |
| `P` | Push |
| `f` | Fetch |
| `S` | Stash |
| `Z` | Stash pop |
| `n` | New branch |
| `V` | Enter multi-select mode |

#### Multi-Select Mode

Active in Issues, Notes, and Git screens after pressing `V`.

| Key | Action |
|---|---|
| `space` | Toggle selection on current item |
| `V` | Exit multi-select mode |
| `c` | Close selected issues (Issues) |
| `r` | Reopen selected issues (Issues) |
| `t` | Stage/unstage selected files (Git) |
| `d` | Delete selected branches (Git) / notes (Notes) |

#### Settings Screen (`Ctrl+g`)

| Key | Action |
|---|---|
| `j`/`k` | Navigate settings list |
| `Enter` | Select CLI tool or theme |
| `Esc` / `q` | Back to dashboard |

### Configuration

Config file at `~/.config/vex/config.toml`:

```toml
token = "ghp_..."  # optional — falls back to `gh auth token`
selected_cli = "opencode"  # optional — for Ctrl+e launcher (auto-detected in settings)
[[projects]]
name = "my-project"
path = "/home/user/projects/my-project"
owner = "myuser"
repo = "my-repo"
```

Projects are added automatically via the dashboard (`a` → file browser). Use `e` on a project to edit its path. Configure the CLI launcher via `Ctrl+g` settings screen.

### Quick Capture

```bash
vex add "Fix the login bug" --body "Investigate the auth flow" --priority high
```

## Dependencies

- [ratatui](https://github.com/ratatui/ratatui) — TUI framework
- [crossterm](https://github.com/crossterm-rs/crossterm) — terminal control
- [tokio](https://tokio.rs) — async runtime
- [reqwest](https://github.com/seanmonstar/reqwest) — HTTP client
- [rusqlite](https://github.com/rusqlite/rusqlite) — SQLite (API cache only)
- [serde](https://serde.rs) — serialization
- [fuzzy-matcher](https://github.com/rapiz1/fuzzy-matcher) — fuzzy search
- [git2](https://github.com/rust-lang/git2-rs) — git operations (staging, commit, log, branches, push/pull/stash)

## License

MIT
