# Markdown Rendering, Themes, and Multi-Select Design

> **Date:** 2026-05-27
> **Status:** Approved
> **Feature scope:** Three independent features for vex TUI

---

## 1. Markdown Rendering

### Goal
Render markdown content (issue titles, bodies, comments, notes) with proper formatting instead of raw text.

### Approach
Use `ratatui-markdown` v0.3.6 crate with `markdown` + `highlight-lang-all` features. MIT license (compatible with vex's MIT).

### Supported syntax
- Headings (h1–h6)
- Bold, italic, strikethrough, inline code
- Ordered and unordered lists
- Code blocks with syntax highlighting (tree-sitter)
- Tables
- Block quotes
- Horizontal rules
- Links (displayed as styled text)
- Images (alt text shown)
- CJK-aware wrapping

### Files affected
- **`src/ui/issues.rs`** — Replace raw `Paragraph::new(text)` with `MarkdownPreview` for issue title, body, and comments in `draw_detail()`
- **`src/ui/notes.rs`** — Replace raw `Paragraph` in note detail view
- **Side note:** `git diff` content is not markdown; syntax highlighting may be applied separately later

### Architecture
`ratatui-markdown` parses markdown via `pulldown-cmark` and emits `ratatui::text::Line`s with proper styles. No data flow changes — only the rendering layer.

---

## 2. Themes / Colors

### Goal
Allow users to customize the app's color scheme via config, with built-in presets and individual color overrides.

### Configuration
In `~/.config/vex/config.toml`:
```toml
theme = "catppuccin"          # named preset, or "terminal" for terminal defaults

# optional overrides (override individual slots of the selected preset)
[theme.overrides]
accent = "#7C3AED"
selection = "#1E88E5"
```

`theme = "terminal"` uses `Color::Reset` (inherits terminal palette). Named presets set every color slot explicitly, ignoring the terminal palette.

### Color slots
| Slot | Purpose |
|------|---------|
| `accent` | Titles, active borders, indicators |
| `selection` | Selected list item background |
| `text` | Normal text |
| `text_dim` | Secondary text, metadata |
| `background` | Main background |
| `surface` | Panel/popup background |
| `border` | Panel borders |
| `success` | Open issues, git +, stash pop |
| `danger` | Closed issues, git -, delete |
| `warning` | Git diff header, warnings |

### Built-in presets (11)
1. **terminal** — no custom colors, uses terminal palette (default)
2. **monochrome** — grayscale, professional
3. **amoled** — `#000` background, neon green accents
4. **catppuccin-mocha** — dark, soft purples and blues
5. **gruvbox-dark** — warm, retro yellow/orange
6. **dracula** — purple/pink contrast
7. **nord** — cold blues, minimalist
8. **solarized-dark** — classic, high legibility
9. **tokyo-night** — deep blue, neon violet
10. **one-dark** — Atom default, balanced blue/yellow
11. **rose-pine** — soft rose and pine, popular in neovim

### Architecture
New module `src/theme.rs` with:
- `Theme` struct with all color slots
- `ThemePreset` enum + `impl` mapping each variant to color values
- `parse_theme(config: &Config) -> Theme` — reads config section, resolves preset, applies overrides
- `Into<ratatui::style::Style>` conversions for each slot usage

The `Theme` struct is stored in `App` and passed/used in all `draw()` methods instead of hardcoded `Color::*` values.

### Files affected
- Create: `src/theme.rs`
- Modify: `src/config.rs`, `src/app.rs`, `src/ui/*.rs` (every draw method), `src/ui/mod.rs`

---

## 3. Multi-Select

### Goal
Enable selecting multiple items in list views for bulk operations.

### Approach (A: toggle mode)
- Press `V` to enter/exit multi-select mode (indicated in status bar)
- In multi-select mode: `j/k` navigates, `space` toggles selection on current item
- Selected items rendered with `selection` style + `●` marker
- Bulk operations apply to all selected items

### Scopes
| Screen | Bulk operations |
|--------|-----------------|
| **Issues** | Close, reopen, assign label |
| **PRs** | Merge all |
| **Notes** | Delete |
| **Git (Files)** | Stage/unstage all selected |
| **Git (Branches)** | Delete selected |

### Architecture
- Generic `MultiSelectState` struct holding a `HashSet<usize>` of selected indices + bool `active`
- Embedded in each view struct (IssuesView, NotesView, GitScreen, etc.)
- Only one mode active at a time (no multi-select across screens)
- Status bar shows:
  - Normal: existing keys
  - Multi-select: `"V exit select | space toggle | <action key> apply"`

### Files affected
- Create (or add to existing struct): multi-select state in `src/app.rs`, `src/ui/issues.rs`, `src/ui/notes.rs`, `src/ui/git.rs`
- Modify: `event.rs` handlers for `V` key, bulk action dispatch

---

## Implementation Order

1. **Themes** — foundational: enables colored presets for markdown, needed by other features
2. **Markdown rendering** — depends on themes for styled output
3. **Multi-Select** — fully independent, can be last
