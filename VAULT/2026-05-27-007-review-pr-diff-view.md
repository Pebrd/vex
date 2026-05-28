# Review: CHANGES REQUIRED ❌
**Files:** `src/app.rs` | **Date:** 2026-05-27 | **VAULT ref:** 2026-05-27-007-review-pr-diff-view

## TL;DR
**Status:** BLOCKED
**Blockers:** 1 — Scroll is non-functional (j/k keys increment `detail_scroll` but the Paragraph widget never receives it)
**Detail needed:** yes

## Blocker (1)

### 1. Scroll offset not wired to Paragraph widget
**Location:** `src/app.rs:786-793`
**Problem:** `draw_pr_diff` renders a `Paragraph` for the diff content but never calls `.scroll()`, so `detail_scroll` (modified by j/k and mouse scroll) has no visual effect. Any diff longer than the visible area is clipped and unreachable.
**Fix:** Add `.scroll((self.detail_scroll, 0))` to the Paragraph builder at line 792, following the existing pattern in `src/ui/issues.rs:578-581` and `src/ui/issues.rs:639-642`.
```rust
// Current (broken):
let text = Paragraph::new(lines)
    .block(block)
    .wrap(Wrap { trim: false });

// Fix:
let text = Paragraph::new(lines)
    .block(block)
    .wrap(Wrap { trim: false })
    .scroll((self.detail_scroll, 0));
```

## Observations (non-blocking)

1. **Unnecessary `PullRequest` clone** (`src/app.rs:2589`): `self.selected_pr.clone()` clones the entire `PullRequest` (9+ String fields) when only `pr.number` (u64) is needed. To avoid the clone across the await point, extract `let number = pr.number;` before the match and pass `number` to `get_pr_diff`. Minor performance nit — no correctness impact.

2. **No `detail_scroll` reset on PrDiff enter/exit** (`src/app.rs:1648-1650`): Pressing `q` clears `pr_diff` but not `detail_scroll`. If the user returns to the diff later, the old scroll position carries over. Pre-existing pattern in the codebase; adding `self.detail_scroll = 0` on enter or exit would improve UX.

3. **`pr_diff` not cleared on `go_to_dashboard()`** (`src/app.rs:402-412`): `g` key from PrDiff navigates to dashboard but doesn't free `pr_diff`. Stale data persists; minor memory concern for large diffs.

## Build verification

| Check | Result |
|---|---|
| `cargo build` | ✅ 0 errors (1 unrelated warning in `diff.rs`) |
| `cargo test` | ✅ 10 passed |
| `cargo clippy` | ✅ 0 errors (all warnings are pre-existing) |
| `cargo fmt --check` | ✅ clean |

## Assessment

Code structure, state management, key bindings, help text, mouse handlers, and error handling all follow existing patterns correctly. The single blocker is the missing `.scroll()` call which makes the entire scrolling feature non-functional. One edit fixes it.
