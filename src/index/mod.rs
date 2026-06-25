use std::path::Path;
use std::sync::LazyLock;
use regex::Regex;
use sha2::{Sha256, Digest};

use crate::error::SrrResult;
use crate::storage::StorageManager;
use crate::types::{FileEntry, Symbol, SymbolKind};
use crate::scanner::walker::DefaultScanner;
use crate::scanner::FileScanner;

// Rust symbol regexes
static RUST_FN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*(?:pub\s+)?(?:async\s+)?fn\s+(\w+)\s*[<(]").unwrap()
});
static RUST_STRUCT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*(?:pub\s+)?struct\s+(\w+)").unwrap()
});
static RUST_ENUM_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*(?:pub\s+)?enum\s+(\w+)").unwrap()
});
static RUST_TRAIT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*(?:pub\s+)?(?:unsafe\s+)?trait\s+(\w+)").unwrap()
});
static RUST_IMPL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*(?:pub\s+)?(?:unsafe\s+)?impl\s+(\w+)").unwrap()
});
static RUST_MOD_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*(?:pub\s+)?mod\s+(\w+)").unwrap()
});
static RUST_MACRO_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*#\[(\w+)\]").unwrap()
});
static RUST_TYPE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*(?:pub\s+)?type\s+(\w+)").unwrap()
});
static RUST_CONST_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*(?:pub\s+)?(?:const|static)\s+(\w+)").unwrap()
});
static RUST_USE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*use\s+(?:::)?([\w:]+)").unwrap()
});

// Python symbol regexes
static PY_FN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*(?:async\s+)?def\s+(\w+)\s*\(").unwrap()
});
static PY_CLASS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*class\s+(\w+)").unwrap()
});

// JS/TS symbol regexes
static JS_FN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)(?:export\s+)?(?:default\s+)?(?:async\s+)?function\s+(\w+)\s*\(").unwrap()
});
static JS_CLASS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)(?:export\s+)?(?:default\s+)?class\s+(\w+)").unwrap()
});
static JS_INTERFACE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)(?:export\s+)?interface\s+(\w+)").unwrap()
});
static JS_TYPE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)(?:export\s+)?type\s+(\w+)\s*=").unwrap()
});
static JS_ARROW_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)(?:export\s+)?(?:const|let|var)\s+(\w+)\s*=\s*(?:async\s*)?\(?").unwrap()
});

// Go symbol regexes
static GO_FN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*func\s+(?:\(\w+\s+\*?\w+\)\s+)?(\w+)\s*\(").unwrap()
});
static GO_STRUCT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*type\s+(\w+)\s+struct").unwrap()
});
static GO_INTERFACE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*type\s+(\w+)\s+interface").unwrap()
});

pub struct SemanticIndexer;

impl SemanticIndexer {
    pub fn index_project(path: &Path, exclude: &[String], respect_gitignore: bool) -> SrrResult<Vec<Symbol>> {
        let scanner = DefaultScanner;
        let files = scanner.scan(path, exclude, false, respect_gitignore)?;
        let mut all_symbols = Vec::new();
        for file in &files {
            if file.is_binary || file.content.is_none() {
                continue;
            }
            let symbols = extract_symbols(file);
            all_symbols.extend(symbols);
        }
        Ok(all_symbols)
    }

    pub fn build_index(storage: &StorageManager, path: &Path, exclude: &[String], respect_gitignore: bool, force: bool) -> SrrResult<usize> {
        if force {
            storage.clear_index()?;
            let symbols = Self::index_project(path, exclude, respect_gitignore)?;
            let checksums = compute_checksums(path, &symbols)?;
            storage.store_symbols(&symbols, &checksums)?;
            return Ok(symbols.len());
        }

        let scanner = DefaultScanner;
        let files = scanner.scan(path, exclude, false, respect_gitignore)?;
        let mut current_paths = Vec::new();
        let mut total_count = 0;

        for file in &files {
            if file.is_binary || file.content.is_none() {
                continue;
            }
            current_paths.push(file.relative_path.clone());
            let content = file.content.as_ref().unwrap();
            let mut hasher = Sha256::new();
            hasher.update(content.as_bytes());
            let hash = format!("{:x}", hasher.finalize());

            let stored = storage.get_file_checksum(&file.relative_path)?;
            if stored.as_deref() == Some(&hash) {
                continue;
            }

            let symbols = extract_symbols(file);
            let count = symbols.len();
            storage.delete_symbols_for_file(&file.relative_path)?;
            storage.store_symbols(&symbols, &[(file.relative_path.clone(), hash)])?;
            total_count += count;
        }

        storage.delete_stale_files(&current_paths)?;
        if total_count == 0 && files.iter().any(|f| !f.is_binary) {
            total_count = storage.symbol_count()? as usize;
        }
        Ok(total_count)
    }
}

