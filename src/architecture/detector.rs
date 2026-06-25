use std::collections::HashMap;
use std::sync::LazyLock;
use regex::Regex;
use petgraph::graph::DiGraph;

use crate::error::SrrResult;
use crate::types::{FileEntry, Architecture, LayerInfo};
use super::graph;
use super::ArchitectureAnalyzer;

static IMPORT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*(?:use|import|from|require|include)\s+([\w:{}]+)").unwrap()
});

pub struct ArchitectureDetector;

impl ArchitectureAnalyzer for ArchitectureDetector {
    fn analyze(&self, files: &[FileEntry]) -> SrrResult<Architecture> {
        let mut graph = DiGraph::<String, String>::new();
        let layer_names = vec!["Frontend", "Backend", "Database", "API Layer", "Authentication", "Services", "Config", "Documentation", "Tests", "Other"];

        let mut layer_nodes = HashMap::new();
        let mut layer_files: HashMap<String, Vec<&FileEntry>> = HashMap::new();
        for layer in &layer_names {
            layer_files.entry(layer.to_string()).or_default();
        }

        for file in files {
            let layer = classify_file(file);
            layer_files.entry(layer.clone()).or_default().push(file);
        }

        for name in &layer_names {
            let key = name.to_string();
            if layer_files.get(&key).map_or(0, |v| v.len()) > 0 || *name == "Other" {
                let node = graph.add_node(key.clone());
                layer_nodes.insert(key.clone(), node);
            }
        }

        let mut dep_count: HashMap<(String, String), usize> = HashMap::new();

        for file in files {
            if let Some(ref content) = file.content {
                let source_layer = classify_file(file);
                for cap in IMPORT_RE.captures_iter(content) {
                    let imported = cap[1].to_string();
                    let target_layer = classify_import(&imported);
                    if source_layer != target_layer {
                        *dep_count.entry((source_layer.clone(), target_layer)).or_insert(0) += 1;
                    }
                }
            }
        }

        for (src, tgt) in dep_count.keys() {
            if let (Some(&src_node), Some(&tgt_node)) = (layer_nodes.get(src), layer_nodes.get(tgt)) {
                if !graph.contains_edge(src_node, tgt_node) {
                    let label = "depends_on".to_string();
                    graph.add_edge(src_node, tgt_node, label);
                }
            }
        }

        let hierarchy_text = graph::render_hierarchy(&graph, &layer_nodes);

        let layers: Vec<LayerInfo> = layer_names.iter()
            .filter_map(|name| {
                let files_in_layer = layer_files.get(*name).map(|v| v.len()).unwrap_or(0);
                if files_in_layer == 0 && *name != "Other" {
                    return None;
                }
                let techs = detect_technologies(name, files);
                Some(LayerInfo {
                    name: name.to_string(),
                    file_count: files_in_layer,
                    technologies: techs,
                })
            })
            .collect();

        let graph_dot = format!("{}", petgraph::dot::Dot::new(&graph));

        Ok(Architecture {
            layers,
            graph_dot,
            hierarchy_text,
        })
    }
}

pub fn classify_file(file: &FileEntry) -> String {
    let path = file.path.to_string_lossy().to_lowercase();

    if file.extension == "md" || path.contains("/docs/") || path.contains("\\docs\\")
        || path.contains("/readme") || path.contains("\\readme") {
        return "Documentation".to_string();
    }

    if path.contains("/tests/") || path.contains("\\tests\\")
        || path.contains("/test_") || path.contains("\\test_")
        || path.contains("_test.") || path.ends_with("_test.rs") {
        return "Tests".to_string();
    }

    if file.extension == "sql" || path.contains("/migration") || path.contains("\\migration")
        || path.contains("/migrations") || path.contains("\\migrations")
        || file.extension == "prisma" {
        return "Database".to_string();
    }

    if path.contains("/auth") || path.contains("\\auth")
        || path.contains("/login") || path.contains("\\login")
        || path.contains("/oauth") || path.contains("\\oauth")
        || path.contains("/signup") || path.contains("\\signup") {
        return "Authentication".to_string();
    }

    if path.contains("/api") || path.contains("\\api")
        || path.contains("/routes") || path.contains("\\routes")
        || path.contains("/endpoint") || path.contains("\\endpoint") {
        return "API Layer".to_string();
    }

    if path.contains("/service") || path.contains("\\service")
        || path.contains("/services") || path.contains("\\services") {
        return "Services".to_string();
    }

    if file.extension == "toml" || file.extension == "yaml" || file.extension == "yml"
        || file.extension == "json" || file.extension == "ini" || file.extension == "cfg"
        || path.contains("/config") || path.contains("\\config")
        || path.contains(".env") {
        return "Config".to_string();
    }

    match file.extension.as_str() {
        "js" | "jsx" | "ts" | "tsx" | "css" | "scss" | "less" | "html" | "htm"
        | "vue" | "svelte" | "sass" => "Frontend".to_string(),
        "rs" | "py" | "go" | "java" | "rb" | "php" | "cs" | "swift" | "kt" => "Backend".to_string(),
        _ => "Other".to_string(),
    }
}

