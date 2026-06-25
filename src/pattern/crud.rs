use std::collections::HashMap;
use std::sync::LazyLock;
use regex::Regex;
use rayon::prelude::*;

use crate::types::{FileEntry, Pattern};
use super::PatternDetector;

static RE_CREATE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(fn\s+(?:create_\w+|create[A-Z]\w*)|def\s+(?:create_\w+|create[A-Z]\w*)|INSERT\s+INTO|\.insert\(|\.create\(|fn\s+new\b|CREATE\s+)").unwrap()
});
static RE_READ: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(fn\s+(?:(?:get|find|fetch|retrieve|read)_\w+|(?:get|find|fetch|retrieve|read)[A-Z]\w*)|def\s+(?:(?:get|find|fetch|retrieve|read)_\w+|(?:get|find|fetch|retrieve|read)[A-Z]\w*)|SELECT\s+|\.find\(|\.get\(|GET\s+)").unwrap()
});
static RE_UPDATE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(fn\s+(?:update_\w+|update[A-Z]\w*)|def\s+(?:update_\w+|update[A-Z]\w*)|UPDATE\s+|\.update\(|\.set\(|PUT\s+)").unwrap()
});
static RE_DELETE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(fn\s+(?:(?:delete|remove|destroy|drop)_\w+|(?:delete|remove|destroy|drop)[A-Z]\w*)|def\s+(?:(?:delete|remove|destroy|drop)_\w+|(?:delete|remove|destroy|drop)[A-Z]\w*)|DELETE\s+|DROP\s+|\.delete\(|\.remove\()").unwrap()
});

pub struct CrudDetector;

impl PatternDetector for CrudDetector {
    fn detect_patterns(&self, files: &[FileEntry]) -> Vec<Pattern> {

        let results: Vec<Vec<CrudHit>> = files
            .par_iter()
            .filter(|f| !f.is_binary && f.content.is_some())
            .filter(|f| f.content.as_ref().is_some_and(|c| c.len() <= 200_000))
            .map(|file| {
                let content = file.content.as_ref().unwrap();
                let mut hits = Vec::new();

                for cap in RE_CREATE.find_iter(content) {
                    hits.push(CrudHit { operation: "Create".to_string(), entity: extract_entity(cap.as_str()), file: file.path.clone() });
                }
                for cap in RE_READ.find_iter(content) {
                    hits.push(CrudHit { operation: "Read".to_string(), entity: extract_entity(cap.as_str()), file: file.path.clone() });
                }
                for cap in RE_UPDATE.find_iter(content) {
                    hits.push(CrudHit { operation: "Update".to_string(), entity: extract_entity(cap.as_str()), file: file.path.clone() });
                }
                for cap in RE_DELETE.find_iter(content) {
                    hits.push(CrudHit { operation: "Delete".to_string(), entity: extract_entity(cap.as_str()), file: file.path.clone() });
                }

                hits
            })
            .collect();

        let mut entity_ops: HashMap<String, Pattern> = HashMap::new();

        for hits in &results {
            for hit in hits {
                let entry = entity_ops.entry(hit.entity.clone()).or_insert_with(|| Pattern {
                    pattern_type: "CRUD".to_string(),
                    entity: hit.entity.clone(),
                    operations: Vec::new(),
                    files: Vec::new(),
                });
                if !entry.operations.contains(&hit.operation) {
                    entry.operations.push(hit.operation.clone());
                }
                if !entry.files.contains(&hit.file) {
                    entry.files.push(hit.file.clone());
                }
            }
        }

        entity_ops
            .into_values()
            .filter(|p| p.operations.len() >= 2)
            .collect()
    }
}

struct CrudHit {
    operation: String,
    entity: String,
    file: std::path::PathBuf,
}

fn extract_entity(matched: &str) -> String {
    let clean = matched.trim_start_matches(|c: char| !c.is_ascii_alphanumeric());

    if let Some(after_prefix) = clean.strip_prefix("fn ").or_else(|| clean.strip_prefix("def "))
        .or_else(|| clean.strip_prefix("class "))
    {
        let name = after_prefix.trim_end_matches(|c: char| !c.is_alphanumeric());
        if !name.is_empty() {
            if let Some(entity) = name.split('_').nth(1) {
                if !entity.is_empty() {
                    return entity.to_string();
                }
            }
            let camel_upper: String = name.chars().skip_while(|c| c.is_lowercase())
                .take_while(|c| c.is_uppercase() || c.is_lowercase())
                .collect();
            if !camel_upper.is_empty() {
                return camel_upper.to_lowercase();
            }
        }
    }

    for prefix in &["CREATE", "INSERT", "SELECT", "UPDATE", "DELETE", "DROP"] {
        if matched.to_uppercase().contains(prefix) {
            let upper = matched.to_uppercase();
            let after = upper.split(prefix).nth(1).unwrap_or("");
            let parts: Vec<&str> = after.split_whitespace().collect();
            if let Some(&entity) = parts.first() {
                if entity.len() <= 20 && entity.chars().all(|c| c.is_alphanumeric()) {
                    return entity.to_lowercase();
                }
            }
            return format!("{}_entity", prefix.to_lowercase());
        }
    }

    "unknown".to_string()
}

impl CrudDetector {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CrudDetector {
    fn default() -> Self {
        Self::new()
    }
}
