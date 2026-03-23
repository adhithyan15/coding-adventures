// Directed graph data structure for dependency resolution.
//
// # What is a directed graph?
//
// A directed graph (or "digraph") is a set of nodes connected by edges,
// where each edge has a direction — it goes FROM one node TO another.
// Think of it like a one-way street map: you can travel from A to B,
// but that doesn't mean you can travel from B to A.
//
// In this build system, nodes are packages and edges are dependencies:
// if package B depends on package A, there's an edge from A to B
// (A must be built before B).
//
// # Why embed the graph here?
//
// The Go implementation uses a separate `directed-graph` package via
// Go module replace directives. In Rust, we embed the graph directly
// since there's no equivalent of `replace` for local path dependencies
// outside the workspace. This keeps the build tool self-contained.
//
// # Key algorithms
//
// - **Topological sort** (Kahn's algorithm): order nodes so every
//   dependency comes before its dependents.
// - **Independent groups**: partition nodes into "levels" where
//   everything at the same level can run in parallel.
// - **Affected nodes**: given changed nodes, find everything that
//   transitively depends on them.

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::fmt;

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Errors that can occur during graph operations.
#[derive(Debug)]
pub enum GraphError {
    /// The graph contains a cycle, making topological sorting impossible.
    /// A cycle means A depends on B depends on C depends on A — an
    /// unresolvable circular dependency.
    CycleDetected,
    /// A referenced node does not exist in the graph.
    NodeNotFound(String),
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphError::CycleDetected => write!(f, "cycle detected in dependency graph"),
            GraphError::NodeNotFound(node) => write!(f, "node not found: {}", node),
        }
    }
}

impl std::error::Error for GraphError {}

// ---------------------------------------------------------------------------
// Graph
// ---------------------------------------------------------------------------

/// A directed graph with string-typed nodes.
///
/// It stores both forward edges (node -> its successors) and reverse
/// edges (node -> its predecessors) for efficient lookups in both
/// directions. This doubles memory usage but makes transitive-dependent
/// queries O(V+E) instead of requiring a full graph reversal.
pub struct Graph {
    /// Forward adjacency: node -> set of nodes it points TO.
    forward: HashMap<String, HashSet<String>>,
    /// Reverse adjacency: node -> set of nodes that point TO it.
    reverse: HashMap<String, HashSet<String>>,
}

impl Graph {
    /// Creates an empty directed graph.
    pub fn new() -> Self {
        Graph {
            forward: HashMap::new(),
            reverse: HashMap::new(),
        }
    }

    /// Adds a node to the graph. No-op if the node already exists.
    pub fn add_node(&mut self, node: &str) {
        self.forward
            .entry(node.to_string())
            .or_insert_with(HashSet::new);
        self.reverse
            .entry(node.to_string())
            .or_insert_with(HashSet::new);
    }

    /// Adds a directed edge from `from` to `to`.
    /// Both nodes are implicitly added if they don't exist.
    /// Panics on self-loops (from == to).
    pub fn add_edge(&mut self, from: &str, to: &str) {
        assert!(
            from != to,
            "self-loop not allowed: {:?}",
            from
        );
        self.add_node(from);
        self.add_node(to);
        self.forward
            .get_mut(from)
            .unwrap()
            .insert(to.to_string());
        self.reverse
            .get_mut(to)
            .unwrap()
            .insert(from.to_string());
    }

    /// Returns true if the node exists in the graph.
    pub fn has_node(&self, node: &str) -> bool {
        self.forward.contains_key(node)
    }

    /// Returns the direct predecessors of a node (nodes with edges TO it).
    /// These are the node's direct dependencies in our build-system convention.
    pub fn predecessors(&self, node: &str) -> Result<Vec<String>, GraphError> {
        match self.reverse.get(node) {
            Some(preds) => {
                let mut result: Vec<String> = preds.iter().cloned().collect();
                result.sort();
                Ok(result)
            }
            None => Err(GraphError::NodeNotFound(node.to_string())),
        }
    }

