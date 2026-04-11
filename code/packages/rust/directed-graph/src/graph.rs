// graph.rs -- Directed Graph with Shared Traversal Algorithms
// ====================================================
//
// This module contains the directed graph implementation: the data structure,
// mutation methods, query methods, and directed-specific graph algorithms.
// Shared traversal helpers like BFS live in the `graph` package and are consumed
// through a small trait seam so we can reuse them without duplicating traversal
// logic here.
//
// Internal Storage
// ----------------
//
// We maintain **two** adjacency maps:
//
// - `forward.get(u)`  = set of nodes that `u` points TO   (successors / children)
// - `reverse.get(v)`  = set of nodes that point TO `v`     (predecessors / parents)
//
// Every node that exists in the graph has an entry in both maps, even if its
// adjacency set is empty. This invariant lets us use `self.forward.contains_key(node)`
// as the canonical "does this node exist?" check, and it means we never need
// to special-case missing keys.
//
// Why two maps? Because many of our algorithms need to walk edges in *both*
// directions efficiently:
//
// - `topological_sort` needs to find nodes with zero in-degree, which means
//   checking `self.reverse[node].len() == 0` -- O(1) with the reverse map.
// - `transitive_closure` walks *forward* from a node using BFS.
// - `remove_node` needs to clean up both incoming and outgoing edges, which
//   is O(degree) with both maps but would be O(E) with only one.
//
// The trade-off is that every `add_edge` and `remove_edge` must update both
// maps, but that's O(1) per operation, so it's a good deal.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt;

use graph_base::{bfs, TraversalGraph};

// ---------------------------------------------------------------------------
// Custom error types
// ---------------------------------------------------------------------------
// Each error carries enough context for the caller to produce a useful
// error message. We use a single enum with variants rather than separate
// types, because Rust's `Result<T, E>` works best with a unified error type.
//
// We implement `Display` and `std::error::Error` manually rather than
// pulling in the `thiserror` crate. The crate now depends on the shared
// `graph` package for reusable traversal helpers.

/// All errors that can occur when operating on a [`Graph`].
///
/// The three variants correspond to the three kinds of things that can go wrong:
///
/// - **CycleError**: a topological sort was requested on a graph that contains
///   a cycle. A cycle means there's a circular dependency: A depends on B depends
///   on C depends on A. This makes it impossible to determine a build order.
///
/// - **NodeNotFound**: an operation referenced a node that doesn't exist in
///   the graph (e.g., `remove_node("X")` when X was never added).
///
/// - **EdgeNotFound**: `remove_edge(u, v)` was called but the edge u -> v
///   doesn't exist.
///
/// - **SelfLoop**: `add_edge(u, u)` was called — self-loops are not allowed
///   in a DAG-oriented graph because they trivially create a cycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphError {
    /// The graph contains a cycle, so topological ordering is impossible.
    CycleError,
    /// The specified node does not exist in the graph.
    NodeNotFound(String),
    /// The specified edge does not exist in the graph.
    EdgeNotFound(String, String),
    /// Self-loops (an edge from a node to itself) are not allowed.
    SelfLoop(String),
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphError::CycleError => write!(f, "graph contains a cycle"),
            GraphError::NodeNotFound(node) => write!(f, "node not found: {:?}", node),
            GraphError::EdgeNotFound(from, to) => {
                write!(f, "edge not found: {:?} -> {:?}", from, to)
            }
            GraphError::SelfLoop(node) => write!(f, "self-loop not allowed: {:?}", node),
        }
    }
}

impl std::error::Error for GraphError {}

// ---------------------------------------------------------------------------
// The Graph struct
// ---------------------------------------------------------------------------

/// A directed graph backed by forward and reverse adjacency maps.
///
/// The graph stores string-typed nodes. Edges are directed: `add_edge("A", "B")`
/// means A points to B, so B is a *successor* of A and A is a *predecessor* of B.
///
/// # Self-loops
///
/// By default, self-loops (edges from a node to itself, like A→A) are
/// prohibited because they create trivial cycles, which makes topological
/// sorting impossible. However, some use cases genuinely need self-loops —
/// for example, modeling state machines where a state can transition to
/// itself, or representing "retry" semantics in a workflow graph.
///
/// Use [`Graph::new_allow_self_loops()`] to create a graph that permits
/// self-loops. The `allow_self_loops` flag is checked only in `add_edge`;
/// all other methods work correctly regardless of the flag's value.
///
/// Duplicate edges and nodes are silently ignored (idempotent adds).
///
/// # Example
///
/// ```
/// use directed_graph::Graph;
///
/// let mut g = Graph::new();
/// g.add_edge("compile", "link").unwrap();
/// g.add_edge("link", "package").unwrap();
/// assert_eq!(g.topological_sort().unwrap(), vec!["compile", "link", "package"]);
/// ```
pub struct Graph {
    /// forward adjacency: node -> set of successors (nodes it points TO)
    forward: HashMap<String, HashSet<String>>,
    /// reverse adjacency: node -> set of predecessors (nodes that point TO it)
    reverse: HashMap<String, HashSet<String>>,
    /// whether self-loops (A→A edges) are permitted
    allow_self_loops: bool,
}