fn compute_checksums(project_path: &Path, symbols: &[Symbol]) -> SrrResult<Vec<(std::path::PathBuf, String)>> {
    use std::collections::HashMap;
    let mut file_map: HashMap<std::path::PathBuf, Vec<&Symbol>> = HashMap::new();
    for sym in symbols {
        file_map.entry(sym.file_path.clone()).or_default().push(sym);
    }
    let mut checksums = Vec::new();
    for path in file_map.keys() {
        let full_path = project_path.join(path);
        if let Ok(content) = std::fs::read_to_string(&full_path) {
            let mut hasher = Sha256::new();
            hasher.update(content.as_bytes());
            let hash = format!("{:x}", hasher.finalize());
            checksums.push((path.clone(), hash));
        }
    }
    Ok(checksums)
}

pub fn extract_symbols(file: &FileEntry) -> Vec<Symbol> {
    let content = match file.content.as_ref() {
        Some(c) => c,
        None => return Vec::new(),
    };
    let ext = file.extension.as_str();
    let path = file.relative_path.clone();

    match ext {
        "rs" => extract_rust(content, &path),
        "py" => extract_python(content, &path),
        "js" | "jsx" | "ts" | "tsx" => extract_js(content, &path),
        "go" => extract_go(content, &path),
        _ => extract_generic(content, &path),
    }
}

fn make_symbol(name: &str, kind: SymbolKind, path: &Path, line: usize, col: usize, sig: Option<&str>, doc: Option<&str>) -> Symbol {
    Symbol {
        name: name.to_string(),
        kind,
        file_path: path.to_path_buf(),
        line,
        column: col,
        signature: sig.map(|s| s.to_string()),
        doc_comment: doc.map(|d| d.to_string()),
    }
}

fn extract_rust(content: &str, path: &Path) -> Vec<Symbol> {
    // Try tree-sitter first; fall back to regex
    let mut parser = tree_sitter::Parser::new();
    if parser.set_language(&tree_sitter_rust::LANGUAGE.into()).is_ok() {
        if let Some(tree) = parser.parse(content, None) {
            let mut symbols = Vec::new();
            let mut cursor = tree.walk();
            walk_ts_tree(&tree.root_node(), &mut cursor, content, path, &mut symbols, false);
            return symbols;
        }
    }
    // Fallback: regex extraction
    extract_rust_regex(content, path)
}

