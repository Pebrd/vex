# Review: APPROVED ✅
**Files:** `src/ui/issues.rs`, `src/app.rs` | **Date:** 2026-05-27 | **VAULT ref:** 2026-05-27-005-review-multi-select-issues.md

## TL;DR
**Status:** APPROVED
**Detail needed:** no

Ready to deliver.

## Build/Test Report

| Check | Result |
|---|---|
| `cargo build` | ✅ passes (0 errors) |
| `cargo test` | ✅ 7 passed (1 suite) |
| `cargo fmt --check` | ✅ passes (no output) |
| `cargo clippy` | ✅ 0 errors, 63 warnings (all pre-existing, none related to this change) |

## What Changed

**`src/ui/issues.rs`** (109 lines added):
- New `MultiSelect` struct with `active` flag and `HashSet<usize>` for selected indices
- Methods: `toggle()`, `is_selected()`, `toggle_item()`, `clear()`, `selected_indices()`
- Integrated into `IssuesView` with `issues_multi` field
- `draw_issues_list` uses `.enumerate()` and conditionally renders `●`/`○` markers and selection background highlight
- 5 unit tests covering all `MultiSelect` methods

**`src/app.rs`** (156 lines changed net):
- `go_to_dashboard()`, `switch_to_issues()`, `switch_to_prs()`, `show_stats()`, `show_roadmap()` — all clear multi-select state
- `status_keys()` shows multi-select-specific help bar when active
- **`KeyCode::Char('V')`** toggles multi-select on/off
- **`KeyCode::Char(' ')`** toggles current issue selection (guarded by `active && focus == Issues`)
- **`KeyCode::Char('c')`** closes selected issues when multi-select active (else creates issue — existing behavior)
- **`KeyCode::Char('r')`** reopens selected issues when multi-select active (else refreshes — existing behavior)
- All navigation/filter/sort/search actions (`/`, `f`, `l`, `S`, search, restore, label filter) clear multi-select

## Correctness Review

| Concern | Verdict |
|---|---|
| **Selected indices stale after list mutation?** | ✅ All list-modifying actions (search, filter, sort, label filter) clear multi-select first. No path where indices go stale. |
| **Index out of bounds?** | ✅ `issues_view.issues.get(idx)` used everywhere; `selected` is bound-checked existing pattern |
| **Concurrent API calls for c/r on many issues?** | ✅ Sequential but correct; optimistic local update + `refresh_issues()` at end restores API state |
| **Space key conflicts with editing?** | ✅ Space during `EditIssue`/`EditNote` is handled in `handle_text_key` (label toggle), NOT in `handle_issues_key`. No overlap. |
| **V key conflicts with editing?** | ✅ `V` (Shift+v) during editing is swallowed by `handle_input_key`'s text input handler. Never reaches issues handler. |
| **V/space have no focus guard on V?** | ✅ `V` works from any focus but that's intended (and consistent with other toggle keys). Space is guarded by `focus == Issues`. |
| **State consistency after c/r?** | ✅ `clear()` resets both `active` and `selected`, then `refresh_issues()` replaces the full list from API. |
| **Error handling for API calls?** | ✅ Uses `let _ = ...` same as existing `toggle_issue_state` pattern; errors are non-fatal, state gets corrected by refresh. |

## Scope

Changes are strictly limited to `src/ui/issues.rs` and `src/app.rs`. No scope violations.

## Observations (non-blocking)

1. **No focus guard on `V` key** — pressing `V` while focus is on Notes toggles multi-select (and status says "exited multi-select"). Harmless since `space` is guarded by `focus == Issues`, but the UX is slightly confusing. This is consistent with how other "mode toggles" work in the codebase.
2. **Indices survive but aren't used after `clear()`** — `selected_indices()` clones into a `Vec<usize>` then `clear()` is called on line 2130/2362. The Vec holds the values, so this is correct. But readers might wonder if the clone is wasted — it's not, since `clear()` resets before `for` loop ends. (Actually re-reading: `clear()` is called AFTER the `for` loop on both paths, so the Vec is already populated. Fine.)

None of these are blockers.
