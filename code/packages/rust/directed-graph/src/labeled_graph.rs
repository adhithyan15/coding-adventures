// labeled_graph.rs -- Labeled Directed Graph
// ============================================
//
// A `LabeledDirectedGraph` extends the basic directed graph with edge labels.
// Each edge can carry one or more string labels, turning the graph into a
// "multigraph-like" structure where the same pair of nodes can be connected
// by edges with different semantic meanings.
//
// # Why labeled edges?
//
// In a build system, you might want to distinguish between different kinds
// of dependencies:
//
//   - "compile" dependency: package A needs package B at compile time
//   - "test"    dependency: package A needs package B only for testing
//   - "runtime" dependency: package A needs package B at runtime
//
// With labeled edges, you can query "what are A's compile-time dependencies?"
// without conflating them with test-only dependencies.
//
// # Architecture: composition over inheritance
//
// Rather than duplicating the adjacency-map logic from `Graph`, this struct
// wraps a `Graph` and adds a label map on top:
//
//     ┌───────────────────────────────────────────────────────┐
//     │ LabeledDirectedGraph                                  │
//     │                                                       │
//     │  ┌────────────────────────┐                           │
//     │  │ graph: Graph           │  ← handles nodes, edges,  │
//     │  │   forward adjacency    │    algorithms              │
//     │  │   reverse adjacency    │                           │
//     │  └────────────────────────┘                           │
//     │                                                       │
//     │  labels: HashMap<(String, String), HashSet<String>>   │
//     │    (from, to) → set of label strings                  │
//     │                                                       │
//     └───────────────────────────────────────────────────────┘
//
// The underlying `Graph` stores a single edge between any two nodes,
// regardless of how many labels that edge carries. When all labels
// for an edge are removed, the underlying edge is also removed.
//
// All algorithm methods (topological_sort, has_cycle, transitive_closure,
// etc.) delegate to the underlying Graph, so they work identically.

use std::collections::{HashMap, HashSet};

use crate::graph::{Graph, GraphError};

/// A directed graph where each edge carries one or more string labels.
///
/// Internally, it wraps a [`Graph`] (for nodes, edges, and algorithms) and
/// adds a label map keyed by `(from, to)` pairs.
///
/// # Example
///
/// ```
/// use directed_graph::LabeledDirectedGraph;
///
/// let mut lg = LabeledDirectedGraph::new();
/// lg.add_edge("A", "B", "compile").unwrap();
/// lg.add_edge("A", "B", "test").unwrap();
/// assert!(lg.has_edge_with_label("A", "B", "compile"));
/// assert!(lg.has_edge_with_label("A", "B", "test"));
/// ```
pub struct LabeledDirectedGraph {
    /// The underlying unlabeled graph that handles structural operations.
    graph: Graph,
    /// Edge labels: maps (from, to) pairs to a set of label strings.
    labels: HashMap<(String, String), HashSet<String>>,
}

impl LabeledDirectedGraph {
    // ------------------------------------------------------------------
    // Constructors
    // ------------------------------------------------------------------

    /// Create an empty labeled directed graph that prohibits self-loops.
    pub fn new() -> Self {
        LabeledDirectedGraph {
            graph: Graph::new(),
            labels: HashMap::new(),
        }
    }

    /// Create an empty labeled directed graph that permits self-loops.
    pub fn new_allow_self_loops() -> Self {
        LabeledDirectedGraph {
            graph: Graph::new_allow_self_loops(),
            labels: HashMap::new(),
        }
    }

    // ------------------------------------------------------------------
    // Node operations
    // ------------------------------------------------------------------
    // These delegate directly to the underlying Graph.

    /// Add a node to the graph. No-op if the node already exists.
    pub fn add_node(&mut self, node: &str) {
        self.graph.add_node(node);
    }

