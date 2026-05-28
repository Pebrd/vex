use crate::theme::Theme;
use ratatui::prelude::*;

/// The kind of a single diff line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffLineKind {
    Add,
    Delete,
    Context,
    Header,
}

/// A single line in a diff hunk.
#[derive(Debug, Clone)]
pub struct DiffLine {
    pub content: String,
    pub kind: DiffLineKind,
}

/// A parsed hunk from a unified diff.
#[derive(Debug, Clone)]
pub struct DiffHunk {
    pub header: String,
    pub old_lines: Vec<DiffLine>,
    pub new_lines: Vec<DiffLine>,
}

/// Parse a unified diff string into structured hunks.
///
/// Skips file‑header lines and parses each hunk into aligned old/new line
/// arrays. Consecutive `-` / `+` lines are paired as replacements; orphan
/// `+` lines receive a padding entry (empty Context) in the old array and
/// orphan `-` lines receive a padding entry in the new array.
pub fn parse_diff(text: &str) -> Vec<DiffHunk> {
    let lines: Vec<&str> = text.lines().collect();
    let mut hunks = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        // Skip to the next hunk header
        if !lines[i].starts_with("@@") {
            i += 1;
            continue;
        }

        let header = lines[i].to_string();
        i += 1;

        let mut old_lines: Vec<DiffLine> = Vec::new();
        let mut new_lines: Vec<DiffLine> = Vec::new();
        let mut pending_delete: Vec<String> = Vec::new();

        // Read content lines until the next @@ or end of input
        while i < lines.len() && !lines[i].starts_with("@@") {
            let content_line = lines[i];
            i += 1;

            if content_line.is_empty() {
                continue;
            }

            let first = content_line.chars().next().unwrap();
            let rest = &content_line[1..];

            match first {
                ' ' => {
                    // Flush any pending deletions first
                    for d in pending_delete.drain(..) {
                        old_lines.push(DiffLine {
                            content: d,
                            kind: DiffLineKind::Delete,
                        });
                        new_lines.push(DiffLine {
                            content: String::new(),
                            kind: DiffLineKind::Context,
                        });
                    }
                    old_lines.push(DiffLine {
                        content: rest.to_string(),
                        kind: DiffLineKind::Context,
                    });
                    new_lines.push(DiffLine {
                        content: rest.to_string(),
                        kind: DiffLineKind::Context,
                    });
                }
                '-' => {
                    pending_delete.push(rest.to_string());
                }
                '+' => {
                    if let Some(del_content) = pending_delete.first() {
                        // Pair with a pending deletion
                        old_lines.push(DiffLine {
                            content: del_content.clone(),
                            kind: DiffLineKind::Delete,
                        });
                        new_lines.push(DiffLine {
                            content: rest.to_string(),
                            kind: DiffLineKind::Add,
                        });
                        pending_delete.remove(0);
                    } else {
                        // Orphan addition — pad old side
                        old_lines.push(DiffLine {
                            content: String::new(),
                            kind: DiffLineKind::Context,
                        });
                        new_lines.push(DiffLine {
                            content: rest.to_string(),
                            kind: DiffLineKind::Add,
                        });
                    }
                }
                _ => {
                    // Any other character (e.g. "\ No newline at end of file")
                    for d in pending_delete.drain(..) {
                        old_lines.push(DiffLine {
                            content: d,
                            kind: DiffLineKind::Delete,
                        });
                        new_lines.push(DiffLine {
                            content: String::new(),
                            kind: DiffLineKind::Context,
                        });
                    }
                    old_lines.push(DiffLine {
                        content: content_line.to_string(),
                        kind: DiffLineKind::Context,
                    });
                    new_lines.push(DiffLine {
                        content: content_line.to_string(),
                        kind: DiffLineKind::Context,
                    });
                }
            }
        }

        // Flush any remaining pending deletions
        for d in pending_delete {
            old_lines.push(DiffLine {
                content: d,
                kind: DiffLineKind::Delete,
            });
            new_lines.push(DiffLine {
                content: String::new(),
                kind: DiffLineKind::Context,
            });
        }

        hunks.push(DiffHunk {
            header,
            old_lines,
            new_lines,
        });
    }

    hunks
}

/// Render parsed diff hunks into a side-by-side view.
///
/// Each hunk is rendered as:
/// - A header line (styled with `theme.accent` + BOLD)
/// - Rows of side-by-side old/new content separated by ` │ `
pub fn render_side_by_side(hunks: &[DiffHunk], theme: &Theme, panel_width: usize) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let col_width = if panel_width < 3 { 0 } else { (panel_width - 3) / 2 };

    for hunk in hunks {
        // Header line
        lines.push(Line::from(Span::styled(
            hunk.header.clone(),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )));

        let row_count = hunk.old_lines.len().max(hunk.new_lines.len());
        let separator = Span::styled(
            " │ ",
            Style::default().fg(theme.text_dim),
        );

        for i in 0..row_count {
            let old_line = hunk.old_lines.get(i);
            let new_line = hunk.new_lines.get(i);

            // Left column (old)
            let (left_content, left_kind) = match old_line {
                Some(l) => (l.content.as_str(), &l.kind),
                None => ("", &DiffLineKind::Context),
            };
            let left_style = match left_kind {
                DiffLineKind::Delete => Style::default().fg(theme.danger),
                _ => Style::default(),
            };
            let left_padded = pad_right(left_content, col_width);

            // Right column (new)
            let (right_content, right_kind) = match new_line {
                Some(l) => (l.content.as_str(), &l.kind),
                None => ("", &DiffLineKind::Context),
            };
            let right_style = match right_kind {
                DiffLineKind::Add => Style::default().fg(theme.success),
                _ => Style::default(),
            };

            lines.push(Line::from(vec![
                Span::styled(left_padded, left_style),
                separator.clone(),
                Span::styled(right_content.to_string(), right_style),
            ]));
        }
    }

    lines
}

/// Pad a string to `width` by appending spaces, or truncate if longer.
pub fn pad_right(s: &str, width: usize) -> String {
    if s.len() >= width {
        s.chars().take(width).collect()
    } else {
        let mut result = s.to_string();
        result.push_str(&" ".repeat(width - s.len()));
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_diff_empty() {
        let hunks = parse_diff("");
        assert!(hunks.is_empty());
    }

    #[test]
    fn test_parse_diff_simple() {
        let input = "\
@@ -1,3 +1,4 @@
 line1
-line2
+line2_modified
+line3
";
        let hunks = parse_diff(input);
        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].old_lines.len(), 3);
        assert_eq!(hunks[0].new_lines.len(), 3);
        assert_eq!(hunks[0].old_lines[1].kind, DiffLineKind::Delete);
        assert_eq!(hunks[0].new_lines[1].kind, DiffLineKind::Add);
        assert_eq!(hunks[0].old_lines[2].kind, DiffLineKind::Context);
        assert_eq!(hunks[0].new_lines[2].kind, DiffLineKind::Add);
    }

    #[test]
    fn test_parse_diff_with_file_header() {
        let input = "\
diff --git a/src/main.rs b/src/main.rs
index abc..def 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1 +1 @@
-old
+new
";
        let hunks = parse_diff(input);
        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].old_lines[0].kind, DiffLineKind::Delete);
        assert_eq!(hunks[0].new_lines[0].kind, DiffLineKind::Add);
    }
}