fn walk_ts_tree(node: &tree_sitter::Node, cursor: &mut tree_sitter::TreeCursor, content: &str, path: &Path, symbols: &mut Vec<Symbol>, inside_impl: bool) {
    let kind = node.kind();

    match kind {
        "impl_item" => {
            let type_node = node.child_by_field_name("type");
            let name = type_node.and_then(|n| n.utf8_text(content.as_bytes()).ok()).unwrap_or("impl");
            let line = node.start_position().row + 1;
            let col = node.start_position().column + 1;
            let sig = node.utf8_text(content.as_bytes()).ok().map(|s| {
                s.lines().next().unwrap_or("").to_string()
            });
            symbols.push(make_symbol(name, SymbolKind::Module, path, line, col, sig.as_deref(), None));
            // Walk children with inside_impl=true for method detection
            if cursor.goto_first_child() {
                loop {
                    walk_ts_tree(&cursor.node(), cursor, content, path, symbols, true);
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
                cursor.goto_parent();
            }
            return;
        }
        "function_item" | "associated_function" => {
            let emit_kind = if inside_impl { SymbolKind::Method } else { SymbolKind::Function };
            let name = node.child_by_field_name("name")
                .and_then(|n| n.utf8_text(content.as_bytes()).ok())
                .unwrap_or("");
            let line = node.start_position().row + 1;
            let col = node.start_position().column + 1;
            let sig = node.utf8_text(content.as_bytes()).ok().map(|s| {
                s.lines().next().unwrap_or("").chars().take(120).collect::<String>()
            });
            if !name.is_empty() {
                symbols.push(make_symbol(name, emit_kind, path, line, col, sig.as_deref(), None));
            }
        }
        "struct_item" => emit_named(node, content, path, symbols, SymbolKind::Struct),
        "enum_item" => emit_named(node, content, path, symbols, SymbolKind::Enum),
        "trait_item" => emit_named(node, content, path, symbols, SymbolKind::Trait),
        "type_item" => emit_named(node, content, path, symbols, SymbolKind::Type),
        "const_item" | "static_item" => emit_named(node, content, path, symbols, SymbolKind::Constant),
        "macro_definition" => emit_named(node, content, path, symbols, SymbolKind::Macro),
        "mod_item" => emit_named(node, content, path, symbols, SymbolKind::Module),
        "use_declaration" => {
            let line = node.start_position().row + 1;
            let col = node.start_position().column + 1;
            if let Ok(text) = node.utf8_text(content.as_bytes()) {
                let name = text.split("::").last().unwrap_or(text).trim_end_matches(';');
                let sig = text.chars().take(120).collect::<String>();
                if !name.is_empty() {
                    symbols.push(make_symbol(name, SymbolKind::Import, path, line, col, Some(&sig), None));
                }
            }
        }
        _ => {}
    }

    // Recurse into children
    if cursor.goto_first_child() {
        loop {
            walk_ts_tree(&cursor.node(), cursor, content, path, symbols, inside_impl);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

fn emit_named(node: &tree_sitter::Node, content: &str, path: &Path, symbols: &mut Vec<Symbol>, kind: SymbolKind) {
    let name = node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(content.as_bytes()).ok())
        .unwrap_or("");
    let line = node.start_position().row + 1;
    let col = node.start_position().column + 1;
    let sig = node.utf8_text(content.as_bytes()).ok().map(|s| {
        s.lines().next().unwrap_or("").chars().take(120).collect::<String>()
    });
    if !name.is_empty() {
        symbols.push(make_symbol(name, kind, path, line, col, sig.as_deref(), None));
    }
}

fn extract_rust_regex(content: &str, path: &Path) -> Vec<Symbol> {
    let mut symbols = Vec::new();
    for cap in RUST_FN_RE.captures_iter(content) {
        let line = content[..cap.get(0).unwrap().start()].matches('\n').count() + 1;
        symbols.push(make_symbol(&cap[1], SymbolKind::Function, path, line, 0, None, None));
    }
    for cap in RUST_STRUCT_RE.captures_iter(content) {
        let line = content[..cap.get(0).unwrap().start()].matches('\n').count() + 1;
        symbols.push(make_symbol(&cap[1], SymbolKind::Struct, path, line, 0, None, None));
    }
    for cap in RUST_ENUM_RE.captures_iter(content) {
        let line = content[..cap.get(0).unwrap().start()].matches('\n').count() + 1;
        symbols.push(make_symbol(&cap[1], SymbolKind::Enum, path, line, 0, None, None));
    }
    for cap in RUST_TRAIT_RE.captures_iter(content) {
        let line = content[..cap.get(0).unwrap().start()].matches('\n').count() + 1;
        symbols.push(make_symbol(&cap[1], SymbolKind::Trait, path, line, 0, None, None));
    }
    for cap in RUST_IMPL_RE.captures_iter(content) {
        let line = content[..cap.get(0).unwrap().start()].matches('\n').count() + 1;
        symbols.push(make_symbol(&cap[1], SymbolKind::Module, path, line, 0, None, None));
    }
    for cap in RUST_MOD_RE.captures_iter(content) {
        let line = content[..cap.get(0).unwrap().start()].matches('\n').count() + 1;
        symbols.push(make_symbol(&cap[1], SymbolKind::Module, path, line, 0, None, None));
    }
    for cap in RUST_TYPE_RE.captures_iter(content) {
        let line = content[..cap.get(0).unwrap().start()].matches('\n').count() + 1;
        symbols.push(make_symbol(&cap[1], SymbolKind::Type, path, line, 0, None, None));
    }
    for cap in RUST_CONST_RE.captures_iter(content) {
        let line = content[..cap.get(0).unwrap().start()].matches('\n').count() + 1;
        symbols.push(make_symbol(&cap[1], SymbolKind::Constant, path, line, 0, None, None));
    }
    for cap in RUST_MACRO_RE.captures_iter(content) {
        let line = content[..cap.get(0).unwrap().start()].matches('\n').count() + 1;
        symbols.push(make_symbol(&cap[1], SymbolKind::Macro, path, line, 0, Some(&cap[0]), None));
    }
    for cap in RUST_USE_RE.captures_iter(content) {
        let line = content[..cap.get(0).unwrap().start()].matches('\n').count() + 1;
        let name = cap[1].split("::").last().unwrap_or(&cap[1]);
        symbols.push(make_symbol(name, SymbolKind::Import, path, line, 0, Some(&cap[1]), None));
    }
    symbols
}

fn extract_python_ts(content: &str, path: &Path) -> Option<Vec<Symbol>> {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&tree_sitter_python::LANGUAGE.into()).ok()?;
    let tree = parser.parse(content, None)?;
    let mut symbols = Vec::new();
    let mut cursor = tree.walk();
    walk_py_tree(&tree.root_node(), &mut cursor, content, path, &mut symbols);
    Some(symbols)
}

fn walk_py_tree(node: &tree_sitter::Node, cursor: &mut tree_sitter::TreeCursor, content: &str, path: &Path, symbols: &mut Vec<Symbol>) {
    match node.kind() {
        "function_definition" => {
            emit_named(node, content, path, symbols, SymbolKind::Function);
        }
        "class_definition" => {
            emit_named(node, content, path, symbols, SymbolKind::Class);
            if cursor.goto_first_child() {
                loop {
                    walk_py_tree(&cursor.node(), cursor, content, path, symbols);
                    if !cursor.goto_next_sibling() { break; }
                }
                cursor.goto_parent();
            }
            return;
        }
        "decorated_definition" => {
            if cursor.goto_first_child() {
                loop {
                    walk_py_tree(&cursor.node(), cursor, content, path, symbols);
                    if !cursor.goto_next_sibling() { break; }
                }
                cursor.goto_parent();
            }
            return;
        }
        _ => {}
    }
    if cursor.goto_first_child() {
        loop {
            walk_py_tree(&cursor.node(), cursor, content, path, symbols);
            if !cursor.goto_next_sibling() { break; }
        }
        cursor.goto_parent();
    }
}

fn extract_javascript_ts(content: &str, path: &Path) -> Option<Vec<Symbol>> {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&tree_sitter_javascript::LANGUAGE.into()).ok()?;
    let tree = parser.parse(content, None)?;
    let mut symbols = Vec::new();
    let mut cursor = tree.walk();
    walk_js_tree(&tree.root_node(), &mut cursor, content, path, &mut symbols);
    Some(symbols)
}

fn walk_js_tree(node: &tree_sitter::Node, cursor: &mut tree_sitter::TreeCursor, content: &str, path: &Path, symbols: &mut Vec<Symbol>) {
    match node.kind() {
        "function_declaration" => {
            emit_named(node, content, path, symbols, SymbolKind::Function);
        }
        "class_declaration" => {
            emit_named(node, content, path, symbols, SymbolKind::Class);
            if cursor.goto_first_child() {
                loop {
                    walk_js_tree(&cursor.node(), cursor, content, path, symbols);
                    if !cursor.goto_next_sibling() { break; }
                }
                cursor.goto_parent();
            }
            return;
        }
        "method_definition" => {
            emit_named(node, content, path, symbols, SymbolKind::Method);
        }
        "arrow_function" => {
            let parent = node.parent();
            if let Some(p) = parent {
                if p.kind() == "variable_declarator" {
                    if let Some(name_node) = p.child_by_field_name("name") {
                        if let Ok(name) = name_node.utf8_text(content.as_bytes()) {
                            let line = node.start_position().row + 1;
                            let col = node.start_position().column + 1;
                            let sig = node.utf8_text(content.as_bytes()).ok().map(|s| s.chars().take(120).collect::<String>());
                            if !name.is_empty() {
                                symbols.push(make_symbol(name, SymbolKind::Function, path, line, col, sig.as_deref(), None));
                            }
                        }
                    }
                }
            }
        }
        "export_statement" => {
            if cursor.goto_first_child() {
                loop {
                    walk_js_tree(&cursor.node(), cursor, content, path, symbols);
                    if !cursor.goto_next_sibling() { break; }
                }
                cursor.goto_parent();
            }
            return;
        }
        "lexical_declaration" | "variable_declaration" => {
            if cursor.goto_first_child() {
                loop {
                    let child = cursor.node();
                    if child.kind() == "variable_declarator" {
                        if let Some(name_node) = child.child_by_field_name("name") {
                            if let Ok(name) = name_node.utf8_text(content.as_bytes()) {
                                let line = child.start_position().row + 1;
                                let col = child.start_position().column + 1;
                                if !name.is_empty() {
                                    symbols.push(make_symbol(name, SymbolKind::Variable, path, line, col, None, None));
                                }
                            }
                        }
                    }
                    if !cursor.goto_next_sibling() { break; }
                }
                cursor.goto_parent();
            }
            return;
        }
        _ => {}
    }
    if cursor.goto_first_child() {
        loop {
            walk_js_tree(&cursor.node(), cursor, content, path, symbols);
            if !cursor.goto_next_sibling() { break; }
        }
        cursor.goto_parent();
    }
}

fn extract_go_ts(content: &str, path: &Path) -> Option<Vec<Symbol>> {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&tree_sitter_go::LANGUAGE.into()).ok()?;
    let tree = parser.parse(content, None)?;
    let mut symbols = Vec::new();
    let mut cursor = tree.walk();
    walk_go_tree(&tree.root_node(), &mut cursor, content, path, &mut symbols);
    Some(symbols)
}