    /// Remove a node and all its incident edges (including labels).
    ///
    /// When a node is removed, we must also clean up any label entries for
    /// edges that touched that node. We collect matching keys first to
    /// avoid mutating the map during iteration.
    ///
    /// Returns [`GraphError::NodeNotFound`] if the node doesn't exist.
    pub fn remove_node(&mut self, node: &str) -> Result<(), GraphError> {
        if !self.graph.has_node(node) {
            return Err(GraphError::NodeNotFound(node.to_string()));
        }

        // Clean up labels for all edges involving this node.
        let keys_to_delete: Vec<(String, String)> = self
            .labels
            .keys()
            .filter(|(from, to)| from == node || to == node)
            .cloned()
            .collect();
        for key in keys_to_delete {
            self.labels.remove(&key);
        }

        self.graph.remove_node(node)
    }

    /// Return true if the node exists in the graph.
    pub fn has_node(&self, node: &str) -> bool {
        self.graph.has_node(node)
    }

    /// Return all nodes in sorted order (deterministic output).
    pub fn nodes(&self) -> Vec<String> {
        self.graph.nodes()
    }

    /// Return the number of nodes in the graph.
    pub fn size(&self) -> usize {
        self.graph.size()
    }

    // ------------------------------------------------------------------
    // Labeled edge operations
    // ------------------------------------------------------------------
    //
    // Each edge in a LabeledDirectedGraph carries one or more labels.
    // `add_edge` requires a label; if you want multiple labels on the
    // same edge, call `add_edge` multiple times with different labels.
    //
    // The underlying Graph tracks whether an edge exists at all (for
    // algorithm purposes). The label map tracks which labels are on
    // each edge.

    /// Add a directed edge from `from` to `to` with the given label.
    ///
    /// If the edge already exists (possibly with different labels), the new
    /// label is added to the existing set — the edge is not duplicated in
    /// the underlying graph.
    ///
    /// Returns [`GraphError::SelfLoop`] if the underlying graph prohibits
    /// self-loops and `from == to`.
    pub fn add_edge(&mut self, from: &str, to: &str, label: &str) -> Result<(), GraphError> {
        // The underlying graph handles self-loop validation.
        self.graph.add_edge(from, to)?;

        let key = (from.to_string(), to.to_string());
        self.labels
            .entry(key)
            .or_insert_with(HashSet::new)
            .insert(label.to_string());

        Ok(())
    }

    /// Remove a specific label from the edge from→to.
    ///
    /// If this was the last label on the edge, the underlying edge is also
    /// removed from the graph. If other labels remain, only the specified
    /// label is removed and the edge persists.
    ///
    /// Returns an error if:
    /// - The edge does not exist ([`GraphError::EdgeNotFound`])
    /// - The label does not exist on the edge ([`GraphError::EdgeNotFound`]
    ///   with a descriptive message)
    pub fn remove_edge(
        &mut self,
        from: &str,
        to: &str,
        label: &str,
    ) -> Result<(), GraphError> {
        let key = (from.to_string(), to.to_string());

        // Check that the edge and label exist.
        let label_set = self.labels.get_mut(&key).ok_or_else(|| {
            GraphError::EdgeNotFound(from.to_string(), to.to_string())
        })?;

        if !label_set.remove(label) {
            return Err(GraphError::EdgeNotFound(
                from.to_string(),
                to.to_string(),
            ));
        }

        // If no labels remain, remove the underlying edge entirely.
        if label_set.is_empty() {
            self.labels.remove(&key);
            self.graph.remove_edge(from, to)?;
        }

        Ok(())
    }

    /// Return true if there's any edge from `from` to `to` (regardless of label).
    pub fn has_edge(&self, from: &str, to: &str) -> bool {
        self.graph.has_edge(from, to)
    }

    /// Return true if there's an edge from `from` to `to` with the specific label.
    pub fn has_edge_with_label(&self, from: &str, to: &str, label: &str) -> bool {
        let key = (from.to_string(), to.to_string());
        if let Some(label_set) = self.labels.get(&key) {
            label_set.contains(label)
        } else {
            false
        }
    }

