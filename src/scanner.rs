use crate::config::{Config, expand_path, SortOrder};
use crate::history::History;
use chrono::{DateTime, Utc};
use std::cmp::Ordering;
use std::fs;
use std::path::Path;

// ── GitStatus ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum GitStatus {
    Clean,
    Dirty(usize),
}

// ── Project ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct Project {
    pub name: String,
    pub path: String,
    pub label: String,
    pub color: String,
    pub last_used: Option<DateTime<Utc>>,
    pub has_children: bool,
    pub git_status: Option<GitStatus>,
}

// ── scan_directory ────────────────────────────────────────────────────────────

/// Returns absolute paths of immediate subdirectories under `path`.
/// Skips hidden entries (starting with `.`) when `show_hidden` is false.
pub fn scan_directory(path: &str, show_hidden: bool) -> Vec<String> {
    let dir = match fs::read_dir(path) {
        Ok(d) => d,
        Err(_) => return vec![],
    };

    let mut entries: Vec<String> = dir
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();

            // Skip hidden unless allowed
            if !show_hidden && name.starts_with('.') {
                return None;
            }

            // Only include directories
            let file_type = entry.file_type().ok()?;
            if !file_type.is_dir() {
                return None;
            }

            Some(entry.path().to_string_lossy().into_owned())
        })
        .collect();

    entries.sort();
    entries
}

// ── has_subdirs ───────────────────────────────────────────────────────────────

/// Returns true if `path` contains at least one subdirectory.
pub fn has_subdirs(path: &str, show_hidden: bool) -> bool {
    let dir = match fs::read_dir(path) {
        Ok(d) => d,
        Err(_) => return false,
    };

    dir.filter_map(|e| e.ok()).any(|entry| {
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();
        if !show_hidden && name.starts_with('.') {
            return false;
        }
        entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false)
    })
}

// ── get_git_status ────────────────────────────────────────────────────────────

/// Opens the given path as a git repository and returns its status.
/// Returns None if the path is not a git repo.
pub fn get_git_status(path: &str) -> Option<GitStatus> {
    let repo = git2::Repository::open(path).ok()?;

    let statuses = repo.statuses(None).ok()?;

    let dirty_count = statuses
        .iter()
        .filter(|s| {
            let flags = s.status();
            flags != git2::Status::CURRENT && flags != git2::Status::IGNORED
        })
        .count();

    if dirty_count == 0 {
        Some(GitStatus::Clean)
    } else {
        Some(GitStatus::Dirty(dirty_count))
    }
}

// ── scan_all ──────────────────────────────────────────────────────────────────

/// The main scanning function.
///
/// - `browsing_path = Some(p)`: scan `p` as a flat directory listing and find
///   which configured dir it belongs to for label/color.
/// - `browsing_path = None`: scan all configured directories at depth 1.
///
/// Returns an *unsorted* `Vec<Project>`.
pub fn scan_all(config: &Config, history: &History, browsing_path: Option<&str>) -> Vec<Project> {
    let show_hidden = config.show_hidden;

    match browsing_path {
        Some(bp) => {
            // Find the matching configured dir for label/color
            let (label, color) = config
                .directories
                .iter()
                .find(|d| {
                    let expanded = expand_path(&d.path);
                    bp.starts_with(&expanded)
                })
                .map(|d| (d.label.clone(), d.color.clone()))
                .unwrap_or_else(|| ("browse".into(), "white".into()));

            scan_directory(bp, show_hidden)
                .into_iter()
                .map(|p| build_project(p, &label, &color, history, show_hidden))
                .collect()
        }
        None => {
            let mut projects = Vec::new();
            for dir_entry in &config.directories {
                let expanded = expand_path(&dir_entry.path);
                for child_path in scan_directory(&expanded, show_hidden) {
                    let project = build_project(
                        child_path,
                        &dir_entry.label,
                        &dir_entry.color,
                        history,
                        show_hidden,
                    );
                    projects.push(project);
                }
            }
            projects
        }
    }
}

/// Build a `Project` from a filesystem path, enriching it with history/children/git.
fn build_project(
    path: String,
    label: &str,
    color: &str,
    history: &History,
    show_hidden: bool,
) -> Project {
    let name = Path::new(&path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.clone());

    let last_used = history.get(&path);
    let has_children = has_subdirs(&path, show_hidden);
    let git_status = get_git_status(&path);

    Project {
        name,
        path,
        label: label.to_string(),
        color: color.to_string(),
        last_used,
        has_children,
        git_status,
    }
}

// ── sort_projects ─────────────────────────────────────────────────────────────

/// Sort `projects` in-place according to `sort_order`.
///
/// - `Single(key)`: sort by that single key.
/// - `Multi(keys)`: apply keys in reverse order so the *first* key wins.
pub fn sort_projects(projects: &mut [Project], sort_order: &SortOrder) {
    match sort_order {
        SortOrder::Single(key) => {
            apply_sort(projects, key);
        }
        SortOrder::Multi(keys) => {
            // Apply in reverse: last key = lowest precedence, first key = highest.
            for key in keys.iter().rev() {
                apply_sort(projects, key);
            }
        }
    }
}

