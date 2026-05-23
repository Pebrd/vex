use anyhow::Result;
use chrono::Local;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Note {
    pub slug: String,
    pub title: String,
    pub body: Option<String>,
    pub priority: String,
    pub status: String,
    pub created_at: String,
    pub issue: Option<u64>,
}

fn notes_dir(project_dir: &Path) -> PathBuf {
    project_dir.join(".vex").join("notes")
}

fn parse_front_matter(content: &str) -> Option<(Vec<(String, String)>, String)> {
    let content = content.trim();
    if !content.starts_with("---") {
        return None;
    }
    let after_first = &content[3..];
    let end = after_first.find("\n---")?;
    let front = after_first[..end].trim();
    let body = after_first[end + 4..].trim().to_string();

    let mut fields = Vec::new();
    for line in front.lines() {
        let line = line.trim();
        if let Some(eq) = line.find(':') {
            let key = line[..eq].trim().to_string();
            let value = line[eq + 1..].trim().to_string();
            fields.push((key, value));
        }
    }
    Some((fields, body))
}

fn get_field(fields: &[(String, String)], key: &str) -> Option<String> {
    fields
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.clone())
}

fn serialize_note(note: &Note) -> String {
    let mut s = String::from("---\n");
    s.push_str(&format!("title: {}\n", note.title));
    s.push_str(&format!("status: {}\n", note.status));
    s.push_str(&format!("priority: {}\n", note.priority));
    s.push_str(&format!("created_at: {}\n", note.created_at));
    if let Some(issue) = note.issue {
        s.push_str(&format!("issue: {issue}\n"));
    }
    s.push_str("---\n");
    if let Some(ref body) = note.body {
        s.push_str(body);
        s.push('\n');
    }
    s
}

fn slugify(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .filter_map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' {
                Some(c)
            } else {
                None
            }
        })
        .collect::<String>()
        .replace(' ', "-")
        .trim_matches('-')
        .to_string()
}

fn priority_order(p: &str) -> u8 {
    match p {
        "high" => 0,
        "medium" => 1,
        _ => 2,
    }
}

fn read_note_file(path: &Path) -> Option<Note> {
    let content = std::fs::read_to_string(path).ok()?;
    let slug = path.file_stem()?.to_string_lossy().to_string();
    let (fields, body) = parse_front_matter(&content)?;

    let body = if body.is_empty() { None } else { Some(body) };
    let issue = get_field(&fields, "issue").and_then(|v| v.parse::<u64>().ok());

    Some(Note {
        slug,
        title: get_field(&fields, "title").unwrap_or_default(),
        body,
        priority: get_field(&fields, "priority").unwrap_or_else(|| "medium".to_string()),
        status: get_field(&fields, "status").unwrap_or_else(|| "open".to_string()),
        created_at: get_field(&fields, "created_at").unwrap_or_else(|| Local::now().format("%Y-%m-%d").to_string()),
        issue,
    })
}

fn ensure_notes_dir(project_dir: &Path) -> Result<PathBuf> {
    let dir = notes_dir(project_dir);
    std::fs::create_dir_all(&dir)?;
    let gitignore_path = project_dir.join(".vex").join(".gitignore");
    if !gitignore_path.exists() {
        std::fs::write(&gitignore_path, "notes/\n")?;
    }
    Ok(dir)
}

pub fn list_notes(project_dir: &Path) -> Result<Vec<Note>> {
    let dir = notes_dir(project_dir);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut notes: Vec<Note> = std::fs::read_dir(&dir)?
        .filter_map(|entry| entry.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "md").unwrap_or(false))
        .filter_map(|e| read_note_file(&e.path()))
        .collect();

    notes.sort_by(|a, b| {
        priority_order(&a.priority)
            .cmp(&priority_order(&b.priority))
            .then_with(|| b.created_at.cmp(&a.created_at))
    });
    Ok(notes)
}

pub fn create_note(
    project_dir: &Path,
    title: &str,
    body: Option<&str>,
    priority: &str,
    issue: Option<u64>,
) -> Result<Note> {
    let dir = ensure_notes_dir(project_dir)?;
    let now = Local::now().format("%Y-%m-%d").to_string();
    let base = slugify(title);
    let slug = if base.is_empty() { format!("note-{now}") } else { base };

    let mut final_slug = slug.clone();
    let mut counter = 1;
    while dir.join(format!("{final_slug}.md")).exists() {
        final_slug = format!("{slug}-{counter}");
        counter += 1;
    }

    let note = Note {
        slug: final_slug.clone(),
        title: title.to_string(),
        body: body.map(|b| b.to_string()),
        priority: priority.to_string(),
        status: "open".to_string(),
        created_at: now,
        issue,
    };

    std::fs::write(dir.join(format!("{final_slug}.md")), serialize_note(&note))?;
    Ok(note)
}

pub fn update_note(
    project_dir: &Path,
    slug: &str,
    title: &str,
    body: Option<&str>,
    priority: &str,
    status: &str,
    issue: Option<u64>,
) -> Result<()> {
    let dir = ensure_notes_dir(project_dir)?;
    let path = dir.join(format!("{slug}.md"));

    let existing = read_note_file(&path).ok_or_else(|| anyhow::anyhow!("note not found: {slug}"))?;

    let note = Note {
        slug: slug.to_string(),
        title: title.to_string(),
        body: body.map(|b| b.to_string()),
        priority: priority.to_string(),
        status: status.to_string(),
        created_at: existing.created_at.clone(),
        issue,
    };

    std::fs::write(path, serialize_note(&note))?;
    Ok(())
}

pub fn delete_note(project_dir: &Path, slug: &str) -> Result<()> {
    let path = notes_dir(project_dir).join(format!("{slug}.md"));
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

pub fn get_note(project_dir: &Path, slug: &str) -> Result<Option<Note>> {
    let path = notes_dir(project_dir).join(format!("{slug}.md"));
    if path.exists() {
        Ok(read_note_file(&path))
    } else {
        Ok(None)
    }
}
