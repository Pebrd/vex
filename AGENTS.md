# vex — agent instructions

## Build & test

```bash
cargo build              # debug build
cargo build --release    # release with LTO, single codegen-unit, stripped
cargo test               # 3 tests (all in src/git.rs, URL parsing only)
cargo fmt && cargo clippy && cargo test   # preferred check order
```

No test framework beyond `#[test]`. No CI workflows.

## Run

```bash
cargo run                # opens TUI (requires GitHub auth)
cargo run -- add "title" --body "..." --priority high   # quick-capture note
```

GitHub token: set `token` in `~/.config/vex/config.toml`, or falls back to `gh auth token`.

## Architecture

Single binary, no workspaces. Entrypoint: `src/main.rs`.

| Path | Role |
|---|---|
| `src/app.rs` | Main app loop, state machine, event dispatch (~3.5k loc, thickest file) |
| `src/github/client.rs` | GitHub REST API client (reqwest + rustls, no native-tls) |
| `src/cache.rs` | SQLite cache at `~/.local/share/vex/cache.db` (rusqlite bundled) |
| `src/config.rs` | TOML config at `~/.config/vex/config.toml` |
| `src/git.rs` | Git remote detection (walks parent dirs for `.git/config`) |
| `src/notes.rs` | Local `.vex/*.md` notes with YAML front matter (title, priority, status, issue link) |
| `src/ui/` | TUI screens: dashboard, issues, PRs, notes, stats, roadmap, popups, file browser |

## Key details

- **Rust edition 2024** — requires Rust 1.85+
- **`src/app.rs` is the main file** — most feature changes touch this file
- **Config is TOML**, not a CLI flag or env var. Token resolution: config > `gh auth token` > empty
- **Local notes live in `.vex/notes/`** inside the project directory (git-ignored via `.vex/.gitignore`)
- **Cache is SQLite** — deleting `~/.local/share/vex/cache.db` forces a fresh API fetch
- **Mouse input is enabled by default** (`mouse_enabled: true` in config)
- **No generated code or build steps** — just `cargo build`
- **Terminal launcher** (`Ctrl+t`): auto-detects terminal emulator (gnome-terminal, kitty, alacritty, xterm, wezterm, foot, konsole) and opens a terminal in project directory
- **CLI launcher** (`Ctrl+e`): opens configured CLI tool (opencode, claude, code, gh, cursor, windsurf, claude-code) inside terminal in project directory
- **Settings screen** (`Ctrl+g`): auto-detects CLIs from PATH, select with j/k/Enter, saved to `config.selected_cli`
- **Dashboard warnings**: projects whose stored path doesn't exist show `⚠` in red
- **`e` key in Dashboard**: edit project path via file browser (`EditProjectPath` mode)

## What's different from defaults

- Uses `reqwest` with `rustls` (no native-tls dependency)
- Uses `rusqlite` with `bundled` feature (statically links SQLite)
- Release profile: `opt-level=3`, `lto=true`, `codegen-units=1`, `strip=true`