    /// Returns the direct successors of a node (nodes it has edges TO).
    /// These are the node's direct dependents in our build-system convention.
    #[allow(dead_code)]
    pub fn successors(&self, node: &str) -> Result<Vec<String>, GraphError> {
        match self.forward.get(node) {
            Some(succs) => {
                let mut result: Vec<String> = succs.iter().cloned().collect();
                result.sort();
                Ok(result)
            }
            None => Err(GraphError::NodeNotFound(node.to_string())),
        }
    }

    /// Partitions nodes into levels by topological depth using Kahn's algorithm.
    ///
    /// Kahn's algorithm works by repeatedly removing nodes with no incoming edges:
    ///  1. Find all nodes with in-degree 0 (no predecessors)
    ///  2. Remove them from the graph (conceptually), add to current level
    ///  3. Their successors may now have in-degree 0 — form the next level
    ///  4. If all nodes are processed, we have valid levels
    ///  5. If some remain, there's a cycle
    ///
    /// Nodes at the same level have no dependency on each other and can
    /// run in parallel. This is the key method for parallel execution.
    ///
    /// Example for a diamond graph (A->B, A->C, B->D, C->D):
    ///   Level 0: [A]      — no dependencies
    ///   Level 1: [B, C]   — depend only on A, can run in parallel
    ///   Level 2: [D]      — depends on B and C
    pub fn independent_groups(&self) -> Result<Vec<Vec<String>>, GraphError> {
        // Compute in-degrees for each node.
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        for (node, preds) in &self.reverse {
            in_degree.insert(node.clone(), preds.len());
        }

        // Collect nodes with in-degree 0 into the first queue.
        // We use BTreeSet for deterministic (sorted) ordering.
        let mut queue: BTreeSet<String> = BTreeSet::new();
        for (node, &deg) in &in_degree {
            if deg == 0 {
                queue.insert(node.clone());
            }
        }

        let mut levels: Vec<Vec<String>> = Vec::new();
        let mut processed = 0;

        while !queue.is_empty() {
            // The current queue IS one level — all these nodes can run in parallel.
            let level: Vec<String> = queue.iter().cloned().collect();
            processed += level.len();
            levels.push(level.clone());

            let mut next_queue: BTreeSet<String> = BTreeSet::new();
            for node in &level {
                if let Some(succs) = self.forward.get(node) {
                    for succ in succs {
                        if let Some(deg) = in_degree.get_mut(succ) {
                            *deg -= 1;
                            if *deg == 0 {
                                next_queue.insert(succ.clone());
                            }
                        }
                    }
                }
            }
            queue = next_queue;
        }

        if processed != self.forward.len() {
            return Err(GraphError::CycleDetected);
        }

        Ok(levels)
    }

    /// Returns all nodes transitively reachable from `node` by following
    /// forward edges. In our build-system convention, these are all packages
    /// that depend (directly or indirectly) on the given package.
    pub fn transitive_dependents(&self, node: &str) -> Result<HashSet<String>, GraphError> {
        if !self.has_node(node) {
            return Err(GraphError::NodeNotFound(node.to_string()));
        }

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(node.to_string());

        while let Some(curr) = queue.pop_front() {
            if let Some(succs) = self.forward.get(&curr) {
                for succ in succs {
                    if visited.insert(succ.clone()) {
                        queue.push_back(succ.clone());
                    }
                }
            }
        }

        Ok(visited)
    }

