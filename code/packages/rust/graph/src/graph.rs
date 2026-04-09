use crate::errors::GraphError;
use std::collections::{HashMap, HashSet, VecDeque};

/// An undirected graph data structure.
///
/// The graph uses an adjacency map representation internally. Each node maps
/// to a set of its neighbors.
pub struct Graph {
    /// Adjacency map: node -> set of neighbors
    adjacency: HashMap<String, HashSet<String>>,
}

impl Graph {
    /// Creates a new empty graph.
    pub fn new() -> Self {
        Graph {
            adjacency: HashMap::new(),
        }
    }

    /// Adds a node to the graph.
    pub fn add_node(&mut self, node: &str) {
        self.adjacency
            .entry(node.to_string())
            .or_insert_with(HashSet::new);
    }

    /// Removes a node and all its edges from the graph.
    pub fn remove_node(&mut self, node: &str) -> Result<(), GraphError> {
        if !self.has_node(node) {
            return Err(GraphError::NodeNotFound(node.to_string()));
        }

        // Remove all edges connected to this node
        if let Some(neighbors) = self.adjacency.get(node) {
            let neighbors: Vec<String> = neighbors.iter().cloned().collect();
            for neighbor in neighbors {
                self.adjacency
                    .get_mut(&neighbor)
                    .map(|set| set.remove(node));
            }
        }

        // Remove the node itself
        self.adjacency.remove(node);
        Ok(())
    }

    /// Returns true if the node exists in the graph.
    pub fn has_node(&self, node: &str) -> bool {
        self.adjacency.contains_key(node)
    }

    /// Returns all nodes in the graph.
    pub fn nodes(&self) -> Vec<String> {
        let mut nodes: Vec<String> = self.adjacency.keys().cloned().collect();
        nodes.sort();
        nodes
    }

    /// Returns the number of nodes in the graph.
    pub fn size(&self) -> usize {
        self.adjacency.len()
    }

    /// Adds an edge between two nodes. Creates nodes if they don't exist.
    pub fn add_edge(&mut self, from: &str, to: &str) -> Result<(), GraphError> {
        if from == to {
            return Err(GraphError::SelfLoop(from.to_string()));
        }

        // Ensure both nodes exist
        self.add_node(from);
        self.add_node(to);

        // Add edge in both directions (undirected)
        self.adjacency
            .get_mut(from)
            .map(|set| set.insert(to.to_string()));
        self.adjacency
            .get_mut(to)
            .map(|set| set.insert(from.to_string()));

        Ok(())
    }

    /// Removes an edge between two nodes.
    pub fn remove_edge(&mut self, from: &str, to: &str) -> Result<(), GraphError> {
        if !self.has_edge(from, to) {
            return Err(GraphError::EdgeNotFound(from.to_string(), to.to_string()));
        }

        self.adjacency
            .get_mut(from)
            .map(|set| set.remove(to));
        self.adjacency
            .get_mut(to)
            .map(|set| set.remove(from));

        Ok(())
    }

    /// Returns true if an edge exists between two nodes.
    pub fn has_edge(&self, from: &str, to: &str) -> bool {
        self.adjacency
            .get(from)
            .map(|neighbors| neighbors.contains(to))
            .unwrap_or(false)
    }

    /// Returns all edges in the graph.
    pub fn edges(&self) -> Vec<(String, String)> {
        let mut edges = Vec::new();
        let mut seen = HashSet::new();

        for (from, neighbors) in &self.adjacency {
            for to in neighbors {
                // For undirected graphs, only add each edge once
                let edge = if from < to {
                    (from.clone(), to.clone())
                } else {
                    (to.clone(), from.clone())
                };

                if !seen.contains(&edge) {
                    edges.push(edge.clone());
                    seen.insert(edge);
                }
            }
        }

        edges.sort();
        edges
    }

    /// Returns all neighbors of a node.
    pub fn neighbors(&self, node: &str) -> Result<Vec<String>, GraphError> {
        if !self.has_node(node) {
            return Err(GraphError::NodeNotFound(node.to_string()));
        }

        let mut neighbors: Vec<String> = self
            .adjacency
            .get(node)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default();
        neighbors.sort();
        Ok(neighbors)
    }

    /// Returns the degree (number of neighbors) of a node.
    pub fn degree(&self, node: &str) -> Result<usize, GraphError> {
        if !self.has_node(node) {
            return Err(GraphError::NodeNotFound(node.to_string()));
        }

        Ok(self
            .adjacency
            .get(node)
            .map(|set| set.len())
            .unwrap_or(0))
    }
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_graph_is_empty() {
        let g = Graph::new();
        assert_eq!(g.size(), 0);
    }

    #[test]
    fn test_add_node() {
        let mut g = Graph::new();
        g.add_node("A");
        assert!(g.has_node("A"));
        assert_eq!(g.size(), 1);
    }

    #[test]
    fn test_add_edge_creates_nodes() {
        let mut g = Graph::new();
        g.add_edge("A", "B").unwrap();
        assert!(g.has_node("A"));
        assert!(g.has_node("B"));
        assert!(g.has_edge("A", "B"));
        assert!(g.has_edge("B", "A")); // undirected
    }

    #[test]
    fn test_self_loop_error() {
        let mut g = Graph::new();
        let result = g.add_edge("A", "A");
        assert!(matches!(result, Err(GraphError::SelfLoop(_))));
    }

    #[test]
    fn test_neighbors() {
        let mut g = Graph::new();
        g.add_edge("A", "B").unwrap();
        g.add_edge("A", "C").unwrap();
        let neighbors = g.neighbors("A").unwrap();
        assert_eq!(neighbors.len(), 2);
        assert!(neighbors.contains(&"B".to_string()));
        assert!(neighbors.contains(&"C".to_string()));
    }

    #[test]
    fn test_degree() {
        let mut g = Graph::new();
        g.add_edge("A", "B").unwrap();
        g.add_edge("A", "C").unwrap();
        assert_eq!(g.degree("A").unwrap(), 2);
        assert_eq!(g.degree("B").unwrap(), 1);
    }

    #[test]
    fn test_remove_edge() {
        let mut g = Graph::new();
        g.add_edge("A", "B").unwrap();
        g.remove_edge("A", "B").unwrap();
        assert!(!g.has_edge("A", "B"));
    }

    #[test]
    fn test_remove_node() {
        let mut g = Graph::new();
        g.add_edge("A", "B").unwrap();
        g.add_edge("A", "C").unwrap();
        g.remove_node("A").unwrap();
        assert!(!g.has_node("A"));
        assert_eq!(g.neighbors("B").unwrap().len(), 0);
        assert_eq!(g.neighbors("C").unwrap().len(), 0);
    }

    #[test]
    fn test_nodes() {
        let mut g = Graph::new();
        g.add_node("C");
        g.add_node("A");
        g.add_node("B");
        let nodes = g.nodes();
        assert_eq!(nodes, vec!["A", "B", "C"]);
    }

    #[test]
    fn test_edges() {
        let mut g = Graph::new();
        g.add_edge("A", "B").unwrap();
        g.add_edge("B", "C").unwrap();
        let edges = g.edges();
        assert_eq!(edges.len(), 2);
    }
}