impl Graph {
    // ------------------------------------------------------------------
    // Constructors
    // ------------------------------------------------------------------

    /// Create an empty directed graph that prohibits self-loops.
    ///
    /// This is the default constructor. If you try to add an edge from a
    /// node to itself (e.g., `g.add_edge("A", "A")`), it will return
    /// [`GraphError::SelfLoop`].
    ///
    /// Both adjacency maps start empty. Nodes are added either explicitly
    /// with [`add_node`] or implicitly by [`add_edge`].
    pub fn new() -> Self {
        Graph {
            forward: HashMap::new(),
            reverse: HashMap::new(),
            allow_self_loops: false,
        }
    }

    /// Create an empty directed graph that permits self-loops.
    ///
    /// A self-loop is an edge from a node to itself, like A→A. This is
    /// useful for modeling state machines, retry loops, or any domain
    /// where a node can reference itself.
    ///
    /// Note: a graph with self-loops will have cycles (a self-loop IS a
    /// cycle of length 1), so `topological_sort()` will return
    /// [`GraphError::CycleError`] and `has_cycle()` will return true.
    pub fn new_allow_self_loops() -> Self {
        Graph {
            forward: HashMap::new(),
            reverse: HashMap::new(),
            allow_self_loops: true,
        }
    }

    /// Returns whether this graph permits self-loops.
    pub fn allows_self_loops(&self) -> bool {
        self.allow_self_loops
    }

    // ------------------------------------------------------------------
    // Node operations
    // ------------------------------------------------------------------

    /// Add a node to the graph. No-op if the node already exists.
    ///
    /// This is called implicitly by [`add_edge`], so you only need to call
    /// it directly for isolated nodes (nodes with no edges).
    pub fn add_node(&mut self, node: &str) {
        if !self.forward.contains_key(node) {
            self.forward.insert(node.to_string(), HashSet::new());
            self.reverse.insert(node.to_string(), HashSet::new());
        }
    }

    /// Remove a node and all its incoming/outgoing edges.
    ///
    /// Returns [`GraphError::NodeNotFound`] if the node doesn't exist.
    ///
    /// This is O(in-degree + out-degree) because we need to update the
    /// adjacency sets of all neighbors.
    pub fn remove_node(&mut self, node: &str) -> Result<(), GraphError> {
        if !self.has_node(node) {
            return Err(GraphError::NodeNotFound(node.to_string()));
        }

        // Clean up outgoing edges: for each successor, remove `node` from
        // that successor's reverse (predecessor) set.
        if let Some(successors) = self.forward.get(node).cloned() {
            for succ in &successors {
                if let Some(preds) = self.reverse.get_mut(succ) {
                    preds.remove(node);
                }
            }
        }

        // Clean up incoming edges: for each predecessor, remove `node` from
        // that predecessor's forward (successor) set.
        if let Some(predecessors) = self.reverse.get(node).cloned() {
            for pred in &predecessors {
                if let Some(succs) = self.forward.get_mut(pred) {
                    succs.remove(node);
                }
            }
        }

        // Finally, remove the node itself from both maps.
        self.forward.remove(node);
        self.reverse.remove(node);

        Ok(())
    }

    /// Return true if the node exists in the graph.
    pub fn has_node(&self, node: &str) -> bool {
        self.forward.contains_key(node)
    }

    /// Return all nodes in sorted order (deterministic output).
    ///
    /// We sort the nodes so that tests and other consumers get consistent
    /// results regardless of HashMap iteration order.
    pub fn nodes(&self) -> Vec<String> {
        let mut nodes: Vec<String> = self.forward.keys().cloned().collect();
        nodes.sort();
        nodes
    }

    /// Return the number of nodes in the graph.
    pub fn size(&self) -> usize {
        self.forward.len()
    }

    // ------------------------------------------------------------------
    // Edge operations
    // ------------------------------------------------------------------

    /// Add a directed edge from `from` to `to`.
    ///
    /// Both nodes are implicitly added if they don't exist yet. This means
    /// you can build a graph entirely with `add_edge` calls — no need
    /// to call `add_node` first.
    ///
    /// Self-loop behavior depends on how the graph was created:
    ///
    /// - `Graph::new()` → self-loops are **prohibited** (returns
    ///   [`GraphError::SelfLoop`])
    /// - `Graph::new_allow_self_loops()` → self-loops are **allowed**
    ///
    /// When self-loops are allowed, `add_edge("A", "A")` inserts A into
    /// both the forward and reverse adjacency sets for A. This means:
    /// - `has_edge("A", "A")` returns true
    /// - `successors("A")` includes "A"
    /// - `predecessors("A")` includes "A"
    /// - `has_cycle()` returns true (a self-loop is a cycle of length 1)
    ///
    /// Duplicate edges are silently ignored (HashSets handle deduplication).
    pub fn add_edge(&mut self, from: &str, to: &str) -> Result<(), GraphError> {
        if from == to && !self.allow_self_loops {
            return Err(GraphError::SelfLoop(from.to_string()));
        }

        // Ensure both nodes exist (idempotent).
        self.add_node(from);
        self.add_node(to);

        // Add the edge to both adjacency maps.
        self.forward.get_mut(from).unwrap().insert(to.to_string());
        self.reverse.get_mut(to).unwrap().insert(from.to_string());

        Ok(())
    }

