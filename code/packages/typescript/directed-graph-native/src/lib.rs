// lib.rs -- napi-rs wrapper for the Rust directed-graph crate
// ============================================================
//
// This is a thin wrapper that exposes the Rust `directed_graph::Graph` type
// to Node.js via napi-rs. The wrapper handles type conversion at the boundary:
//
// - Rust `String` <-> JavaScript `string`
// - Rust `Vec<String>` <-> JavaScript `string[]`
// - Rust `Vec<Vec<String>>` <-> JavaScript `string[][]`
// - Rust `HashSet<String>` <-> JavaScript `string[]` (sets don't cross napi directly)
// - Rust `Vec<(String, String)>` <-> JavaScript `[string, string][]`
// - Rust `GraphError` -> JavaScript `Error` (with descriptive messages)
//
// The core algorithms (topological sort, cycle detection, transitive closure,
// independent groups, affected nodes) all live in the Rust crate. This file
// contains zero algorithm logic -- it's pure glue code.
//
// # Why a native extension?
//
// The pure TypeScript `@coding-adventures/directed-graph` package works fine,
// but for large graphs (thousands of nodes) the Rust implementation is
// significantly faster because:
//
// 1. No GC pressure -- Rust manages memory directly.
// 2. Cache-friendly data structures -- BTreeSet for sorted iteration.
// 3. Zero-cost abstractions -- iterators compile to tight loops.
//
// # Usage from Node.js / TypeScript
//
// ```typescript
// import { DirectedGraph } from '@coding-adventures/directed-graph-native';
//
// const g = new DirectedGraph();
// g.addEdge("compile", "link");
// g.addEdge("link", "package");
//
// console.log(g.topologicalSort());    // ['compile', 'link', 'package']
// console.log(g.independentGroups());  // [['compile'], ['link'], ['package']]
// ```
//
// # Error handling
//
// Rust's `GraphError` enum maps to JavaScript `Error` instances with descriptive
// messages. The error messages include prefixes that identify the error type:
//
// - "CycleError: ..." for cycle-related errors
// - "NodeNotFoundError: ..." for missing nodes
// - "EdgeNotFoundError: ..." for missing edges
// - "SelfLoopError: ..." for self-loop attempts
//
// This lets JavaScript code match on the message prefix to distinguish error types.

use std::collections::HashSet;

use directed_graph::graph::{Graph, GraphError};
use napi::bindgen_prelude::*;
use napi_derive::napi;

// ---------------------------------------------------------------------------
// Error conversion
// ---------------------------------------------------------------------------
//
// napi-rs uses `napi::Error` for all errors crossing the Rust/JS boundary.
// We convert each `GraphError` variant into a descriptive error message with
// a prefix that JavaScript code can use to identify the error type.
//
// Why prefixes instead of custom error classes? napi-rs doesn't support
// throwing custom JavaScript error subclasses directly from Rust. The
// TypeScript wrapper (index.d.ts) documents the error types, and test code
// can match on the message string.

fn to_napi_err(err: GraphError) -> napi::Error {
    match err {
        GraphError::CycleError => {
            napi::Error::new(Status::GenericFailure, "CycleError: graph contains a cycle")
        }
        GraphError::NodeNotFound(node) => napi::Error::new(
            Status::GenericFailure,
            format!("NodeNotFoundError: node not found: {}", node),
        ),
        GraphError::EdgeNotFound(from, to) => napi::Error::new(
            Status::GenericFailure,
            format!("EdgeNotFoundError: edge not found: {} -> {}", from, to),
        ),
        GraphError::SelfLoop(node) => napi::Error::new(
            Status::GenericFailure,
            format!("SelfLoopError: self-loop not allowed: {}", node),
        ),
    }
}

// ---------------------------------------------------------------------------
// Edge -- a simple struct to represent a directed edge as a JS object
// ---------------------------------------------------------------------------
//
// napi-rs can return tuples as arrays, but using a named struct with #[napi(object)]
// gives us a proper JavaScript object with named fields. However, to match the
// Python wrapper's behavior (which returns tuples), we'll return arrays of
// two-element arrays instead. We use a helper function for this.

