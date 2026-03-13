use crate::config::Config;
use crate::history::History;
use crate::scanner::{scan_all, scan_recursive, sort_projects, Project};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub struct App {
    pub config: Config,
    pub history: History,
    pub projects: Vec<Project>,
    pub deep_projects: Option<Vec<Project>>,
    pub filtered_indices: Vec<usize>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub filter_input: String,
    pub search_mode: bool,
    pub tmux_mode: bool,
    pub browsing_path: Option<String>,
    pub nav_stack: Vec<String>,
    pub should_quit: bool,
    pub selected_project: Option<Project>,
}

impl App {
    pub fn new(config: Config, history: History, path: Option<String>, tmux: bool) -> Self {
        let mut app = App {
            config,
            history,
            projects: Vec::new(),
            deep_projects: None,
            filtered_indices: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            filter_input: String::new(),
            search_mode: false,
            tmux_mode: tmux,
            browsing_path: path,
            nav_stack: Vec::new(),
            should_quit: false,
            selected_project: None,
        };
        app.scan();
        app
    }

    pub fn scan(&mut self) {
        self.projects = scan_all(
            &self.config,
            &self.history,
            self.browsing_path.as_deref(),
        );
        sort_projects(&mut self.projects, &self.config.sort);
        self.filter_input.clear();
        self.filtered_indices = (0..self.projects.len()).collect();
        self.selected = 0;
        self.scroll_offset = 0;
    }

    /// Returns the project list that filtering operates on:
    /// deep_projects when in search mode, otherwise the shallow projects.
    pub fn active_projects(&self) -> &[Project] {
        if self.search_mode {
            self.deep_projects.as_deref().unwrap_or(&self.projects)
        } else {
            &self.projects
        }
    }

    /// Lazily initialize deep (recursive) project scan.
    fn ensure_deep_projects(&mut self) {
        if self.deep_projects.is_none() {
            self.deep_projects = Some(scan_recursive(&self.config, &self.history));
        }
    }

    pub fn update_filter(&mut self) {
        if self.filter_input.is_empty() && !self.search_mode {
            self.filtered_indices = (0..self.projects.len()).collect();
        } else {
            // When in search mode or filtering, use deep projects
            if self.search_mode {
                self.ensure_deep_projects();
            }

            let source = if self.search_mode {
                self.deep_projects.as_deref().unwrap_or(&self.projects)
            } else {
                &self.projects as &[Project]
            };

            if self.filter_input.is_empty() {
                self.filtered_indices = (0..source.len()).collect();
            } else {
                use fuzzy_matcher::skim::SkimMatcherV2;
                use fuzzy_matcher::FuzzyMatcher;

                let matcher = SkimMatcherV2::default();
                // Match against the relative path from home for better deep search
                let home = dirs::home_dir()
                    .map(|h| h.to_string_lossy().into_owned())
                    .unwrap_or_default();

                let mut scored: Vec<(usize, i64)> = source
                    .iter()
                    .enumerate()
                    .filter_map(|(i, p)| {
                        let match_str = if self.search_mode {
                            p.path.strip_prefix(&home).unwrap_or(&p.path)
                        } else {
                            &p.name
                        };
                        matcher
                            .fuzzy_match(match_str, &self.filter_input)
                            .map(|score| (i, score))
                    })
                    .collect();

                scored.sort_by(|a, b| b.1.cmp(&a.1));
                self.filtered_indices = scored.into_iter().map(|(i, _)| i).collect();
            }
        }

        // Clamp selection
        if self.filtered_indices.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.filtered_indices.len() {
            self.selected = self.filtered_indices.len() - 1;
        }
    }

    pub fn move_selection(&mut self, delta: i32) {
        if self.filtered_indices.is_empty() {
            return;
        }
        let len = self.filtered_indices.len() as i32;
        let new_sel = (self.selected as i32 + delta).rem_euclid(len);
        self.selected = new_sel as usize;
    }

    /// Adjust scroll_offset so that `self.selected` is visible within `visible_height` rows.
    pub fn adjust_scroll(&mut self, visible_height: usize) {
        if visible_height == 0 {
            return;
        }
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + visible_height {
            self.scroll_offset = self.selected + 1 - visible_height;
        }
    }

    pub fn visible_projects(&self) -> Vec<&Project> {
        let source = self.active_projects();
        self.filtered_indices
            .iter()
            .filter_map(|&i| source.get(i))
            .collect()
    }