    /// Remove the directed edge from `from` to `to`.
    ///
    /// Returns [`GraphError::EdgeNotFound`] if the edge doesn't exist
    /// (including if either node doesn't exist).
    pub fn remove_edge(&mut self, from: &str, to: &str) -> Result<(), GraphError> {
        if !self.has_edge(from, to) {
            return Err(GraphError::EdgeNotFound(
                from.to_string(),
                to.to_string(),
            ));
        }

        self.forward.get_mut(from).unwrap().remove(to);
        self.reverse.get_mut(to).unwrap().remove(from);

        Ok(())
    }

    /// Return true if the directed edge from `from` to `to` exists.
    pub fn has_edge(&self, from: &str, to: &str) -> bool {
        if let Some(succs) = self.forward.get(from) {
            succs.contains(to)
        } else {
            false
        }
    }

    /// Return all edges as (from, to) pairs, sorted deterministically.
    ///
    /// Sorting first by `from`, then by `to` ensures consistent output
    /// regardless of HashMap iteration order. This is important for tests.
    pub fn edges(&self) -> Vec<(String, String)> {
        let mut edges: Vec<(String, String)> = Vec::new();
        for (from, succs) in &self.forward {
            for to in succs {
                edges.push((from.clone(), to.clone()));
            }
        }
        edges.sort();
        edges
    }

    // ------------------------------------------------------------------
    // Neighbor queries
    // ------------------------------------------------------------------

    /// Return the direct predecessors (parents) of a node.
    ///
    /// These are the nodes that have an edge pointing TO this node.
    /// Returns [`GraphError::NodeNotFound`] if the node doesn't exist.
    ///
    /// Results are sorted for deterministic output.
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

    /// Return the direct successors (children) of a node.
    ///
    /// These are the nodes that this node points TO.
    /// Returns [`GraphError::NodeNotFound`] if the node doesn't exist.
    ///
    /// Results are sorted for deterministic output.
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

    // ==================================================================
    // ALGORITHMS
    // ==================================================================
    // All algorithms are methods on the graph itself. This keeps the API
    // simple: you just call g.topological_sort() instead of importing a
    // separate module.

    // ------------------------------------------------------------------
    // Topological Sort (Kahn's Algorithm)
    // ------------------------------------------------------------------
    //
    // Kahn's algorithm works by repeatedly removing nodes with zero
    // in-degree from the graph. The order in which we remove them is a
    // valid topological ordering.
    //
    // Why Kahn's instead of DFS-based? Two reasons:
    // 1. It naturally detects cycles (if we can't remove all nodes, there's
    //    a cycle).
    // 2. It's easier to modify for independent_groups (see below).
    //
    // Time complexity: O(V + E) where V = nodes, E = edges.