// ---------------------------------------------------------------------------
// DirectedGraph -- the main JavaScript/TypeScript class
// ---------------------------------------------------------------------------
//
// This struct wraps the Rust `Graph` and exposes all its methods to JavaScript.
// The `#[napi]` attribute on the impl block generates the N-API bindings
// automatically. Each method's signature is translated to its JavaScript
// equivalent:
//
//   Rust `fn add_node(&mut self, node: String)` -> JS `addNode(node: string): void`
//   Rust `fn nodes(&self) -> Vec<String>` -> JS `nodes(): string[]`
//   Rust `fn topological_sort(&self) -> napi::Result<Vec<String>>` -> JS `topologicalSort(): string[]` (throws on error)

#[napi]
pub struct DirectedGraph {
    inner: Graph,
}

#[napi]
impl DirectedGraph {
    // -- Constructor -------------------------------------------------------
    //
    // Creates a new empty directed graph. This is called when JavaScript code
    // does `new DirectedGraph()`.

    #[napi(constructor)]
    pub fn new() -> Self {
        DirectedGraph {
            inner: Graph::new(),
        }
    }

    // -- Node operations ---------------------------------------------------
    //
    // These methods manage individual nodes in the graph. Nodes are identified
    // by string names. Adding a node that already exists is a no-op (idempotent).

    /// Add a node to the graph. If the node already exists, this is a no-op.
    ///
    /// ```typescript
    /// g.addNode("compile");
    /// g.addNode("compile"); // no-op, already exists
    /// ```
    #[napi]
    pub fn add_node(&mut self, node: String) {
        self.inner.add_node(&node);
    }

    /// Remove a node and all its edges from the graph.
    ///
    /// Throws if the node does not exist.
    ///
    /// ```typescript
    /// g.addNode("temp");
    /// g.removeNode("temp");    // OK
    /// g.removeNode("missing"); // throws NodeNotFoundError
    /// ```
    #[napi]
    pub fn remove_node(&mut self, node: String) -> napi::Result<()> {
        self.inner.remove_node(&node).map_err(to_napi_err)
    }

    /// Check whether a node exists in the graph.
    ///
    /// ```typescript
    /// g.addNode("A");
    /// g.hasNode("A"); // true
    /// g.hasNode("Z"); // false
    /// ```
    #[napi]
    pub fn has_node(&self, node: String) -> bool {
        self.inner.has_node(&node)
    }

    /// Return a sorted list of all nodes in the graph.
    ///
    /// The list is sorted alphabetically by node name. This matches the Rust
    /// implementation which uses BTreeSet for deterministic iteration order.
    ///
    /// ```typescript
    /// g.addNode("C");
    /// g.addNode("A");
    /// g.addNode("B");
    /// g.nodes(); // ["A", "B", "C"]
    /// ```
    #[napi]
    pub fn nodes(&self) -> Vec<String> {
        self.inner.nodes()
    }

    // -- Edge operations ---------------------------------------------------
    //
    // Edges are directed: addEdge("A", "B") means A points to B.
    // Both endpoint nodes are created automatically if they don't exist.

    /// Add a directed edge from `fromNode` to `toNode`.
    ///
    /// Both nodes are created if they don't exist yet. Self-loops (fromNode == toNode)
    /// are rejected with a SelfLoopError.
    ///
    /// ```typescript
    /// g.addEdge("compile", "link"); // creates both nodes + edge
    /// g.addEdge("A", "A");          // throws SelfLoopError
    /// ```
    #[napi]
    pub fn add_edge(&mut self, from_node: String, to_node: String) -> napi::Result<()> {
        self.inner
            .add_edge(&from_node, &to_node)
            .map_err(to_napi_err)
    }