    /// Returns the set of nodes affected by changes to the given set.
    /// "Affected" means: the changed nodes themselves, plus everything
    /// that transitively depends on any of them.
    ///
    /// This is the primary method for the build tool's git-diff mode:
    /// if you change logic-gates, the affected set includes logic-gates
    /// plus arithmetic, cpu-simulator, arm-simulator, etc.
    pub fn affected_nodes(&self, changed: &HashSet<String>) -> HashSet<String> {
        let mut affected = HashSet::new();
        for node in changed {
            if !self.has_node(node) {
                continue;
            }
            affected.insert(node.clone());
            if let Ok(deps) = self.transitive_dependents(node) {
                for dep in deps {
                    affected.insert(dep);
                }
            }
        }
        affected
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_node_and_has_node() {
        let mut g = Graph::new();
        assert!(!g.has_node("A"));
        g.add_node("A");
        assert!(g.has_node("A"));
    }

    #[test]
    fn test_add_edge_creates_nodes() {
        let mut g = Graph::new();
        g.add_edge("A", "B");
        assert!(g.has_node("A"));
        assert!(g.has_node("B"));
    }

    #[test]
    #[should_panic(expected = "self-loop not allowed")]
    fn test_self_loop_panics() {
        let mut g = Graph::new();
        g.add_edge("A", "A");
    }

    #[test]
    fn test_predecessors_and_successors() {
        let mut g = Graph::new();
        g.add_edge("A", "B");
        g.add_edge("A", "C");
        g.add_edge("B", "D");
        g.add_edge("C", "D");

        // A has no predecessors (it's the root).
        assert_eq!(g.predecessors("A").unwrap(), Vec::<String>::new());
        // D has two predecessors: B and C.
        assert_eq!(g.predecessors("D").unwrap(), vec!["B", "C"]);
        // A has two successors: B and C.
        assert_eq!(g.successors("A").unwrap(), vec!["B", "C"]);
    }

    #[test]
    fn test_independent_groups_diamond() {
        // Diamond: A->B, A->C, B->D, C->D
        let mut g = Graph::new();
        g.add_edge("A", "B");
        g.add_edge("A", "C");
        g.add_edge("B", "D");
        g.add_edge("C", "D");

        let groups = g.independent_groups().unwrap();
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0], vec!["A"]);
        assert_eq!(groups[1], vec!["B", "C"]);
        assert_eq!(groups[2], vec!["D"]);
    }

    #[test]
    fn test_independent_groups_no_edges() {
        let mut g = Graph::new();
        g.add_node("A");
        g.add_node("B");
        g.add_node("C");

        let groups = g.independent_groups().unwrap();
        // All nodes should be in one level since they're independent.
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0], vec!["A", "B", "C"]);
    }

    #[test]
    fn test_cycle_detection() {
        let mut g = Graph::new();
        g.add_edge("A", "B");
        g.add_edge("B", "C");
        g.add_edge("C", "A");

        let result = g.independent_groups();
        assert!(result.is_err());
    }

    #[test]
    fn test_transitive_dependents() {
        let mut g = Graph::new();
        g.add_edge("A", "B");
        g.add_edge("B", "C");
        g.add_edge("A", "D");

        let deps = g.transitive_dependents("A").unwrap();
        assert!(deps.contains("B"));
        assert!(deps.contains("C"));
        assert!(deps.contains("D"));
        assert!(!deps.contains("A")); // Does not include itself.
    }

    #[test]
    fn test_affected_nodes() {
        let mut g = Graph::new();
        g.add_edge("A", "B");
        g.add_edge("B", "C");
        g.add_node("D"); // Independent node.

        let mut changed = HashSet::new();
        changed.insert("A".to_string());

        let affected = g.affected_nodes(&changed);
        assert!(affected.contains("A")); // Itself.
        assert!(affected.contains("B")); // Direct dependent.
        assert!(affected.contains("C")); // Transitive dependent.
        assert!(!affected.contains("D")); // Unrelated.
    }

    #[test]
    fn test_affected_nodes_unknown_node() {
        let g = Graph::new();
        let mut changed = HashSet::new();
        changed.insert("nonexistent".to_string());
        let affected = g.affected_nodes(&changed);
        assert!(affected.is_empty());
    }
}