    /// Return a topological ordering of all nodes.
    ///
    /// A topological ordering is a linear sequence where for every edge
    /// u -> v, u appears before v. This only exists for DAGs (directed
    /// acyclic graphs).
    ///
    /// Returns [`GraphError::CycleError`] if the graph contains a cycle.
    ///
    /// For an empty graph, returns an empty list.
    ///
    /// # How Kahn's algorithm works
    ///
    /// 1. Compute the in-degree (number of incoming edges) for every node.
    /// 2. Find all nodes with in-degree 0 — they have no dependencies.
    /// 3. Remove them from the graph (conceptually), add to result.
    /// 4. Their successors may now have in-degree 0 — repeat.
    /// 5. If all nodes are removed, we have a valid ordering.
    /// 6. If some nodes remain, there's a cycle.
    pub fn topological_sort(&self) -> Result<Vec<String>, GraphError> {
        // We work on copies of the in-degree counts so we don't mutate the
        // actual graph. This is a "virtual" removal — we just decrement
        // counters instead of actually removing nodes.

        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        for (node, preds) in &self.reverse {
            in_degree.insert(node.as_str(), preds.len());
        }

        // Start with all nodes that have zero in-degree (no dependencies).
        // We use a BTreeSet to maintain sorted order for deterministic output.
        let mut queue: BTreeSet<&str> = BTreeSet::new();
        for (&node, &deg) in &in_degree {
            if deg == 0 {
                queue.insert(node);
            }
        }

        let mut result: Vec<String> = Vec::new();

        while let Some(node) = queue.iter().next().copied() {
            queue.remove(node);
            result.push(node.to_string());

            // "Remove" this node by decrementing the in-degree of all its
            // successors. If any successor's in-degree drops to zero, it's
            // ready to be processed.
            if let Some(succs) = self.forward.get(node) {
                for succ in succs {
                    if let Some(deg) = in_degree.get_mut(succ.as_str()) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.insert(succ.as_str());
                        }
                    }
                }
            }
        }

        // If we couldn't process all nodes, there's a cycle.
        if result.len() != self.forward.len() {
            return Err(GraphError::CycleError);
        }

        Ok(result)
    }

    // ------------------------------------------------------------------
    // Cycle Detection (DFS Three-Color Algorithm)
    // ------------------------------------------------------------------
    //
    // The three-color algorithm uses:
    // - WHITE (0): not yet visited
    // - GRAY  (1): currently being explored (on the recursion stack)
    // - BLACK (2): fully explored
    //
    // If we encounter a GRAY node during DFS, we've found a back edge,
    // which means there's a cycle.
    //
    // Why a separate method when topological_sort also detects cycles?
    // Because has_cycle is a simple boolean check — it doesn't need to
    // compute the full ordering, so it can short-circuit as soon as it
    // finds one cycle.

    /// Return true if the graph contains at least one cycle.
    ///
    /// Uses DFS with three-color marking. This is O(V + E).
    ///
    /// An empty graph has no cycles. A DAG has no cycles.
    /// A graph with A -> B -> C -> A has a cycle.
    pub fn has_cycle(&self) -> bool {
        const WHITE: u8 = 0;
        const GRAY: u8 = 1;
        const BLACK: u8 = 2;

        let mut color: HashMap<&str, u8> = HashMap::new();
        for node in self.forward.keys() {
            color.insert(node.as_str(), WHITE);
        }

        // Recursive DFS function. Returns true if a cycle is reachable
        // from the given node.
        fn dfs<'a>(
            node: &'a str,
            forward: &'a HashMap<String, HashSet<String>>,
            color: &mut HashMap<&'a str, u8>,
        ) -> bool {
            color.insert(node, GRAY);

            if let Some(succs) = forward.get(node) {
                for succ in succs {
                    match color.get(succ.as_str()) {
                        Some(&GRAY) => return true, // Back edge — cycle!
                        Some(&WHITE) => {
                            if dfs(succ.as_str(), forward, color) {
                                return true;
                            }
                        }
                        _ => {} // BLACK — already fully processed, skip
                    }
                }
            }

            color.insert(node, BLACK);
            false
        }

        // We need to start DFS from every unvisited node because the graph
        // might not be connected (it could have multiple disconnected components).
        let nodes = self.nodes();
        for node in &nodes {
            if color.get(node.as_str()) == Some(&WHITE) {
                if dfs(node.as_str(), &self.forward, &mut color) {
                    return true;
                }
            }
        }

        false
    }

    // ------------------------------------------------------------------
    // Transitive Closure
    // ------------------------------------------------------------------
    //
    // The transitive closure of a node is the set of all nodes reachable
    // from it by following edges forward. We use BFS because it's simple
    // and doesn't risk stack overflow on deep graphs.
    //
    // In graph theory, this operation answers: "starting from node X,
    // what can I reach by following directed edges?"

    /// Return all nodes reachable downstream from `node` by following
    /// forward edges.
    ///
    /// The starting node is NOT included in the result (only the nodes
    /// it can reach). Uses BFS for simplicity and stack safety.
    ///
    /// Returns [`GraphError::NodeNotFound`] if the node doesn't exist.
    pub fn transitive_closure(&self, node: &str) -> Result<HashSet<String>, GraphError> {
        if !self.has_node(node) {
            return Err(GraphError::NodeNotFound(node.to_string()));
        }

        let mut reachable: HashSet<String> = HashSet::new();
        if let Some(succs) = self.forward.get(node) {
            for succ in succs {
                for reachable_node in bfs(self, succ)? {
                    reachable.insert(reachable_node);
                }
            }
        }
        Ok(reachable)
    }

    // ------------------------------------------------------------------
    // Affected Nodes
    // ------------------------------------------------------------------
    //
    // Given a set of "changed" nodes, compute everything that is affected:
    // the changed nodes themselves plus all their transitive closure
    // (everything reachable downstream via forward edges).
    //
    // Edge convention: edges go FROM dependency TO dependent.
    // So logic-gates -> arithmetic means "arithmetic depends on logic-gates".
    //
    // If "logic-gates" changes, its affected set includes logic-gates
    // + arithmetic + cpu-simulator + ... — everything downstream.
    //
    // This is the key method for the build tool's change detection.

    /// Return the changed nodes plus all nodes reachable from them
    /// via forward edges.
    ///
    /// For each node in `changed`, we find everything downstream
    /// (directly or transitively) and include it in the result. The changed
    /// nodes themselves are always included.
    ///
    /// Nodes in `changed` that don't exist in the graph are silently
    /// ignored (they might have been removed).
    pub fn affected_nodes(&self, changed: &HashSet<String>) -> HashSet<String> {
        let mut affected: HashSet<String> = HashSet::new();

        for node in changed {
            if !self.has_node(node) {
                continue;
            }
            affected.insert(node.clone());
            if let Ok(deps) = self.transitive_closure(node) {
                for dep in deps {
                    affected.insert(dep);
                }
            }
        }

        affected
    }

    // ------------------------------------------------------------------
    // Independent Groups (Parallel Execution Levels)
    // ------------------------------------------------------------------
    //
    // This is a modified version of Kahn's algorithm. Instead of pulling
    // nodes off the queue one at a time, we pull ALL zero-in-degree nodes
    // at once — they form one "level" of independent tasks that can run
    // in parallel.
    //
    // For a linear chain A -> B -> C, we get [[A], [B], [C]] (fully serial).
    // For a diamond A -> B, A -> C, B -> D, C -> D, we get
    // [[A], [B, C], [D]] — B and C can run in parallel.
    //
    // This is the key method for the build system's parallel execution.
    // The build tool uses these levels to decide which packages to build
    // simultaneously.

    /// Partition nodes into levels by topological depth.
    ///
    /// Each level contains nodes that have no dependencies on each other
    /// and whose dependencies have all been satisfied by earlier levels.
    /// Nodes within a level can be executed in parallel.
    ///
    /// Returns [`GraphError::CycleError`] if the graph contains a cycle.
    ///
    /// Returns an empty list for an empty graph.
    ///
    /// # Example
    ///
    /// For a diamond graph (A->B, A->C, B->D, C->D):
    ///
    /// ```text
    /// Level 0: [A]      — no dependencies
    /// Level 1: [B, C]   — depend only on A, can run in parallel
    /// Level 2: [D]      — depends on B and C
    /// ```
    pub fn independent_groups(&self) -> Result<Vec<Vec<String>>, GraphError> {
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        for (node, preds) in &self.reverse {
            in_degree.insert(node.as_str(), preds.len());
        }

        // Collect the initial set of zero-in-degree nodes.
        let mut current_level: Vec<&str> = Vec::new();
        for (&node, &deg) in &in_degree {
            if deg == 0 {
                current_level.push(node);
            }
        }
        current_level.sort();

        let mut groups: Vec<Vec<String>> = Vec::new();
        let mut processed = 0;

        while !current_level.is_empty() {
            // Record this level — all these nodes can run in parallel.
            let level: Vec<String> = current_level.iter().map(|s| s.to_string()).collect();
            processed += level.len();
            groups.push(level);

            // Find the next level: for each node in the current level,
            // decrement the in-degree of its successors. Any successor
            // whose in-degree drops to zero joins the next level.
            let mut next_level: BTreeSet<&str> = BTreeSet::new();
            for &node in &current_level {
                if let Some(succs) = self.forward.get(node) {
                    for succ in succs {
                        if let Some(deg) = in_degree.get_mut(succ.as_str()) {
                            *deg -= 1;
                            if *deg == 0 {
                                next_level.insert(succ.as_str());
                            }
                        }
                    }
                }
            }

            current_level = next_level.into_iter().collect();
        }

        if processed != self.forward.len() {
            return Err(GraphError::CycleError);
        }

        Ok(groups)
    }
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Graph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Graph(nodes={}, edges={})",
            self.size(),
            self.edges().len()
        )
    }
}