    /// Remove a directed edge from `fromNode` to `toNode`.
    ///
    /// Throws if the edge does not exist. The endpoint nodes are NOT removed.
    ///
    /// ```typescript
    /// g.addEdge("A", "B");
    /// g.removeEdge("A", "B"); // OK, edge removed, nodes A and B remain
    /// g.removeEdge("A", "B"); // throws EdgeNotFoundError
    /// ```
    #[napi]
    pub fn remove_edge(&mut self, from_node: String, to_node: String) -> napi::Result<()> {
        self.inner
            .remove_edge(&from_node, &to_node)
            .map_err(to_napi_err)
    }

    /// Check whether a directed edge exists from `fromNode` to `toNode`.
    ///
    /// Note that edges are directional: hasEdge("A", "B") does NOT imply
    /// hasEdge("B", "A").
    ///
    /// ```typescript
    /// g.addEdge("A", "B");
    /// g.hasEdge("A", "B"); // true
    /// g.hasEdge("B", "A"); // false (directed!)
    /// ```
    #[napi]
    pub fn has_edge(&self, from_node: String, to_node: String) -> bool {
        self.inner.has_edge(&from_node, &to_node)
    }

    /// Return a list of all edges as `[from, to]` pairs, sorted.
    ///
    /// The edges are sorted lexicographically by (from, to). Each edge is
    /// returned as a two-element array `[string, string]`.
    ///
    /// ```typescript
    /// g.addEdge("B", "C");
    /// g.addEdge("A", "B");
    /// g.edges(); // [["A", "B"], ["B", "C"]]
    /// ```
    #[napi]
    pub fn edges(&self) -> Vec<Vec<String>> {
        // napi-rs doesn't support returning Vec<(String, String)> directly,
        // so we convert each tuple into a two-element Vec. On the JavaScript
        // side, this becomes an array of [string, string] pairs.
        self.inner
            .edges()
            .into_iter()
            .map(|(from, to)| vec![from, to])
            .collect()
    }

    // -- Neighbor queries --------------------------------------------------
    //
    // These methods return the immediate neighbors of a node. "Predecessors"
    // are nodes that point TO the given node. "Successors" are nodes that
    // the given node points TO.

    /// Return the predecessors (parents) of a node -- nodes that point TO it.
    ///
    /// Throws if the node does not exist.
    ///
    /// ```typescript
    /// g.addEdge("A", "C");
    /// g.addEdge("B", "C");
    /// g.predecessors("C"); // ["A", "B"]
    /// ```
    #[napi]
    pub fn predecessors(&self, node: String) -> napi::Result<Vec<String>> {
        self.inner.predecessors(&node).map_err(to_napi_err)
    }

    /// Return the successors (children) of a node -- nodes it points TO.
    ///
    /// Throws if the node does not exist.
    ///
    /// ```typescript
    /// g.addEdge("A", "B");
    /// g.addEdge("A", "C");
    /// g.successors("A"); // ["B", "C"]
    /// ```
    #[napi]
    pub fn successors(&self, node: String) -> napi::Result<Vec<String>> {
        self.inner.successors(&node).map_err(to_napi_err)
    }

    // -- Graph properties --------------------------------------------------

    /// Return the number of nodes in the graph.
    ///
    /// ```typescript
    /// g.addNode("A");
    /// g.addNode("B");
    /// g.size(); // 2
    /// ```
    #[napi]
    pub fn size(&self) -> u32 {
        // napi-rs uses u32 for safe integer conversion to JavaScript numbers.
        // usize -> u32 is fine because we won't have 4 billion+ nodes.
        self.inner.size() as u32
    }

    /// Return the number of edges in the graph.
    ///
    /// ```typescript
    /// g.addEdge("A", "B");
    /// g.addEdge("B", "C");
    /// g.edgeCount(); // 2
    /// ```
    #[napi]
    pub fn edge_count(&self) -> u32 {
        self.inner.edges().len() as u32
    }

