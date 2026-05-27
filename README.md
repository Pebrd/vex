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

| Key | Action |
|---|---|
| `j`/`k` | Navigate up/down |
| `Tab` | Switch focus (issues ↔ notes) |
| `Enter` | View inline comments |
| `/` | Fuzzy search |
| `c` | Create issue / create PR (in PRs view) |
| `e` | Edit issue inline |
| `x` | Toggle open/closed |
| `o` | Add comment |
| `n` | New note |
| `L` | Link note to issue |
| `d` | Delete note |
| `m` | Merge PR (then 1=squash, 2=rebase, 3=merge) |
| `f` | Cycle filter (open → all → closed) |
| `s` | Stats view |
| `t` | Roadmap view |
| `p` | Switch to PRs view |
| `i` | Switch to Issues view |
| `r` | Refresh |
| `e` | Edit project path (Dashboard) |
| `Ctrl+o` | Open project in file explorer |
| `O` | Open project in browser |
| `Ctrl+t` | Open terminal in project directory |
| `Ctrl+e` | Launch configured CLI tool |
| `Ctrl+g` | Settings screen (select CLI tool) |
| `g` | Git screen |
| `Tab` | Toggle focus (git screen) |
| `1`/`2`/`3` | Files / Commits / Branches mode |
| `space` | Stage/unstage file |
| `t` | Stage/unstage all |
| `s` | Commit (opens commit modal) |
| `d` | Discard file (unstaged) / delete branch |
| `Enter` | View diff / checkout branch |
| `p` | Pull |
| `P` | Push |
| `f` | Fetch |
| `S` | Stash |
| `Z` | Stash pop |
| `n` | New branch |
| `q` | Back |
| `Q` | Quit |

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
