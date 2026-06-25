use std::collections::HashMap;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::{depth_first_search, DfsEvent, EdgeRef};
use petgraph::algo::dijkstra;

use crate::types::*;
use crate::architecture::detector::classify_file;

pub struct ProjectGraph {
    pub graph: KGraph,
    node_map: HashMap<String, NodeIndex>,
}

impl ProjectGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_map: HashMap::new(),
        }
    }

    pub fn from_state(state: &ProjectState) -> Self {
        let mut pg = Self::new();

        for cluster in &state.clusters {
            let cluster_id = format!("cluster:{}", cluster.name);
            let idx = pg.graph.add_node(GraphNode {
                id: cluster_id.clone(),
                kind: "cluster".to_string(),
                name: cluster.name.clone(),
                file: None,
            });
            pg.node_map.insert(cluster_id.clone(), idx);

            for file_path in &cluster.files {
                let file_id = format!("file:{}", file_path.to_string_lossy());
                if !pg.node_map.contains_key(&file_id) {
                    let idx = pg.graph.add_node(GraphNode {
                        id: file_id.clone(),
                        kind: "file".to_string(),
                        name: file_path.file_name()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_default(),
                        file: Some(file_path.clone()),
                    });
                    pg.node_map.insert(file_id.clone(), idx);
                }
                if let (Some(&cluster_idx), Some(&file_idx)) = (pg.node_map.get(&cluster_id), pg.node_map.get(&file_id)) {
                    if !pg.graph.contains_edge(cluster_idx, file_idx) {
                        pg.graph.add_edge(cluster_idx, file_idx, GraphEdge { relation: "contains".to_string() });
                    }
                }
            }
        }

        for layer in &state.architecture.layers {
            let layer_id = format!("layer:{}", layer.name);
            if !pg.node_map.contains_key(&layer_id) {
                let idx = pg.graph.add_node(GraphNode {
                    id: layer_id.clone(),
                    kind: "layer".to_string(),
                    name: layer.name.clone(),
                    file: None,
                });
                pg.node_map.insert(layer_id, idx);
            }
        }

        for file in &state.files {
            let file_id = format!("file:{}", file.path.to_string_lossy());
            if !pg.node_map.contains_key(&file_id) {
                let idx = pg.graph.add_node(GraphNode {
                    id: file_id.clone(),
                    kind: "file".to_string(),
                    name: file.path.file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default(),
                    file: Some(file.path.clone()),
                });
                pg.node_map.insert(file_id.clone(), idx);
            }
            let layer_name = classify_file(file);
            let layer_id = format!("layer:{layer_name}");
            if let (Some(&layer_idx), Some(&file_idx)) = (pg.node_map.get(&layer_id), pg.node_map.get(&file_id)) {
                if !pg.graph.contains_edge(layer_idx, file_idx) {
                    pg.graph.add_edge(layer_idx, file_idx, GraphEdge { relation: "belongs_to".to_string() });
                }
            }
        }

        pg
    }

    pub fn find_related_files(&self, file_path: &std::path::Path) -> Vec<std::path::PathBuf> {
        let file_id = format!("file:{}", file_path.to_string_lossy());
        let start = match self.node_map.get(&file_id) {
            Some(&n) => n,
            None => return Vec::new(),
        };

        let mut related = Vec::new();
        let mut visited = std::collections::HashSet::new();
        depth_first_search(&self.graph, Some(start), |event| {
            if let DfsEvent::Discover(n, _) = event {
                if n != start && visited.insert(n) {
                    if let Some(node) = self.graph.node_weight(n) {
                        if node.kind == "file" {
                            if let Some(ref p) = node.file {
                                related.push(p.clone());
                            }
                        }
                    }
                }
            }
        });
        related
    }

    pub fn find_path(&self, from_name: &str, to_name: &str) -> Option<Vec<String>> {
        let from_key = self.find_node_key(from_name)?;
        let to_key = self.find_node_key(to_name)?;
        let &from = self.node_map.get(&from_key)?;
        let &to = self.node_map.get(&to_key)?;

        let dist_map = dijkstra(&self.graph, from, Some(to), |_| 1);
        if !dist_map.contains_key(&to) {
            return None;
        }

        let mut path = Vec::new();
        let mut current = to;
        path.push(self.graph.node_weight(to)?.name.clone());
        while current != from {
            let mut found = false;
            for edge in self.graph.edges_directed(current, petgraph::Direction::Incoming) {
                let src = edge.source();
                if dist_map.contains_key(&src) && dist_map[&src] < dist_map[&current] {
                    path.push(self.graph.node_weight(src)?.name.clone());
                    current = src;
                    found = true;
                    break;
                }
            }
            if !found {
                break;
            }
        }
        path.reverse();
        Some(path)
    }

    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    fn find_node_key(&self, name: &str) -> Option<String> {
        for key in self.node_map.keys() {
            if key.ends_with(name) || self.graph.node_weight(*self.node_map.get(key)?)?.name == name {
                return Some(key.clone());
            }
        }
        None
    }

    pub fn export_dot(&self) -> String {
        format!("{}", petgraph::dot::Dot::new(&self.graph))
    }
}

impl Default for ProjectGraph {
    fn default() -> Self {
        Self::new()
    }
}
