use std::collections::HashMap;
use crate::types::{FileEntry, Cluster};
use super::FileClusterer;

pub struct DomainClusterer;

impl FileClusterer for DomainClusterer {
    fn cluster(&self, files: &[FileEntry]) -> Vec<Cluster> {
        let mut cluster_map: HashMap<String, Vec<std::path::PathBuf>> = HashMap::new();

        for file in files {
            let domain = classify_domain(file);
            cluster_map.entry(domain).or_default().push(file.path.clone());
        }

        let mut clusters: Vec<Cluster> = cluster_map
            .into_iter()
            .map(|(name, paths)| {
                let description = match name.as_str() {
                    "Authentication" => "User authentication, authorization, and session management".to_string(),
                    "Database" => "Data models, migrations, and database access layer".to_string(),
                    "API" => "HTTP endpoints, route handlers, and API middleware".to_string(),
                    "Services" => "Business logic and application services".to_string(),
                    "Configuration" => "Application configuration and environment settings".to_string(),
                    "Frontend" => "User interface components and client-side code".to_string(),
                    "Backend" => "Core server-side application logic".to_string(),
                    "Documentation" => "Project documentation and guides".to_string(),
                    "Tests" => "Test suites and testing utilities".to_string(),
                    "Logs" => "Application logs and diagnostic output".to_string(),
                    _ => format!("{} module", name),
                };

                Cluster {
                    name: name.clone(),
                    description,
                    files: paths,
                }
            })
            .collect();

        clusters.sort_by_key(|b| std::cmp::Reverse(b.files.len()));
        clusters
    }
}

fn classify_domain(file: &FileEntry) -> String {
    let path_lower = file.path.to_string_lossy().to_lowercase();
    let ext = file.extension.as_str();

    if ext == "log" || path_lower.contains("/logs/") || path_lower.contains("\\logs\\") {
        return "Logs".to_string();
    }

    if ext == "md" || path_lower.contains("/docs/") || path_lower.contains("\\docs\\")
        || path_lower.contains("/readme") || path_lower.contains("\\readme") {
        return "Documentation".to_string();
    }

    if path_lower.contains("/tests/") || path_lower.contains("\\tests\\")
        || path_lower.ends_with("_test.rs") || path_lower.ends_with("_test.py")
        || path_lower.ends_with(".spec.ts") || path_lower.ends_with(".test.js") {
        return "Tests".to_string();
    }

    if ext == "sql" || path_lower.contains("/migration") || path_lower.contains("\\migration")
        || path_lower.contains("/db/") || path_lower.contains("\\db\\")
        || path_lower.contains("/database") || path_lower.contains("\\database")
        || ext == "prisma" {
        return "Database".to_string();
    }

    if path_lower.contains("/auth") || path_lower.contains("\\auth")
        || path_lower.contains("/login") || path_lower.contains("\\login")
        || path_lower.contains("/oauth") || path_lower.contains("\\oauth") {
        return "Authentication".to_string();
    }

    if path_lower.contains("/api") || path_lower.contains("\\api")
        || path_lower.contains("/routes") || path_lower.contains("\\routes")
        || path_lower.contains("/endpoint") || path_lower.contains("\\endpoint") {
        return "API".to_string();
    }

    if path_lower.contains("/service") || path_lower.contains("\\service") {
        return "Services".to_string();
    }

    if ext == "toml" || ext == "yaml" || ext == "yml" || ext == "json"
        || ext == "ini" || ext == "cfg" || ext == "env"
        || path_lower.contains("/config") || path_lower.contains("\\config") {
        return "Configuration".to_string();
    }

    match ext {
        "js" | "jsx" | "ts" | "tsx" | "css" | "scss" | "html" | "htm" | "vue" | "svelte" => {
            return "Frontend".to_string();
        }
        "rs" | "py" | "go" | "java" | "rb" | "php" | "cs" => {
            return "Backend".to_string();
        }
        _ => {}
    }

    "Other".to_string()
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
    fn test_domain_logs() {
        let f = make_file("/project/logs/app.log", "log");
        assert_eq!(classify_domain(&f), "Logs");
    }

    #[test]
    fn test_domain_docs() {
        let f = make_file("/project/docs/guide.md", "md");
        assert_eq!(classify_domain(&f), "Documentation");
    }

    #[test]
    fn test_domain_auth() {
        let f = make_file("/project/src/auth/login.rs", "rs");
        assert_eq!(classify_domain(&f), "Authentication");
    }

    #[test]
    fn test_domain_tests() {
        let f = make_file("/project/tests/test_main.rs", "rs");
        assert_eq!(classify_domain(&f), "Tests");
    }

    #[test]
    fn test_domain_config() {
        let f = make_file("/project/config/app.toml", "toml");
        assert_eq!(classify_domain(&f), "Configuration");
    }
}