fn walk_go_tree(node: &tree_sitter::Node, cursor: &mut tree_sitter::TreeCursor, content: &str, path: &Path, symbols: &mut Vec<Symbol>) {
    match node.kind() {
        "function_declaration" => {
            emit_named(node, content, path, symbols, SymbolKind::Function);
        }
        "method_declaration" => {
            emit_named(node, content, path, symbols, SymbolKind::Method);
        }
        "type_declaration" => {
            if cursor.goto_first_child() {
                loop {
                    let child = cursor.node();
                    if child.kind() == "type_spec" {
                        if let Some(name_node) = child.child_by_field_name("name") {
                            if let Ok(name) = name_node.utf8_text(content.as_bytes()) {
                                let kind = if child.child_by_field_name("type").map(|t| t.kind()) == Some("struct_type") {
                                    SymbolKind::Struct
                                } else if child.child_by_field_name("type").map(|t| t.kind()) == Some("interface_type") {
                                    SymbolKind::Interface
                                } else {
                                    SymbolKind::Type
                                };
                                let line = child.start_position().row + 1;
                                let col = child.start_position().column + 1;
                                if !name.is_empty() {
                                    symbols.push(make_symbol(name, kind, path, line, col, None, None));
                                }
                            }
                        }
                    }
                    if !cursor.goto_next_sibling() { break; }
                }
                cursor.goto_parent();
            }
            return;
        }
        _ => {}
    }
    if cursor.goto_first_child() {
        loop {
            walk_go_tree(&cursor.node(), cursor, content, path, symbols);
            if !cursor.goto_next_sibling() { break; }
        }
        cursor.goto_parent();
    }
}