fn apply_sort(projects: &mut [Project], key: &str) {
    match key {
        "recent" => projects.sort_by(cmp_recent),
        "name" | "alphabetical" => projects.sort_by(|a, b| a.name.cmp(&b.name)),
        "label" => projects.sort_by(|a, b| a.label.cmp(&b.label)),
        _ => {} // unknown key: no-op
    }
}

/// Compare two projects by recency: items with a timestamp sort before those
/// without; among timestamped items, most recent first.
fn cmp_recent(a: &Project, b: &Project) -> Ordering {
    match (&a.last_used, &b.last_used) {
        (Some(ta), Some(tb)) => tb.cmp(ta), // descending
        (Some(_), None) => Ordering::Less,  // a comes first
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SortOrder;

    #[test]
    fn test_scan_directory() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("project-a")).unwrap();
        std::fs::create_dir(dir.path().join("project-b")).unwrap();
        std::fs::create_dir(dir.path().join(".hidden")).unwrap();
        // Also create a regular file (should not appear)
        std::fs::write(dir.path().join("file.txt"), "hello").unwrap();

        let entries = scan_directory(dir.path().to_str().unwrap(), false);
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().any(|e| e.ends_with("project-a")));
        assert!(entries.iter().any(|e| e.ends_with("project-b")));

        let entries_all = scan_directory(dir.path().to_str().unwrap(), true);
        assert_eq!(entries_all.len(), 3);
    }

    #[test]
    fn test_scan_nonexistent() {
        let entries = scan_directory("/nonexistent/path/xyz", false);
        assert!(entries.is_empty());
    }

    #[test]
    fn test_has_subdirs() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("child");
        std::fs::create_dir(&sub).unwrap();
        std::fs::create_dir(sub.join("grandchild")).unwrap();

        assert!(has_subdirs(sub.to_str().unwrap(), false));
    }

    #[test]
    fn test_has_subdirs_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!has_subdirs(dir.path().to_str().unwrap(), false));
    }

    #[test]
    fn test_has_subdirs_nonexistent() {
        assert!(!has_subdirs("/nonexistent/path/xyz", false));
    }

    #[test]
    fn test_git_status_not_a_repo() {
        let dir = tempfile::tempdir().unwrap();
        assert!(get_git_status(dir.path().to_str().unwrap()).is_none());
    }

    #[test]
    fn test_git_status_clean_repo() {
        let dir = tempfile::tempdir().unwrap();
        git2::Repository::init(dir.path()).unwrap();
        let status = get_git_status(dir.path().to_str().unwrap());
        // A fresh git repo with no files is clean
        assert_eq!(status, Some(GitStatus::Clean));
    }

    #[test]
    fn test_git_status_dirty_repo() {
        let dir = tempfile::tempdir().unwrap();
        git2::Repository::init(dir.path()).unwrap();
        std::fs::write(dir.path().join("file.txt"), "hello").unwrap();
        let status = get_git_status(dir.path().to_str().unwrap());
        assert!(matches!(status, Some(GitStatus::Dirty(_))));
    }

    #[test]
    fn test_sort_recent() {
        let now = chrono::Utc::now();
        let mut projects = vec![
            Project {
                name: "b".into(),
                path: "/b".into(),
                label: "x".into(),
                color: "cyan".into(),
                last_used: None,
                ..Default::default()
            },
            Project {
                name: "a".into(),
                path: "/a".into(),
                label: "x".into(),
                color: "cyan".into(),
                last_used: Some(now),
                ..Default::default()
            },
        ];
        sort_projects(&mut projects, &SortOrder::Single("recent".into()));
        assert_eq!(projects[0].name, "a"); // recent first
    }

    #[test]
    fn test_sort_alphabetical() {
        let mut projects = vec![
            Project {
                name: "zebra".into(),
                ..Default::default()
            },
            Project {
                name: "apple".into(),
                ..Default::default()
            },
        ];
        sort_projects(&mut projects, &SortOrder::Single("alphabetical".into()));
        assert_eq!(projects[0].name, "apple");
    }

    #[test]
    fn test_sort_label() {
        let mut projects = vec![
            Project {
                name: "a".into(),
                label: "work".into(),
                ..Default::default()
            },
            Project {
                name: "b".into(),
                label: "git".into(),
                ..Default::default()
            },
        ];
        sort_projects(&mut projects, &SortOrder::Single("label".into()));
        assert_eq!(projects[0].label, "git");
    }

    #[test]
    fn test_sort_multi_key() {
        let now = chrono::Utc::now();
        let mut projects = vec![
            Project {
                name: "c".into(),
                label: "work".into(),
                last_used: Some(now),
                ..Default::default()
            },
            Project {
                name: "a".into(),
                label: "git".into(),
                last_used: None,
                ..Default::default()
            },
            Project {
                name: "b".into(),
                label: "git".into(),
                last_used: Some(now),
                ..Default::default()
            },
        ];
        // Sort by label first, then recent within each label group
        sort_projects(
            &mut projects,
            &SortOrder::Multi(vec!["label".into(), "recent".into()]),
        );
        // git group first (alphabetical by label), and within git group, "b" (has timestamp) before "a" (no timestamp)
        assert_eq!(projects[0].name, "b"); // git, recent
        assert_eq!(projects[1].name, "a"); // git, no timestamp
        assert_eq!(projects[2].name, "c"); // work
    }
}