    /// Return a human-readable string representation of the graph.
    ///
    /// Format: `DirectedGraph(nodes=N, edges=M)`
    ///
    /// ```typescript
    /// g.addEdge("A", "B");
    /// g.toString(); // "DirectedGraph(nodes=2, edges=1)"
    /// ```
    #[napi]
    pub fn to_string_repr(&self) -> String {
        format!(
            "DirectedGraph(nodes={}, edges={})",
            self.inner.size(),
            self.inner.edges().len()
        )
    }

    // -- Algorithms --------------------------------------------------------
    //
    // These methods implement graph algorithms. They all delegate to the
    // Rust `Graph` type and convert errors to napi::Error.

    /// Return a topological ordering of the graph (Kahn's algorithm).
    ///
    /// Every node appears after all of its dependencies. This gives a valid
    /// build order for the dependency graph.
    ///
    /// Throws CycleError if the graph contains a cycle.
    ///
    /// ```typescript
    /// g.addEdge("A", "B");
    /// g.addEdge("B", "C");
    /// g.topologicalSort(); // ["A", "B", "C"]
    /// ```
    #[napi]
    pub fn topological_sort(&self) -> napi::Result<Vec<String>> {
        self.inner.topological_sort().map_err(to_napi_err)
    }

    /// Check whether the graph contains a cycle (DFS with 3-color marking).
    ///
    /// ```typescript
    /// g.addEdge("A", "B");
    /// g.addEdge("B", "A");
    /// g.hasCycle(); // true
    /// ```
    #[napi]
    pub fn has_cycle(&self) -> bool {
        self.inner.has_cycle()
    }

    /// Return all nodes reachable from `node` (transitive closure).
    ///
    /// Does NOT include the node itself. Returns an array of node names
    /// (JavaScript doesn't have a native Set crossing the napi boundary,
    /// so we return a sorted array instead).
    ///
    /// Throws NodeNotFoundError if the node does not exist.
    ///
    /// ```typescript
    /// g.addEdge("A", "B");
    /// g.addEdge("B", "C");
    /// g.transitiveClosure("A"); // ["B", "C"]
    /// ```
    #[napi]
    pub fn transitive_closure(&self, node: String) -> napi::Result<Vec<String>> {
        let closure = self.inner.transitive_closure(&node).map_err(to_napi_err)?;
        // Convert HashSet to sorted Vec for deterministic output.
        let mut result: Vec<String> = closure.into_iter().collect();
        result.sort();
        Ok(result)
    }

    /// Given a list of changed nodes, return all nodes that are transitively
    /// affected (the changed nodes plus everything that depends on them).
    ///
    /// This is the core of incremental build detection: "if these packages
    /// changed, what else needs rebuilding?"
    ///
    /// Unknown nodes are silently ignored (they might have been removed).
    ///
    /// ```typescript
    /// g.addEdge("A", "B");
    /// g.addEdge("B", "C");
    /// g.affectedNodes(["A"]); // ["A", "B", "C"]
    /// ```
    #[napi]
    pub fn affected_nodes(&self, changed: Vec<String>) -> Vec<String> {
        // Convert the JavaScript array to a HashSet for the Rust API.
        let changed_set: HashSet<String> = changed.into_iter().collect();
        let affected = self.inner.affected_nodes(&changed_set);
        // Convert back to sorted Vec for deterministic JavaScript output.
        let mut result: Vec<String> = affected.into_iter().collect();
        result.sort();
        result
    }

    /// Partition the graph into independent groups (parallel execution levels).
    ///
    /// Level 0 contains nodes with no dependencies. Level 1 depends only on
    /// level 0. And so on. Nodes within the same level can be executed in
    /// parallel because they have no dependencies between them.
    ///
    /// Throws CycleError if the graph contains a cycle.
    ///
    /// ```typescript
    /// g.addEdge("A", "B");
    /// g.addEdge("A", "C");
    /// g.addEdge("B", "D");
    /// g.addEdge("C", "D");
    /// g.independentGroups(); // [["A"], ["B", "C"], ["D"]]
    /// ```
    #[napi]
    pub fn independent_groups(&self) -> napi::Result<Vec<Vec<String>>> {
        self.inner.independent_groups().map_err(to_napi_err)
    }
}