fn extract_python(content: &str, path: &Path) -> Vec<Symbol> {
    if let Some(symbols) = extract_python_ts(content, path) {
        return symbols;
    }
    let mut symbols = Vec::new();
    for cap in PY_CLASS_RE.captures_iter(content) {
        let line = content[..cap.get(0).unwrap().start()].matches('\n').count() + 1;
        symbols.push(make_symbol(&cap[1], SymbolKind::Class, path, line, 0, None, None));
    }
    for cap in PY_FN_RE.captures_iter(content) {
        let line = content[..cap.get(0).unwrap().start()].matches('\n').count() + 1;
        symbols.push(make_symbol(&cap[1], SymbolKind::Function, path, line, 0, None, None));
    }
    symbols
}

fn extract_js(content: &str, path: &Path) -> Vec<Symbol> {
    if let Some(symbols) = extract_javascript_ts(content, path) {
        return symbols;
    }
    let mut symbols = Vec::new();
    for cap in JS_FN_RE.captures_iter(content) {
        let line = content[..cap.get(0).unwrap().start()].matches('\n').count() + 1;
        symbols.push(make_symbol(&cap[1], SymbolKind::Function, path, line, 0, None, None));
    }
    for cap in JS_CLASS_RE.captures_iter(content) {
        let line = content[..cap.get(0).unwrap().start()].matches('\n').count() + 1;
        symbols.push(make_symbol(&cap[1], SymbolKind::Class, path, line, 0, None, None));
    }
    for cap in JS_INTERFACE_RE.captures_iter(content) {
        let line = content[..cap.get(0).unwrap().start()].matches('\n').count() + 1;
        symbols.push(make_symbol(&cap[1], SymbolKind::Interface, path, line, 0, None, None));
    }
    for cap in JS_TYPE_RE.captures_iter(content) {
        let line = content[..cap.get(0).unwrap().start()].matches('\n').count() + 1;
        symbols.push(make_symbol(&cap[1], SymbolKind::Type, path, line, 0, None, None));
    }
    for cap in JS_ARROW_RE.captures_iter(content) {
        let line = content[..cap.get(0).unwrap().start()].matches('\n').count() + 1;
        symbols.push(make_symbol(&cap[1], SymbolKind::Variable, path, line, 0, None, None));
    }
    symbols
}