    /// Return all edges as (from, to, label) triples, sorted deterministically.
    ///
    /// If an edge has multiple labels, it appears once per label.
    ///
    /// Example: if edge A→B has labels "compile" and "test", the output
    /// includes both ("A", "B", "compile") and ("A", "B", "test").
    pub fn edges(&self) -> Vec<(String, String, String)> {
        let mut edges: Vec<(String, String, String)> = Vec::new();
        for ((from, to), label_set) in &self.labels {
            for label in label_set {
                edges.push((from.clone(), to.clone(), label.clone()));
            }
        }
        edges.sort();
        edges
    }

    /// Return the set of labels on the edge from→to.
    ///
    /// Returns an empty set if the edge doesn't exist.
    pub fn labels(&self, from: &str, to: &str) -> HashSet<String> {
        let key = (from.to_string(), to.to_string());
        self.labels
            .get(&key)
            .cloned()
            .unwrap_or_else(HashSet::new)
    }

    // ------------------------------------------------------------------
    // Neighbor queries
    // ------------------------------------------------------------------
    //
    // These methods let you ask "who are my neighbors?" with optional
    // label filtering. The unfiltered versions delegate to the underlying
    // Graph. The label-filtered versions scan the label map.

    /// Return the direct successors of a node (any label).
    pub fn successors(&self, node: &str) -> Result<Vec<String>, GraphError> {
        self.graph.successors(node)
    }

    /// Return successors connected by edges with the given label.
    ///
    /// For example, if A→B has label "compile" and A→C has label "test",
    /// then `successors_with_label("A", "compile")` returns `["B"]`.
    pub fn successors_with_label(
        &self,
        node: &str,
        label: &str,
    ) -> Result<Vec<String>, GraphError> {
        if !self.graph.has_node(node) {
            return Err(GraphError::NodeNotFound(node.to_string()));
        }

        let succs = self.graph.successors(node)?;
        let mut result: Vec<String> = succs
            .into_iter()
            .filter(|succ| {
                let key = (node.to_string(), succ.clone());
                self.labels
                    .get(&key)
                    .map_or(false, |ls| ls.contains(label))
            })
            .collect();
        result.sort();
        Ok(result)
    }

    /// Return the direct predecessors of a node (any label).
    pub fn predecessors(&self, node: &str) -> Result<Vec<String>, GraphError> {
        self.graph.predecessors(node)
    }

    /// Return predecessors connected by edges with the given label.
    pub fn predecessors_with_label(
        &self,
        node: &str,
        label: &str,
    ) -> Result<Vec<String>, GraphError> {
        if !self.graph.has_node(node) {
            return Err(GraphError::NodeNotFound(node.to_string()));
        }

        let preds = self.graph.predecessors(node)?;
        let mut result: Vec<String> = preds
            .into_iter()
            .filter(|pred| {
                let key = (pred.clone(), node.to_string());
                self.labels
                    .get(&key)
                    .map_or(false, |ls| ls.contains(label))
            })
            .collect();
        result.sort();
        Ok(result)
    }

    // ------------------------------------------------------------------
    // Algorithm delegation
    // ------------------------------------------------------------------
    //
    // All graph algorithms delegate to the underlying Graph. Labels don't
    // affect the structural algorithms — topological sort, cycle detection,
    // and transitive closure only care about whether edges exist, not what
    // they're labeled.

    /// Return a topological ordering of all nodes.
    /// Returns [`GraphError::CycleError`] if the graph contains a cycle.
    pub fn topological_sort(&self) -> Result<Vec<String>, GraphError> {
        self.graph.topological_sort()
    }

    /// Return true if the graph contains a cycle.
    pub fn has_cycle(&self) -> bool {
        self.graph.has_cycle()
    }

    /// Return all nodes reachable downstream from `node` by following
    /// forward edges.
    pub fn transitive_closure(&self, node: &str) -> Result<HashSet<String>, GraphError> {
        self.graph.transitive_closure(node)
    }

    /// Return a reference to the underlying unlabeled Graph, giving access
    /// to all base graph methods (independent_groups, affected_nodes, etc.).
    pub fn graph(&self) -> &Graph {
        &self.graph
    }
}

impl Default for LabeledDirectedGraph {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════════
    // Empty graph tests
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_empty_nodes() {
        let lg = LabeledDirectedGraph::new();
        assert!(lg.nodes().is_empty());
    }

