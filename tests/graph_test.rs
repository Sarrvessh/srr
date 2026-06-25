use std::collections::HashMap;
use srr::architecture::graph::render_hierarchy;
use petgraph::graph::DiGraph;

#[test]
fn test_render_hierarchy_empty_graph() {
    let graph: DiGraph<String, String> = DiGraph::new();
    let nodes = HashMap::new();
    let hierarchy = render_hierarchy(&graph, &nodes);
    assert_eq!(hierarchy, "No architecture detected.");
}

#[test]
fn test_render_hierarchy_single_node() {
    let mut graph: DiGraph<String, String> = DiGraph::new();
    let idx = graph.add_node("Backend".to_string());
    let mut nodes = HashMap::new();
    nodes.insert("Backend".to_string(), idx);
    let hierarchy = render_hierarchy(&graph, &nodes);
    assert!(hierarchy.contains("Backend"));
}

#[test]
fn test_render_hierarchy_with_edges() {
    let mut graph: DiGraph<String, String> = DiGraph::new();
    let api = graph.add_node("API".to_string());
    let db = graph.add_node("Database".to_string());
    graph.add_edge(api, db, "depends_on".to_string());
    let mut nodes = HashMap::new();
    nodes.insert("API".to_string(), api);
    nodes.insert("Database".to_string(), db);
    let hierarchy = render_hierarchy(&graph, &nodes);
    assert!(hierarchy.contains("API"));
    assert!(hierarchy.contains("Database"));
    assert!(hierarchy.contains("→"));
}

#[test]
fn test_render_hierarchy_single_dep_per_line() {
    let mut graph: DiGraph<String, String> = DiGraph::new();
    let a = graph.add_node("A".to_string());
    let b = graph.add_node("B".to_string());
    graph.add_edge(a, b, "".to_string());
    let mut nodes = HashMap::new();
    nodes.insert("A".to_string(), a);
    nodes.insert("B".to_string(), b);
    let hierarchy = render_hierarchy(&graph, &nodes);
    let lines: Vec<&str> = hierarchy.lines().collect();
    assert!(lines.iter().any(|l| l.contains("→")));
    assert!(lines.iter().any(|l| l.contains("B")));
}

#[test]
fn test_render_hierarchy_multiple_sources() {
    let mut graph: DiGraph<String, String> = DiGraph::new();
    let a = graph.add_node("A".to_string());
    let b = graph.add_node("B".to_string());
    let mut nodes = HashMap::new();
    nodes.insert("A".to_string(), a);
    nodes.insert("B".to_string(), b);
    let hierarchy = render_hierarchy(&graph, &nodes);
    assert!(hierarchy.contains("A"));
    assert!(hierarchy.contains("B"));
}