fn extract_go(content: &str, path: &Path) -> Vec<Symbol> {
    if let Some(symbols) = extract_go_ts(content, path) {
        return symbols;
    }
    let mut symbols = Vec::new();
    for cap in GO_FN_RE.captures_iter(content) {
        let line = content[..cap.get(0).unwrap().start()].matches('\n').count() + 1;
        symbols.push(make_symbol(&cap[1], SymbolKind::Function, path, line, 0, None, None));
    }
    for cap in GO_STRUCT_RE.captures_iter(content) {
        let line = content[..cap.get(0).unwrap().start()].matches('\n').count() + 1;
        symbols.push(make_symbol(&cap[1], SymbolKind::Struct, path, line, 0, None, None));
    }
    for cap in GO_INTERFACE_RE.captures_iter(content) {
        let line = content[..cap.get(0).unwrap().start()].matches('\n').count() + 1;
        symbols.push(make_symbol(&cap[1], SymbolKind::Interface, path, line, 0, None, None));
    }
    symbols
}

pub fn start_watcher(storage: &StorageManager, path: &Path, exclude: &[String]) -> SrrResult<()> {
    use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    let (tx, rx) = mpsc::channel();
    let mut watcher = RecommendedWatcher::new(tx, Config::default())?;
    watcher.watch(path, RecursiveMode::Recursive)?;

    println!("  {} Watching {} for changes (Ctrl+C to stop)...", crate::ui::symbols::INFO, path.display());

    let path_buf = path.to_path_buf();
    let exclude_owned = exclude.to_vec();
    let mut last_index = Instant::now();
    let debounce = Duration::from_millis(300);

    loop {
        match rx.recv() {
            Ok(Ok(event)) => {
                let should_reindex = matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_));
                if !should_reindex || last_index.elapsed() < debounce {
                    continue;
                }
                // Check none of the affected paths are excluded
                let excluded = event.paths.iter().any(|p| {
                    let rel = p.strip_prefix(&path_buf).unwrap_or(p);
                    let rel_str = rel.to_string_lossy();
                    exclude_owned.iter().any(|e| rel_str.starts_with(e) || rel_str.contains(e))
                });
                if excluded {
                    continue;
                }
                if let Err(e) = SemanticIndexer::build_index(storage, &path_buf, &exclude_owned, true, false) {
                    eprintln!("  {} Re-index error: {e}", crate::ui::symbols::ERROR);
                }
                last_index = Instant::now();
            }
            Ok(Err(e)) => {
                eprintln!("  {} Watch error: {e}", crate::ui::symbols::ERROR);
            }
            Err(mpsc::RecvError) => break,
        }
    }
    Ok(())
}

fn extract_generic(content: &str, path: &Path) -> Vec<Symbol> {
    let mut symbols = Vec::new();
    let re_fn = LazyLock::new(|| Regex::new(r"(?m)^\s*(?:export\s+)?(?:function|def|fn|func)\s+(\w+)").unwrap());
    let re_class = LazyLock::new(|| Regex::new(r"(?m)^\s*(?:export\s+)?(?:class|struct|interface)\s+(\w+)").unwrap());
    for cap in re_fn.captures_iter(content) {
        let line = content[..cap.get(0).unwrap().start()].matches('\n').count() + 1;
        symbols.push(make_symbol(&cap[1], SymbolKind::Function, path, line, 0, None, None));
    }
    for cap in re_class.captures_iter(content) {
        let line = content[..cap.get(0).unwrap().start()].matches('\n').count() + 1;
        symbols.push(make_symbol(&cap[1], SymbolKind::Class, path, line, 0, None, None));
    }
    symbols
}