    #[test]
    fn test_empty_edges() {
        let lg = LabeledDirectedGraph::new();
        assert!(lg.edges().is_empty());
    }

    #[test]
    fn test_empty_size() {
        let lg = LabeledDirectedGraph::new();
        assert_eq!(lg.size(), 0);
    }

    #[test]
    fn test_empty_topo_sort() {
        let lg = LabeledDirectedGraph::new();
        let result = lg.topological_sort().unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_empty_has_cycle() {
        let lg = LabeledDirectedGraph::new();
        assert!(!lg.has_cycle());
    }

    #[test]
    fn test_default() {
        let lg = LabeledDirectedGraph::default();
        assert_eq!(lg.size(), 0);
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Node operations
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_add_node() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_node("A");
        assert!(lg.has_node("A"));
        assert_eq!(lg.size(), 1);
    }

    #[test]
    fn test_add_node_idempotent() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_node("A");
        lg.add_node("A");
        assert_eq!(lg.size(), 1);
    }

    #[test]
    fn test_remove_node() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_node("A");
        lg.remove_node("A").unwrap();
        assert!(!lg.has_node("A"));
    }

    #[test]
    fn test_remove_node_not_found() {
        let mut lg = LabeledDirectedGraph::new();
        let err = lg.remove_node("X").unwrap_err();
        assert_eq!(err, GraphError::NodeNotFound("X".to_string()));
    }

    #[test]
    fn test_remove_node_cleans_labels() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("A", "B", "compile").unwrap();
        lg.add_edge("B", "C", "test").unwrap();
        lg.remove_node("B").unwrap();
        assert!(!lg.has_edge("A", "B"));
        assert!(!lg.has_edge("B", "C"));
        assert!(lg.labels("A", "B").is_empty());
    }

    #[test]
    fn test_nodes_returns_sorted() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_node("C");
        lg.add_node("A");
        lg.add_node("B");
        assert_eq!(lg.nodes(), vec!["A", "B", "C"]);
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Edge operations (single label)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_add_edge() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("A", "B", "compile").unwrap();
        assert!(lg.has_edge("A", "B"));
        assert!(lg.has_edge_with_label("A", "B", "compile"));
    }

    #[test]
    fn test_add_edge_implicit_nodes() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("X", "Y", "dep").unwrap();
        assert!(lg.has_node("X"));
        assert!(lg.has_node("Y"));
    }

    #[test]
    fn test_add_edge_directed() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("A", "B", "compile").unwrap();
        assert!(!lg.has_edge("B", "A"));
    }

    #[test]
    fn test_self_loop_error() {
        let mut lg = LabeledDirectedGraph::new();
        let err = lg.add_edge("A", "A", "loop").unwrap_err();
        assert_eq!(err, GraphError::SelfLoop("A".to_string()));
    }

    #[test]
    fn test_self_loop_allowed() {
        let mut lg = LabeledDirectedGraph::new_allow_self_loops();
        lg.add_edge("A", "A", "retry").unwrap();
        assert!(lg.has_edge("A", "A"));
        assert!(lg.has_edge_with_label("A", "A", "retry"));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Multiple labels on same edge
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_multiple_labels() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("A", "B", "compile").unwrap();
        lg.add_edge("A", "B", "test").unwrap();
        assert!(lg.has_edge_with_label("A", "B", "compile"));
        assert!(lg.has_edge_with_label("A", "B", "test"));
    }

    #[test]
    fn test_multiple_labels_edges_output() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("A", "B", "compile").unwrap();
        lg.add_edge("A", "B", "test").unwrap();
        let edges = lg.edges();
        assert_eq!(edges.len(), 2);
        assert_eq!(
            edges[0],
            ("A".to_string(), "B".to_string(), "compile".to_string())
        );
        assert_eq!(
            edges[1],
            ("A".to_string(), "B".to_string(), "test".to_string())
        );
    }

    #[test]
    fn test_labels_method() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("A", "B", "compile").unwrap();
        lg.add_edge("A", "B", "test").unwrap();
        lg.add_edge("A", "B", "runtime").unwrap();
        let labels = lg.labels("A", "B");
        assert_eq!(labels.len(), 3);
        assert!(labels.contains("compile"));
        assert!(labels.contains("test"));
        assert!(labels.contains("runtime"));
    }

    #[test]
    fn test_labels_empty_edge() {
        let lg = LabeledDirectedGraph::new();
        let labels = lg.labels("X", "Y");
        assert!(labels.is_empty());
    }

    #[test]
    fn test_duplicate_label_is_noop() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("A", "B", "compile").unwrap();
        lg.add_edge("A", "B", "compile").unwrap(); // duplicate
        assert_eq!(lg.labels("A", "B").len(), 1);
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Remove edge (with labels)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_remove_edge_single_label() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("A", "B", "compile").unwrap();
        lg.remove_edge("A", "B", "compile").unwrap();
        assert!(!lg.has_edge("A", "B"));
        assert!(!lg.has_edge_with_label("A", "B", "compile"));
    }

    #[test]
    fn test_remove_edge_one_of_many() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("A", "B", "compile").unwrap();
        lg.add_edge("A", "B", "test").unwrap();
        lg.remove_edge("A", "B", "compile").unwrap();
        // Edge should still exist because "test" label remains
        assert!(lg.has_edge("A", "B"));
        assert!(!lg.has_edge_with_label("A", "B", "compile"));
        assert!(lg.has_edge_with_label("A", "B", "test"));
    }

    #[test]
    fn test_remove_edge_not_found() {
        let mut lg = LabeledDirectedGraph::new();
        let err = lg.remove_edge("A", "B", "x").unwrap_err();
        assert_eq!(
            err,
            GraphError::EdgeNotFound("A".to_string(), "B".to_string())
        );
    }

    #[test]
    fn test_remove_edge_label_not_found() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("A", "B", "compile").unwrap();
        let err = lg.remove_edge("A", "B", "nonexistent").unwrap_err();
        assert_eq!(
            err,
            GraphError::EdgeNotFound("A".to_string(), "B".to_string())
        );
    }

    #[test]
    fn test_remove_all_labels_removes_edge() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("A", "B", "compile").unwrap();
        lg.add_edge("A", "B", "test").unwrap();
        lg.remove_edge("A", "B", "compile").unwrap();
        lg.remove_edge("A", "B", "test").unwrap();
        assert!(!lg.has_edge("A", "B"));
        // Nodes should still exist
        assert!(lg.has_node("A"));
        assert!(lg.has_node("B"));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // HasEdge / HasEdgeWithLabel
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_has_edge_no_node() {
        let lg = LabeledDirectedGraph::new();
        assert!(!lg.has_edge("X", "Y"));
    }

    #[test]
    fn test_has_edge_with_label_no_node() {
        let lg = LabeledDirectedGraph::new();
        assert!(!lg.has_edge_with_label("X", "Y", "z"));
    }

    #[test]
    fn test_has_edge_with_wrong_label() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("A", "B", "compile").unwrap();
        assert!(!lg.has_edge_with_label("A", "B", "wrong"));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Successors / Predecessors
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_successors() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("A", "B", "compile").unwrap();
        lg.add_edge("A", "C", "test").unwrap();
        let succs = lg.successors("A").unwrap();
        assert_eq!(succs, vec!["B", "C"]);
    }

    #[test]
    fn test_successors_not_found() {
        let lg = LabeledDirectedGraph::new();
        let err = lg.successors("X").unwrap_err();
        assert_eq!(err, GraphError::NodeNotFound("X".to_string()));
    }

    #[test]
    fn test_successors_with_label() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("A", "B", "compile").unwrap();
        lg.add_edge("A", "C", "test").unwrap();
        lg.add_edge("A", "D", "compile").unwrap();
        let succs = lg.successors_with_label("A", "compile").unwrap();
        assert_eq!(succs, vec!["B", "D"]);
    }

    #[test]
    fn test_successors_with_label_not_found() {
        let lg = LabeledDirectedGraph::new();
        let err = lg.successors_with_label("X", "compile").unwrap_err();
        assert_eq!(err, GraphError::NodeNotFound("X".to_string()));
    }

    #[test]
    fn test_successors_with_label_empty() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("A", "B", "compile").unwrap();
        let succs = lg.successors_with_label("A", "nonexistent").unwrap();
        assert!(succs.is_empty());
    }

    #[test]
    fn test_predecessors() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("A", "C", "compile").unwrap();
        lg.add_edge("B", "C", "test").unwrap();
        let preds = lg.predecessors("C").unwrap();
        assert_eq!(preds, vec!["A", "B"]);
    }

    #[test]
    fn test_predecessors_not_found() {
        let lg = LabeledDirectedGraph::new();
        let err = lg.predecessors("X").unwrap_err();
        assert_eq!(err, GraphError::NodeNotFound("X".to_string()));
    }

    #[test]
    fn test_predecessors_with_label() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("A", "C", "compile").unwrap();
        lg.add_edge("B", "C", "test").unwrap();
        lg.add_edge("D", "C", "compile").unwrap();
        let preds = lg.predecessors_with_label("C", "compile").unwrap();
        assert_eq!(preds, vec!["A", "D"]);
    }

    #[test]
    fn test_predecessors_with_label_not_found() {
        let lg = LabeledDirectedGraph::new();
        let err = lg.predecessors_with_label("X", "compile").unwrap_err();
        assert_eq!(err, GraphError::NodeNotFound("X".to_string()));
    }

    #[test]
    fn test_predecessors_with_label_empty() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("A", "B", "compile").unwrap();
        let preds = lg.predecessors_with_label("B", "nonexistent").unwrap();
        assert!(preds.is_empty());
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Algorithm delegation
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_topological_sort() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("A", "B", "compile").unwrap();
        lg.add_edge("B", "C", "compile").unwrap();
        let result = lg.topological_sort().unwrap();
        assert_eq!(result, vec!["A", "B", "C"]);
    }

    #[test]
    fn test_has_cycle_no_cycle() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("A", "B", "dep").unwrap();
        lg.add_edge("B", "C", "dep").unwrap();
        assert!(!lg.has_cycle());
    }

    #[test]
    fn test_has_cycle_with_cycle() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("A", "B", "dep").unwrap();
        lg.add_edge("B", "C", "dep").unwrap();
        lg.add_edge("C", "A", "dep").unwrap();
        assert!(lg.has_cycle());
    }

    #[test]
    fn test_transitive_closure() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("A", "B", "compile").unwrap();
        lg.add_edge("B", "C", "compile").unwrap();
        lg.add_edge("A", "D", "test").unwrap();
        let closure = lg.transitive_closure("A").unwrap();
        assert_eq!(closure.len(), 3);
        assert!(closure.contains("B"));
        assert!(closure.contains("C"));
        assert!(closure.contains("D"));
    }

    #[test]
    fn test_transitive_closure_not_found() {
        let lg = LabeledDirectedGraph::new();
        let err = lg.transitive_closure("X").unwrap_err();
        assert_eq!(err, GraphError::NodeNotFound("X".to_string()));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Graph() accessor
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_graph_accessor() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("A", "B", "compile").unwrap();
        let g = lg.graph();
        assert!(g.has_edge("A", "B"));
    }

    #[test]
    fn test_graph_independent_groups() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("A", "B", "compile").unwrap();
        lg.add_edge("A", "C", "compile").unwrap();
        lg.add_edge("B", "D", "test").unwrap();
        lg.add_edge("C", "D", "test").unwrap();
        let groups = lg.graph().independent_groups().unwrap();
        assert_eq!(groups.len(), 3);
    }

    #[test]
    fn test_graph_affected_nodes() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("A", "B", "compile").unwrap();
        lg.add_edge("B", "C", "compile").unwrap();
        let changed: HashSet<String> = ["A".to_string()].into_iter().collect();
        let affected = lg.graph().affected_nodes(&changed);
        assert_eq!(affected.len(), 3);
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Complex scenarios
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_diamond_graph() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("A", "B", "compile").unwrap();
        lg.add_edge("A", "C", "test").unwrap();
        lg.add_edge("B", "D", "compile").unwrap();
        lg.add_edge("C", "D", "runtime").unwrap();

        let order = lg.topological_sort().unwrap();
        assert_eq!(order.len(), 4);

        let compile_succs = lg.successors_with_label("A", "compile").unwrap();
        assert_eq!(compile_succs, vec!["B"]);

        let test_succs = lg.successors_with_label("A", "test").unwrap();
        assert_eq!(test_succs, vec!["C"]);
    }

    #[test]
    fn test_build_system_scenario() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_node("logic-gates");
        lg.add_edge("logic-gates", "arithmetic", "compile").unwrap();
        lg.add_edge("arithmetic", "cpu-simulator", "compile").unwrap();
        lg.add_edge("logic-gates", "test-harness", "test").unwrap();

        let compile_succs = lg
            .successors_with_label("logic-gates", "compile")
            .unwrap();
        assert_eq!(compile_succs, vec!["arithmetic"]);

        let test_succs = lg
            .successors_with_label("logic-gates", "test")
            .unwrap();
        assert_eq!(test_succs, vec!["test-harness"]);

        let all_succs = lg.successors("logic-gates").unwrap();
        assert_eq!(all_succs.len(), 2);
    }

    #[test]
    fn test_edges_returns_sorted() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("C", "D", "z").unwrap();
        lg.add_edge("A", "B", "a").unwrap();
        lg.add_edge("A", "B", "b").unwrap();
        let edges = lg.edges();
        assert_eq!(edges.len(), 3);
        assert_eq!(
            edges[0],
            ("A".to_string(), "B".to_string(), "a".to_string())
        );
        assert_eq!(
            edges[1],
            ("A".to_string(), "B".to_string(), "b".to_string())
        );
        assert_eq!(
            edges[2],
            ("C".to_string(), "D".to_string(), "z".to_string())
        );
    }

    #[test]
    fn test_remove_node_with_multiple_labels() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("A", "B", "compile").unwrap();
        lg.add_edge("A", "B", "test").unwrap();
        lg.add_edge("B", "C", "runtime").unwrap();
        lg.remove_node("B").unwrap();
        assert!(!lg.has_edge("A", "B"));
        assert!(!lg.has_edge("B", "C"));
        assert!(lg.labels("A", "B").is_empty());
    }

    #[test]
    fn test_self_loop_with_multiple_labels() {
        let mut lg = LabeledDirectedGraph::new_allow_self_loops();
        lg.add_edge("A", "A", "retry").unwrap();
        lg.add_edge("A", "A", "refresh").unwrap();
        let labels = lg.labels("A", "A");
        assert_eq!(labels.len(), 2);
        assert!(labels.contains("retry"));
        assert!(labels.contains("refresh"));
    }

    #[test]
    fn test_remove_self_loop() {
        let mut lg = LabeledDirectedGraph::new_allow_self_loops();
        lg.add_edge("A", "A", "retry").unwrap();
        lg.add_edge("A", "A", "refresh").unwrap();
        lg.remove_edge("A", "A", "retry").unwrap();
        assert!(lg.has_edge("A", "A"), "self-loop should still exist");
        lg.remove_edge("A", "A", "refresh").unwrap();
        assert!(!lg.has_edge("A", "A"), "self-loop should be fully removed");
    }

    #[test]
    fn test_isolated_node_topo_sort() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_node("isolated");
        lg.add_edge("A", "B", "dep").unwrap();
        let order = lg.topological_sort().unwrap();
        assert_eq!(order.len(), 3);
    }

    #[test]
    fn test_edges_after_removal() {
        let mut lg = LabeledDirectedGraph::new();
        lg.add_edge("A", "B", "compile").unwrap();
        lg.add_edge("A", "B", "test").unwrap();
        lg.remove_edge("A", "B", "compile").unwrap();
        let edges = lg.edges();
        assert_eq!(edges.len(), 1);
        assert_eq!(
            edges[0],
            ("A".to_string(), "B".to_string(), "test".to_string())
        );
    }
}
