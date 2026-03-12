use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::path::PathBuf;

// ── DirEntry ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirEntry {
    pub path: String,
    pub label: String,
    pub color: String,
}

// ── SortOrder ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum SortOrder {
    Single(String),
    Multi(Vec<String>),
}

impl Default for SortOrder {
    fn default() -> Self {
        SortOrder::Single("recent".into())
    }
}

impl<'de> Deserialize<'de> for SortOrder {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::{self, Visitor};

        struct SortOrderVisitor;

        impl<'de> Visitor<'de> for SortOrderVisitor {
            type Value = SortOrder;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "a string or array of strings")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<SortOrder, E> {
                Ok(SortOrder::Single(v.to_string()))
            }

            fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<SortOrder, A::Error> {
                let mut items = Vec::new();
                while let Some(v) = seq.next_element::<String>()? {
                    items.push(v);
                }
                Ok(SortOrder::Multi(items))
            }
        }

        deserializer.deserialize_any(SortOrderVisitor)
    }
}

impl Serialize for SortOrder {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            SortOrder::Single(s) => serializer.serialize_str(s),
            SortOrder::Multi(v) => v.serialize(serializer),
        }
    }
}

// ── UiConfig ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_ui_title")]
    pub title: String,
    #[serde(default = "default_ui_icon")]
    pub icon: String,
    #[serde(default = "default_ui_width")]
    pub width: usize,
}

fn default_ui_title() -> String {
    "Quick Select".into()
}

fn default_ui_icon() -> String {
    "📂".into()
}

fn default_ui_width() -> usize {
    25
}

impl Default for UiConfig {
    fn default() -> Self {
        UiConfig {
            title: default_ui_title(),
            icon: default_ui_icon(),
            width: default_ui_width(),
        }
    }
}

// ── Config ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub directories: Vec<DirEntry>,

    #[serde(default)]
    pub command: Option<String>,

    #[serde(default)]
    pub sort: SortOrder,

    #[serde(default)]
    pub show_hidden: bool,

    #[serde(default = "default_cache_dir")]
    pub cache_dir: String,

    #[serde(default)]
    pub ui: UiConfig,
}

fn default_cache_dir() -> String {
    "~/.cache/tmux-quickselect".into()
}

fn default_directories() -> Vec<DirEntry> {
    vec![DirEntry {
        path: "~/Git".into(),
        label: "git".into(),
        color: "cyan".into(),
    }]
}

impl Default for Config {
    fn default() -> Self {
        Config {
            directories: default_directories(),
            command: None,
            sort: SortOrder::default(),
            show_hidden: false,
            cache_dir: default_cache_dir(),
            ui: UiConfig::default(),
        }
    }
}

impl Config {
    pub fn load() -> Config {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("tmux-quickselect");

        let toml_path = config_dir.join("config.toml");
        let nuon_path = config_dir.join("config.nuon");

        if toml_path.exists() {
            let content = std::fs::read_to_string(&toml_path).unwrap_or_default();
            toml::from_str(&content).unwrap_or_else(|e| {
                eprintln!("Warning: failed to parse config.toml: {e}");
                Config::default()
            })
        } else {
            if nuon_path.exists() {
                eprintln!(
                    "Migration hint: found config.nuon but no config.toml. \
                     Please migrate your config to TOML format at {}",
                    toml_path.display()
                );
            }
            Config::default()
        }
    }

    pub fn history_path(&self) -> PathBuf {
        let expanded = expand_path(&self.cache_dir);
        PathBuf::from(expanded).join("history.json")
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

pub fn expand_path(path: &str) -> String {
    shellexpand::tilde(path).into_owned()
}

pub fn parse_color(s: &str) -> ratatui::style::Color {
    match s {
        "cyan" => ratatui::style::Color::Cyan,
        "magenta" => ratatui::style::Color::Magenta,
        "green" => ratatui::style::Color::Green,
        "yellow" => ratatui::style::Color::Yellow,
        "blue" => ratatui::style::Color::Blue,
        "red" => ratatui::style::Color::Red,
        "white" => ratatui::style::Color::White,
        _ => ratatui::style::Color::Cyan,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.sort, SortOrder::Single("recent".into()));
        assert_eq!(config.directories.len(), 1);
        assert_eq!(config.directories[0].label, "git");
        assert!(!config.show_hidden);
        assert_eq!(config.ui.width, 25);
    }

    #[test]
    fn test_parse_toml_full() {
        let toml_str = r#"
sort = "alphabetical"
command = "nvim"
show_hidden = true
cache_dir = "~/.cache/qs"

[ui]
title = "My Selector"
icon = ">"
width = 30

[[directories]]
path = "~/Git"
label = "git"
color = "cyan"

[[directories]]
path = "~/projects"
label = "proj"
color = "green"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.command, Some("nvim".into()));
        assert!(config.show_hidden);
        assert_eq!(config.directories.len(), 2);
        assert_eq!(config.ui.width, 30);
        assert_eq!(config.cache_dir, "~/.cache/qs");
    }

    #[test]
    fn test_parse_toml_minimal() {
        // Only directories required, rest should use defaults
        let toml_str = r#"
[[directories]]
path = "~/Git"
label = "git"
color = "cyan"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.sort, SortOrder::Single("recent".into()));
        assert_eq!(config.command, None);
        assert!(!config.show_hidden);
    }

    #[test]
    fn test_sort_as_list() {
        let toml_str = r#"
sort = ["label", "recent"]

[[directories]]
path = "~/Git"
label = "git"
color = "cyan"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        match config.sort {
            SortOrder::Multi(keys) => assert_eq!(keys, vec!["label", "recent"]),
            _ => panic!("expected multi sort"),
        }
    }

    #[test]
    fn test_expand_tilde() {
        let expanded = expand_path("~/Git");
        assert!(!expanded.starts_with('~'));
        assert!(expanded.ends_with("/Git"));
    }

    #[test]
    fn test_expand_no_tilde() {
        let expanded = expand_path("/absolute/path");
        assert_eq!(expanded, "/absolute/path");
    }

    #[test]
    fn test_history_path() {
        let config = Config::default();
        let hp = config.history_path();
        assert!(hp.to_str().unwrap().ends_with("history.json"));
    }

    #[test]
    fn test_parse_color() {
        assert_eq!(parse_color("cyan"), ratatui::style::Color::Cyan);
        assert_eq!(parse_color("red"), ratatui::style::Color::Red);
        assert_eq!(parse_color("unknown"), ratatui::style::Color::Cyan);
    }
}
