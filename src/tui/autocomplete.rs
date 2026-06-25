use std::time::Instant;

#[derive(Debug, Clone)]
pub struct Autocomplete {
    pub active: bool,
    pub trigger: char,
    pub query: String,
    pub results: Vec<String>,
    pub selected: usize,
    pub cursor_pos: usize,
    file_cache: Vec<String>,
    file_cache_updated: Instant,
}

impl Default for Autocomplete {
    fn default() -> Self {
        Self {
            active: false,
            trigger: '/',
            query: String::new(),
            results: Vec::new(),
            selected: 0,
            cursor_pos: 0,
            file_cache: Vec::new(),
            file_cache_updated: Instant::now(),
        }
    }
}

impl Autocomplete {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn deactivate(&mut self) {
        self.active = false;
        self.query.clear();
        self.results.clear();
        self.selected = 0;
    }

    pub fn select_next(&mut self) {
        if !self.results.is_empty() {
            self.selected = (self.selected + 1) % self.results.len();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.results.is_empty() {
            self.selected = if self.selected == 0 {
                self.results.len() - 1
            } else {
                self.selected - 1
            };
        }
    }

    pub fn update(&mut self, input: &str, cursor: usize) {
        if cursor == 0 {
            self.deactivate();
            return;
        }
        let before = &input[..cursor.min(input.len())];
        let trigger_pos = before.rfind(['/', '@']);

        match trigger_pos {
            Some(pos) => {
                let trigger = before[pos..].chars().next().unwrap_or('/');
                let after = &before[pos + 1..];

                let is_word_start = pos == 0
                    || input[..pos].ends_with(' ')
                    || input[..pos].ends_with('\n');
                if !is_word_start {
                    self.deactivate();
                    return;
                }

                self.active = true;
                self.trigger = trigger;
                self.query = after.to_string();
                self.cursor_pos = pos;

                let q = self.query.clone();
                match trigger {
                    '/' => self.update_commands(&q),
                    '@' => self.update_files(&q),
                    _ => self.deactivate(),
                }
            }
            None => self.deactivate(),
        }
    }

    fn update_commands(&mut self, query: &str) {
        let all_commands = crate::tui::commands::all();
        let q = query.to_lowercase();
        self.results = all_commands
            .iter()
            .filter(|c| q.is_empty() || c.name.to_lowercase().contains(&q))
            .map(|c| format!("{}  — {}", c.name, c.desc))
            .collect();
        self.selected = 0;
    }

    fn update_files(&mut self, query: &str) {
        let project_path = std::env::current_dir().unwrap_or_default();
        let q = query.to_lowercase();

        // Refresh cache every 10s
        if self.file_cache.is_empty() || self.file_cache_updated.elapsed().as_secs() > 10 {
            let mut files = Vec::new();
            if let Ok(entries) = std::fs::read_dir(&project_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let name = path.to_string_lossy().to_string();
                    let rel = name
                        .strip_prefix(project_path.to_string_lossy().as_ref())
                        .unwrap_or(&name)
                        .trim_start_matches(['\\', '/'])
                        .to_string();
                    files.push(rel);
                }
            }
            files.sort();
            self.file_cache = files;
            self.file_cache_updated = Instant::now();
        }

        self.results = self.file_cache.iter()
            .filter(|rel| q.is_empty() || rel.to_lowercase().contains(&q))
            .take(20)
            .cloned()
            .collect();
        self.selected = 0;
    }

    pub fn selected_value(&self) -> Option<String> {
        self.results.get(self.selected).map(|s| {
            if self.trigger == '/' {
                s.split(' ').next().unwrap_or(s).to_string()
            } else {
                s.clone()
            }
        })
    }

    pub fn apply(&self, input: &str) -> String {
        if let Some(value) = self.selected_value() {
            let before_trigger = &input[..self.cursor_pos];
            let query_end = self.cursor_pos + 1 + self.query.len();
            let after_query = &input[query_end.min(input.len())..];
            format!("{}{} {}", before_trigger, value, after_query.trim_start())
        } else {
            input.to_string()
        }
    }
}