impl TraversalGraph for Graph {
    type Error = GraphError;

    fn has_node(&self, node: &str) -> bool {
        Graph::has_node(self, node)
    }

    fn neighbors(&self, node: &str) -> Result<Vec<String>, Self::Error> {
        self.successors(node)
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
    fn test_empty_graph_nodes() {
        let g = Graph::new();
        assert_eq!(g.nodes().len(), 0);
    }

    #[test]
    fn test_empty_graph_edges() {
        let g = Graph::new();
        assert_eq!(g.edges().len(), 0);
    }

    #[test]
    fn test_empty_graph_topo_sort() {
        let g = Graph::new();
        let result = g.topological_sort().unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_empty_graph_size() {
        let g = Graph::new();
        assert_eq!(g.size(), 0);
    }

    #[test]
    fn test_empty_graph_has_cycle() {
        let g = Graph::new();
        assert!(!g.has_cycle());
    }

    #[test]
    fn test_empty_graph_independent_groups() {
        let g = Graph::new();
        let groups = g.independent_groups().unwrap();
        assert!(groups.is_empty());
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Single node tests
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_single_node() {
        let mut g = Graph::new();
        g.add_node("A");
        assert!(g.has_node("A"));
        assert_eq!(g.size(), 1);
    }

    #[test]
    fn test_add_node_idempotent() {
        let mut g = Graph::new();
        g.add_node("A");
        g.add_node("A");
        assert_eq!(g.size(), 1, "duplicate add should be no-op");
    }

    #[test]
    fn test_remove_node() {
        let mut g = Graph::new();
        g.add_node("A");
        g.remove_node("A").unwrap();
        assert!(!g.has_node("A"));
    }

    #[test]
    fn test_remove_node_not_found() {
        let mut g = Graph::new();
        let err = g.remove_node("X").unwrap_err();
        assert_eq!(err, GraphError::NodeNotFound("X".to_string()));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Edge tests
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_add_edge() {
        let mut g = Graph::new();
        g.add_edge("A", "B").unwrap();
        assert!(g.has_edge("A", "B"));
        assert!(!g.has_edge("B", "A"), "should not have edge B->A (directed)");
    }

    #[test]
    fn test_add_edge_implicit_nodes() {
        let mut g = Graph::new();
        g.add_edge("X", "Y").unwrap();
        assert!(g.has_node("X"));
        assert!(g.has_node("Y"));
    }

    #[test]
    fn test_self_loop_error() {
        let mut g = Graph::new();
        let err = g.add_edge("A", "A").unwrap_err();
        assert_eq!(err, GraphError::SelfLoop("A".to_string()));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Self-loop (allowed) tests
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_allow_self_loops_add_edge() {
        let mut g = Graph::new_allow_self_loops();
        g.add_edge("A", "A").unwrap();
        assert!(g.has_edge("A", "A"));
    }

    #[test]
    fn test_allow_self_loops_has_node() {
        let mut g = Graph::new_allow_self_loops();
        g.add_edge("A", "A").unwrap();
        assert!(g.has_node("A"));
        assert_eq!(g.size(), 1);
    }

    #[test]
    fn test_allow_self_loops_successors() {
        let mut g = Graph::new_allow_self_loops();
        g.add_edge("A", "A").unwrap();
        let succs = g.successors("A").unwrap();
        assert_eq!(succs, vec!["A"]);
    }

    #[test]
    fn test_allow_self_loops_predecessors() {
        let mut g = Graph::new_allow_self_loops();
        g.add_edge("A", "A").unwrap();
        let preds = g.predecessors("A").unwrap();
        assert_eq!(preds, vec!["A"]);
    }

    #[test]
    fn test_allow_self_loops_has_cycle() {
        let mut g = Graph::new_allow_self_loops();
        g.add_edge("A", "A").unwrap();
        assert!(g.has_cycle(), "self-loop is a cycle of length 1");
    }

    #[test]
    fn test_allow_self_loops_topo_sort_fails() {
        let mut g = Graph::new_allow_self_loops();
        g.add_edge("A", "A").unwrap();
        let err = g.topological_sort().unwrap_err();
        assert_eq!(err, GraphError::CycleError);
    }

    #[test]
    fn test_allow_self_loops_mixed_edges() {
        let mut g = Graph::new_allow_self_loops();
        g.add_edge("A", "A").unwrap();
        g.add_edge("A", "B").unwrap();
        g.add_edge("B", "C").unwrap();
        assert!(g.has_edge("A", "A"));
        assert!(g.has_edge("A", "B"));
        assert_eq!(g.size(), 3);
    }

    #[test]
    fn test_allow_self_loops_remove_edge() {
        let mut g = Graph::new_allow_self_loops();
        g.add_edge("A", "A").unwrap();
        g.remove_edge("A", "A").unwrap();
        assert!(!g.has_edge("A", "A"));
        assert!(g.has_node("A"), "node should still exist after removing self-loop");
    }

    #[test]
    fn test_allow_self_loops_remove_node() {
        let mut g = Graph::new_allow_self_loops();
        g.add_edge("A", "A").unwrap();
        g.add_edge("A", "B").unwrap();
        g.remove_node("A").unwrap();
        assert!(!g.has_node("A"));
        assert!(!g.has_edge("A", "A"));
        assert!(!g.has_edge("A", "B"));
    }

    #[test]
    fn test_allow_self_loops_edges_output() {
        let mut g = Graph::new_allow_self_loops();
        g.add_edge("A", "A").unwrap();
        g.add_edge("A", "B").unwrap();
        let edges = g.edges();
        assert_eq!(edges.len(), 2);
        assert_eq!(edges[0], ("A".to_string(), "A".to_string()));
        assert_eq!(edges[1], ("A".to_string(), "B".to_string()));
    }

    #[test]
    fn test_default_graph_rejects_self_loops() {
        let mut g = Graph::new();
        let err = g.add_edge("X", "X").unwrap_err();
        assert_eq!(err, GraphError::SelfLoop("X".to_string()));
    }

    #[test]
    fn test_allow_self_loops_normal_edge_still_works() {
        let mut g = Graph::new_allow_self_loops();
        g.add_edge("X", "Y").unwrap();
        assert!(g.has_edge("X", "Y"));
    }

    #[test]
    fn test_allow_self_loops_transitive_closure() {
        let mut g = Graph::new_allow_self_loops();
        g.add_edge("A", "A").unwrap();
        g.add_edge("A", "B").unwrap();
        let closure = g.transitive_closure("A").unwrap();
        assert!(closure.contains("A"), "A should be in its own closure via self-loop");
        assert!(closure.contains("B"));
    }

    #[test]
    fn test_allows_self_loops_flag() {
        let g1 = Graph::new();
        assert!(!g1.allows_self_loops());
        let g2 = Graph::new_allow_self_loops();
        assert!(g2.allows_self_loops());
    }

    #[test]
    fn test_remove_edge() {
        let mut g = Graph::new();
        g.add_edge("A", "B").unwrap();
        g.remove_edge("A", "B").unwrap();
        assert!(!g.has_edge("A", "B"));
        // Nodes should still exist after edge removal
        assert!(g.has_node("A"));
        assert!(g.has_node("B"));
    }

    #[test]
    fn test_remove_edge_not_found() {
        let mut g = Graph::new();
        g.add_node("A");
        let err = g.remove_edge("A", "B").unwrap_err();
        assert_eq!(
            err,
            GraphError::EdgeNotFound("A".to_string(), "B".to_string())
        );
    }

    #[test]
    fn test_remove_node_cleans_edges() {
        let mut g = Graph::new();
        g.add_edge("A", "B").unwrap();
        g.add_edge("B", "C").unwrap();
        g.remove_node("B").unwrap();
        assert!(!g.has_edge("A", "B"));
        assert!(!g.has_edge("B", "C"));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Predecessors and Successors
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_predecessors() {
        let mut g = Graph::new();
        g.add_edge("A", "B").unwrap();
        g.add_edge("C", "B").unwrap();
        let preds = g.predecessors("B").unwrap();
        assert_eq!(preds, vec!["A", "C"]);
    }

    #[test]
    fn test_successors() {
        let mut g = Graph::new();
        g.add_edge("A", "B").unwrap();
        g.add_edge("A", "C").unwrap();
        let succs = g.successors("A").unwrap();
        assert_eq!(succs, vec!["B", "C"]);
    }

    #[test]
    fn test_predecessors_not_found() {
        let g = Graph::new();
        let err = g.predecessors("X").unwrap_err();
        assert_eq!(err, GraphError::NodeNotFound("X".to_string()));
    }

    #[test]
    fn test_successors_not_found() {
        let g = Graph::new();
        let err = g.successors("X").unwrap_err();
        assert_eq!(err, GraphError::NodeNotFound("X".to_string()));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Linear chain: A -> B -> C -> D
    // ═══════════════════════════════════════════════════════════════════════

    fn build_linear_chain() -> Graph {
        let mut g = Graph::new();
        g.add_edge("A", "B").unwrap();
        g.add_edge("B", "C").unwrap();
        g.add_edge("C", "D").unwrap();
        g
    }

    #[test]
    fn test_linear_topo_sort() {
        let g = build_linear_chain();
        let result = g.topological_sort().unwrap();
        assert_eq!(result, vec!["A", "B", "C", "D"]);
    }

    #[test]
    fn test_linear_independent_groups() {
        let g = build_linear_chain();
        let groups = g.independent_groups().unwrap();
        assert_eq!(groups.len(), 4);
        assert_eq!(groups[0], vec!["A"]);
        assert_eq!(groups[1], vec!["B"]);
        assert_eq!(groups[2], vec!["C"]);
        assert_eq!(groups[3], vec!["D"]);
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Diamond: A->B, A->C, B->D, C->D
    // ═══════════════════════════════════════════════════════════════════════

    fn build_diamond() -> Graph {
        let mut g = Graph::new();
        g.add_edge("A", "B").unwrap();
        g.add_edge("A", "C").unwrap();
        g.add_edge("B", "D").unwrap();
        g.add_edge("C", "D").unwrap();
        g
    }

    #[test]
    fn test_diamond_independent_groups() {
        let g = build_diamond();
        let groups = g.independent_groups().unwrap();
        assert_eq!(groups.len(), 3);
        // Level 0: [A]
        assert_eq!(groups[0], vec!["A"]);
        // Level 1: [B, C] — parallel
        assert_eq!(groups[1], vec!["B", "C"]);
        // Level 2: [D]
        assert_eq!(groups[2], vec!["D"]);
    }

    #[test]
    fn test_diamond_transitive_closure() {
        let g = build_diamond();
        let closure = g.transitive_closure("A").unwrap();
        // A can reach B, C, D
        assert_eq!(closure.len(), 3);
        assert!(closure.contains("B"));
        assert!(closure.contains("C"));
        assert!(closure.contains("D"));
    }

    #[test]
    fn test_diamond_transitive_closure_leaf() {
        let g = build_diamond();
        // D is a leaf — nothing is reachable from D
        let deps = g.transitive_closure("D").unwrap();
        assert!(deps.is_empty());
    }

    #[test]
    fn test_diamond_transitive_closure_root() {
        let g = build_diamond();
        // A is the root — everything is reachable from A
        let deps = g.transitive_closure("A").unwrap();
        assert_eq!(deps.len(), 3);
        for node in &["B", "C", "D"] {
            assert!(deps.contains(*node));
        }
    }

    #[test]
    fn test_shared_graph_bfs_works_on_directed_graph() {
        let mut g = Graph::new();
        g.add_edge("A", "B").unwrap();
        g.add_edge("B", "C").unwrap();

        assert_eq!(bfs(&g, "A").unwrap(), vec!["A", "B", "C"]);
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Cycle detection
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_cycle_detection() {
        let mut g = Graph::new();
        g.add_edge("A", "B").unwrap();
        g.add_edge("B", "C").unwrap();
        g.add_edge("C", "A").unwrap(); // cycle!
        assert!(g.has_cycle());
    }

    #[test]
    fn test_cycle_in_topo_sort() {
        let mut g = Graph::new();
        g.add_edge("A", "B").unwrap();
        g.add_edge("B", "C").unwrap();
        g.add_edge("C", "A").unwrap();
        let err = g.topological_sort().unwrap_err();
        assert_eq!(err, GraphError::CycleError);
    }

    #[test]
    fn test_no_cycle() {
        let g = build_diamond();
        assert!(!g.has_cycle());
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Affected nodes
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_affected_nodes_leaf() {
        let g = build_diamond();
        // D is a leaf — nothing depends on D, so only D is affected
        let changed: HashSet<String> = ["D".to_string()].into_iter().collect();
        let affected = g.affected_nodes(&changed);
        assert_eq!(affected.len(), 1);
        assert!(affected.contains("D"));
    }

    #[test]
    fn test_affected_nodes_root() {
        let g = build_diamond();
        // A is the root — everything depends on A
        // If A changes, A + B + C + D all need rebuilding
        let changed: HashSet<String> = ["A".to_string()].into_iter().collect();
        let affected = g.affected_nodes(&changed);
        assert_eq!(affected.len(), 4);
        for node in &["A", "B", "C", "D"] {
            assert!(affected.contains(*node));
        }
    }

    #[test]
    fn test_affected_nodes_middle() {
        let g = build_diamond();
        // B is in the middle — D depends on B
        let changed: HashSet<String> = ["B".to_string()].into_iter().collect();
        let affected = g.affected_nodes(&changed);
        assert_eq!(affected.len(), 2);
        assert!(affected.contains("B"));
        assert!(affected.contains("D"));
    }

    #[test]
    fn test_affected_nodes_nonexistent() {
        let g = build_diamond();
        let changed: HashSet<String> = ["X".to_string()].into_iter().collect();
        let affected = g.affected_nodes(&changed);
        assert!(affected.is_empty());
    }

    #[test]
    fn test_transitive_closure_not_found() {
        let g = Graph::new();
        let err = g.transitive_closure("X").unwrap_err();
        assert_eq!(err, GraphError::NodeNotFound("X".to_string()));
    }

    #[test]
    fn test_edges_returns_sorted() {
        let mut g = Graph::new();
        g.add_edge("C", "D").unwrap();
        g.add_edge("A", "B").unwrap();
        let edges = g.edges();
        assert_eq!(edges[0], ("A".to_string(), "B".to_string()));
        assert_eq!(edges[1], ("C".to_string(), "D".to_string()));
    }

    #[test]
    fn test_has_edge_no_node() {
        let g = Graph::new();
        assert!(!g.has_edge("X", "Y"));
    }

    #[test]
    fn test_display() {
        let g = build_diamond();
        let s = format!("{}", g);
        assert_eq!(s, "Graph(nodes=4, edges=4)");
    }

    #[test]
    fn test_default() {
        let g = Graph::default();
        assert_eq!(g.size(), 0);
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Real repo dependency graph
    // ═══════════════════════════════════════════════════════════════════════

    fn build_repo_graph() -> Graph {
        let mut g = Graph::new();
        // Independent roots
        for pkg in &[
            "logic-gates",
            "grammar-tools",
            "virtual-machine",
            "jvm-simulator",
            "clr-simulator",
            "wasm-simulator",
            "intel4004-simulator",
            "html-renderer",
        ] {
            g.add_node(pkg);
        }
        // Dependency edges (from dependency TO dependent)
        g.add_edge("logic-gates", "arithmetic").unwrap();
        g.add_edge("arithmetic", "cpu-simulator").unwrap();
        g.add_edge("cpu-simulator", "arm-simulator").unwrap();
        g.add_edge("cpu-simulator", "riscv-simulator").unwrap();
        g.add_edge("grammar-tools", "lexer").unwrap();
        g.add_edge("lexer", "parser").unwrap();
        g.add_edge("grammar-tools", "parser").unwrap();
        g.add_edge("lexer", "bytecode-compiler").unwrap();
        g.add_edge("parser", "bytecode-compiler").unwrap();
        g.add_edge("virtual-machine", "bytecode-compiler").unwrap();
        g.add_edge("lexer", "pipeline").unwrap();
        g.add_edge("parser", "pipeline").unwrap();
        g.add_edge("bytecode-compiler", "pipeline").unwrap();
        g.add_edge("virtual-machine", "pipeline").unwrap();
        g.add_edge("arm-simulator", "assembler").unwrap();
        g.add_edge("virtual-machine", "jit-compiler").unwrap();
        g.add_edge("assembler", "jit-compiler").unwrap();
        g
    }

    #[test]
    fn test_repo_graph_no_cycle() {
        let g = build_repo_graph();
        assert!(!g.has_cycle());
    }

    #[test]
    fn test_repo_graph_topo_sort() {
        let g = build_repo_graph();
        let order = g.topological_sort().unwrap();
        assert_eq!(order.len(), g.size());

        // Verify ordering: every dependency appears before its dependent
        let pos: HashMap<&str, usize> = order
            .iter()
            .enumerate()
            .map(|(i, n)| (n.as_str(), i))
            .collect();
        for (from, to) in g.edges() {
            assert!(
                pos[from.as_str()] < pos[to.as_str()],
                "{} should come before {} in topo sort",
                from,
                to
            );
        }
    }

    #[test]
    fn test_repo_graph_independent_groups() {
        let g = build_repo_graph();
        let groups = g.independent_groups().unwrap();

        // Level 0 should contain all independent roots
        let level0: HashSet<&str> = groups[0].iter().map(|s| s.as_str()).collect();
        for root in &[
            "logic-gates",
            "grammar-tools",
            "virtual-machine",
            "jvm-simulator",
            "clr-simulator",
        ] {
            assert!(
                level0.contains(root),
                "expected {} in level 0, got {:?}",
                root,
                groups[0]
            );
        }
        println!("Groups: {:?}", groups);
    }
}