fn classify_import(import: &str) -> String {
    let lower = import.to_lowercase();
    if lower.contains("diesel") || lower.contains("sql") || lower.contains("postgres")
        || lower.contains("mysql") || lower.contains("redis") || lower.contains("mongodb")
        || lower.contains("database") || lower.contains("db::") {
        return "Database".to_string();
    }
    if lower.contains("auth") || lower.contains("oauth") || lower.contains("jwt")
        || lower.contains("password") || lower.contains("session") || lower.contains("login") {
        return "Authentication".to_string();
    }
    if lower.contains("api") || lower.contains("route") || lower.contains("endpoint")
        || lower.contains("http") || lower.contains("rest") || lower.contains("axum")
        || lower.contains("actix") || lower.contains("rocket") || lower.contains("express")
        || lower.contains("django") || lower.contains("flask") {
        return "API Layer".to_string();
    }
    if lower.contains("react") || lower.contains("vue") || lower.contains("angular")
        || lower.contains("svelte") || lower.contains("jquery") || lower.contains("dom")
        || lower.contains("css") || lower.contains("html") || lower.contains("webpack")
        || lower.contains("babel") {
        return "Frontend".to_string();
    }
    if lower.contains("serde") || lower.contains("tokio") || lower.contains("async")
        || lower.contains("std::") || lower.contains("core::") || lower.contains("anyhow")
        || lower.contains("thiserror") || lower.contains("clap") {
        return "Backend".to_string();
    }
    "Other".to_string()
}

fn detect_technologies(layer: &str, files: &[FileEntry]) -> Vec<String> {
    let mut techs = Vec::new();
    match layer {
        "Frontend" => {
            let mut has_react = false;
            let mut has_vue = false;
            for file in files {
                let path = file.path.to_string_lossy().to_lowercase();
                if path.contains("react") || file.extension == "jsx" || file.extension == "tsx" { has_react = true; }
                if path.contains("vue") { has_vue = true; }
            }
            if has_react { techs.push("React".to_string()); }
            if has_vue { techs.push("Vue".to_string()); }
            if techs.is_empty() { techs.push("Web".to_string()); }
        }
        "Backend" => {
            techs.push("Server".to_string());
        }
        "Database" => {
            techs.push("SQL".to_string());
        }
        "API Layer" => {
            techs.push("REST".to_string());
        }
        "Authentication" => {
            techs.push("Auth".to_string());
        }
        "Services" => {
            techs.push("Business Logic".to_string());
        }
        _ => {}
    }
    techs
}

impl ArchitectureDetector {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ArchitectureDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_file(path: &str, ext: &str) -> FileEntry {
        FileEntry {
            path: PathBuf::from(path),
            relative_path: PathBuf::from(path),
            extension: ext.to_string(),
            size_bytes: 0,
            line_count: 0,
            is_binary: false,
            content: None,
            token_count: 0,
        }
    }

    #[test]
    fn test_classify_backend_rust() {
        let f = make_file("/project/src/main.rs", "rs");
        assert_eq!(classify_file(&f), "Backend");
    }

    #[test]
    fn test_classify_database_sql() {
        let f = make_file("/project/db/migrations/001.sql", "sql");
        assert_eq!(classify_file(&f), "Database");
    }

    #[test]
    fn test_classify_docs_markdown() {
        let f = make_file("/project/docs/setup.md", "md");
        assert_eq!(classify_file(&f), "Documentation");
    }

    #[test]
    fn test_classify_import_database() {
        assert_eq!(classify_import("use diesel::prelude::*"), "Database");
    }

    #[test]
    fn test_classify_import_api() {
        assert_eq!(classify_import("use axum::Router"), "API Layer");
    }
}