    pub fn select_current(&mut self) {
        if self.filtered_indices.is_empty() {
            return;
        }
        let idx = self.filtered_indices[self.selected];
        let source = if self.search_mode {
            self.deep_projects.as_deref().unwrap_or(&self.projects)
        } else {
            &self.projects
        };
        let project = match source.get(idx) {
            Some(p) => p.clone(),
            None => return,
        };

        if self.search_mode {
            // In search mode, always select directly
            self.selected_project = Some(project);
        } else if project.has_children {
            self.nav_stack.push(
                self.browsing_path
                    .clone()
                    .unwrap_or_default(),
            );
            self.browsing_path = Some(project.path.clone());
            self.scan();
        } else {
            self.selected_project = Some(project);
        }
    }

    pub fn go_back(&mut self) {
        if let Some(prev) = self.nav_stack.pop() {
            self.browsing_path = if prev.is_empty() { None } else { Some(prev) };
            self.scan();
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        // Ctrl-C always quits
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return;
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') if self.filter_input.is_empty() => {
                self.move_selection(-1);
            }
            KeyCode::Up => self.move_selection(-1),
            KeyCode::Down | KeyCode::Char('j') if self.filter_input.is_empty() => {
                self.move_selection(1);
            }
            KeyCode::Down => self.move_selection(1),
            KeyCode::Enter => self.select_current(),
            KeyCode::Backspace => {
                if self.search_mode {
                    self.filter_input.pop();
                    if self.filter_input.is_empty() {
                        // Stay in search mode but show all deep results
                    }
                    self.update_filter();
                } else if self.filter_input.is_empty() && self.browsing_path.is_some() {
                    self.go_back();
                } else {
                    self.filter_input.pop();
                    self.update_filter();
                }
            }
            KeyCode::Esc => {
                if self.search_mode {
                    self.search_mode = false;
                    self.filter_input.clear();
                    self.filtered_indices = (0..self.projects.len()).collect();
                    self.selected = 0;
                } else if !self.filter_input.is_empty() {
                    self.filter_input.clear();
                    self.update_filter();
                } else if self.browsing_path.is_some() {
                    self.go_back();
                } else {
                    self.should_quit = true;
                }
            }
            KeyCode::Char('q') if self.filter_input.is_empty() => {
                self.should_quit = true;
            }
            KeyCode::Tab if self.browsing_path.is_some() => {
                // Select the current browsing directory itself
                if let Some(ref bp) = self.browsing_path.clone() {
                    let name = std::path::Path::new(bp)
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| bp.clone());
                    self.selected_project = Some(Project {
                        name,
                        path: bp.clone(),
                        ..Default::default()
                    });
                }
            }
            KeyCode::Char('/') if self.filter_input.is_empty() && !self.search_mode => {
                self.search_mode = true;
                self.filter_input.clear();
                self.update_filter();
            }
            KeyCode::Char('h') if self.filter_input.is_empty() && !self.search_mode => {
                self.config.show_hidden = !self.config.show_hidden;
                self.deep_projects = None; // invalidate deep cache
                self.scan();
            }
            KeyCode::Char('e') if self.filter_input.is_empty() => {
                // Handled in main.rs event loop (needs terminal restore)
                // We signal via a special mechanism — but for simplicity,
                // the main loop checks for 'e' before calling handle_key.
            }
            KeyCode::Char(c) => {
                self.filter_input.push(c);
                self.update_filter();
            }
            _ => {}
        }
    }

    pub fn group_counts(&self) -> Vec<(String, usize, String)> {
        let mut groups: Vec<(String, usize, String)> = Vec::new();
        for project in &self.projects {
            if let Some(g) = groups.iter_mut().find(|g| g.0 == project.label) {
                g.1 += 1;
            } else {
                groups.push((project.label.clone(), 1, project.color.clone()));
            }
        }
        groups
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, DirEntry, SortOrder, UiConfig};
    use crate::history::History;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    fn make_key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn make_test_config(dir: &std::path::Path) -> Config {
        Config {
            directories: vec![DirEntry {
                path: dir.to_str().unwrap().to_string(),
                label: "test".into(),
                color: "cyan".into(),
            }],
            command: None,
            sort: SortOrder::Single("alphabetical".into()),
            show_hidden: false,
            cache_dir: "/tmp/qs-test-cache".into(),
            ui: UiConfig::default(),
        }
    }

    #[test]
    fn test_app_selection_flow() {
        // Create temp dir tree
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("alpha")).unwrap();
        std::fs::create_dir(dir.path().join("beta")).unwrap();
        std::fs::create_dir(dir.path().join("gamma")).unwrap();

        let config = make_test_config(dir.path());
        let history = History::default();

        let mut app = App::new(config, history, None, false);

        assert_eq!(app.projects.len(), 3);
        assert_eq!(app.selected, 0);

        // Move down
        app.handle_key(make_key(KeyCode::Down));
        assert_eq!(app.selected, 1);

        // Select
        app.handle_key(make_key(KeyCode::Enter));

        // beta has no children, so selected_project should be set
        assert!(app.selected_project.is_some());
        assert_eq!(app.selected_project.as_ref().unwrap().name, "beta");
    }

    #[test]
    fn test_app_filter() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("apple")).unwrap();
        std::fs::create_dir(dir.path().join("banana")).unwrap();
        std::fs::create_dir(dir.path().join("avocado")).unwrap();

        let config = make_test_config(dir.path());
        let history = History::default();
        let mut app = App::new(config, history, None, false);

        // Type 'a' to filter
        app.handle_key(make_key(KeyCode::Char('a')));
        assert_eq!(app.filter_input, "a");
        // "apple", "banana", "avocado" all contain 'a'
        assert_eq!(app.filtered_indices.len(), 3);

        // Type 'p' -> "ap"
        app.handle_key(make_key(KeyCode::Char('p')));
        assert_eq!(app.filtered_indices.len(), 1); // only "apple"
    }

    #[test]
    fn test_app_drill_down() {
        let dir = tempfile::tempdir().unwrap();
        let parent = dir.path().join("parent");
        std::fs::create_dir(&parent).unwrap();
        std::fs::create_dir(parent.join("child")).unwrap();

        let config = make_test_config(dir.path());
        let history = History::default();
        let mut app = App::new(config, history, None, false);

        assert_eq!(app.projects.len(), 1);
        assert!(app.projects[0].has_children);

        // Enter drills down
        app.handle_key(make_key(KeyCode::Enter));
        assert!(app.browsing_path.is_some());
        assert_eq!(app.projects.len(), 1);
        assert_eq!(app.projects[0].name, "child");

        // Go back
        app.handle_key(make_key(KeyCode::Esc));
        assert!(app.browsing_path.is_none());
        assert_eq!(app.projects.len(), 1);
        assert_eq!(app.projects[0].name, "parent");
    }

    #[test]
    fn test_app_quit() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("proj")).unwrap();

        let config = make_test_config(dir.path());
        let history = History::default();
        let mut app = App::new(config, history, None, false);

        app.handle_key(make_key(KeyCode::Char('q')));
        assert!(app.should_quit);
    }

    #[test]
    fn test_app_toggle_hidden() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("visible")).unwrap();
        std::fs::create_dir(dir.path().join(".hidden")).unwrap();

        let config = make_test_config(dir.path());
        let history = History::default();
        let mut app = App::new(config, history, None, false);

        assert_eq!(app.projects.len(), 1);

        // Toggle hidden
        app.handle_key(make_key(KeyCode::Char('h')));
        assert!(app.config.show_hidden);
        assert_eq!(app.projects.len(), 2);
    }

    #[test]
    fn test_move_wraps_around() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("a")).unwrap();
        std::fs::create_dir(dir.path().join("b")).unwrap();
        std::fs::create_dir(dir.path().join("c")).unwrap();

        let config = make_test_config(dir.path());
        let history = History::default();
        let mut app = App::new(config, history, None, false);

        assert_eq!(app.selected, 0);
        app.move_selection(-1); // wraps to end
        assert_eq!(app.selected, 2);
        app.move_selection(1); // wraps to start
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn test_group_counts() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("a")).unwrap();
        std::fs::create_dir(dir.path().join("b")).unwrap();

        let config = make_test_config(dir.path());
        let history = History::default();
        let app = App::new(config, history, None, false);

        let groups = app.group_counts();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].0, "test");
        assert_eq!(groups[0].1, 2);
    }

    #[test]
    fn test_fuzzy_filter() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("my-awesome-project")).unwrap();
        std::fs::create_dir(dir.path().join("another-thing")).unwrap();
        std::fs::create_dir(dir.path().join("map-renderer")).unwrap();

        let config = make_test_config(dir.path());
        let history = History::default();
        let mut app = App::new(config, history, None, false);

        // Fuzzy match "map" should match "my-awesome-project" and "map-renderer"
        app.filter_input = "map".to_string();
        app.update_filter();

        let visible: Vec<&str> = app
            .visible_projects()
            .iter()
            .map(|p| p.name.as_str())
            .collect();
        assert!(visible.contains(&"map-renderer")); // exact prefix match
        // "my-awesome-project" might match too (m-a-p), that's OK
        assert!(!visible.contains(&"another-thing")); // should not match
    }
}
