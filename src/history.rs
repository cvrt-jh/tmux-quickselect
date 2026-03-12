use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::Path;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct History {
    entries: HashMap<String, DateTime<Utc>>,
}

impl History {
    pub fn load(path: &Path) -> Self {
        let content = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => return Self::default(),
        };
        serde_json::from_str(&content).unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(path, json)
    }

    pub fn record(&mut self, path: &str) {
        self.entries.insert(path.to_string(), Utc::now());
    }

    pub fn get(&self, path: &str) -> Option<DateTime<Utc>> {
        self.entries.get(path).copied()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

pub fn format_relative(dt: DateTime<Utc>) -> String {
    let now = Utc::now();
    let delta = now.signed_duration_since(dt);

    let secs = delta.num_seconds();
    if secs < 60 {
        return "just now".to_string();
    }

    let mins = delta.num_minutes();
    if mins < 60 {
        return format!("{}m ago", mins);
    }

    let hours = delta.num_hours();
    if hours < 24 {
        return format!("{}h ago", hours);
    }

    let days = delta.num_days();
    if days < 7 {
        return format!("{}d ago", days);
    }

    let weeks = days / 7;
    format!("{}w ago", weeks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_empty_history() {
        let history = History::default();
        assert!(history.get("/some/path").is_none());
    }

    #[test]
    fn test_record_and_get() {
        let mut history = History::default();
        history.record("/foo/bar");
        assert!(history.get("/foo/bar").is_some());
    }

    #[test]
    fn test_record_updates_timestamp() {
        let mut history = History::default();
        history.record("/foo");
        let first = history.get("/foo").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        history.record("/foo");
        let second = history.get("/foo").unwrap();
        assert!(second >= first);
    }

    #[test]
    fn test_clear() {
        let mut history = History::default();
        history.record("/foo");
        history.clear();
        assert!(history.get("/foo").is_none());
    }

    #[test]
    fn test_format_relative_just_now() {
        let now = chrono::Utc::now();
        assert_eq!(format_relative(now - Duration::seconds(30)), "just now");
    }

    #[test]
    fn test_format_relative_minutes() {
        let now = chrono::Utc::now();
        assert_eq!(format_relative(now - Duration::minutes(5)), "5m ago");
    }

    #[test]
    fn test_format_relative_hours() {
        let now = chrono::Utc::now();
        assert_eq!(format_relative(now - Duration::hours(3)), "3h ago");
    }

    #[test]
    fn test_format_relative_days() {
        let now = chrono::Utc::now();
        assert_eq!(format_relative(now - Duration::days(2)), "2d ago");
    }

    #[test]
    fn test_format_relative_weeks() {
        let now = chrono::Utc::now();
        assert_eq!(format_relative(now - Duration::weeks(3)), "3w ago");
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.json");

        let mut history = History::default();
        history.record("/foo/bar");
        history.record("/baz/qux");
        history.save(&path).unwrap();

        let loaded = History::load(&path);
        assert!(loaded.get("/foo/bar").is_some());
        assert!(loaded.get("/baz/qux").is_some());
    }

    #[test]
    fn test_load_missing_file() {
        let history = History::load(std::path::Path::new("/nonexistent/history.json"));
        assert!(history.get("/anything").is_none());
    }

    #[test]
    fn test_load_corrupt_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.json");
        std::fs::write(&path, "not json").unwrap();
        let history = History::load(&path);
        assert!(history.get("/anything").is_none());
    }
}
