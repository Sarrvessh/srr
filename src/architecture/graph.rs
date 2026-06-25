use std::collections::HashMap;
use petgraph::graph::DiGraph;
use petgraph::visit::EdgeRef;

pub fn render_hierarchy(
    graph: &DiGraph<String, String>,
    layer_nodes: &HashMap<String, petgraph::graph::NodeIndex>,
) -> String {
    if layer_nodes.is_empty() {
        return "No architecture detected.".to_string();
    }

    let mut lines = Vec::new();

    let nodes: Vec<_> = layer_nodes.iter()
        .map(|(name, &idx)| (name.clone(), idx))
        .collect();

    for (name, idx) in &nodes {
        let mut dependencies: Vec<String> = Vec::new();
        for edge in graph.edges(*idx) {
            let target = &graph[edge.target()];
            dependencies.push(target.clone());
        }

        if dependencies.is_empty() {
            lines.push(format!("  {}", name));
        } else {
            for dep in &dependencies {
                lines.push(format!("  {} → {}", name, dep));
            }
        }
    }

    lines.join("\n")
}

pub fn find_source_nodes(graph: &DiGraph<String, String>) -> Vec<petgraph::graph::NodeIndex> {
    graph.node_indices()
        .filter(|&n| graph.edges_directed(n, petgraph::Direction::Incoming).count() == 0)
        .collect()
}
