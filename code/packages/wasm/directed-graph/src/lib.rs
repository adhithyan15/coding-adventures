// lib.rs -- wasm-bindgen wrapper for the Rust directed-graph crate
// =================================================================
//
// This wrapper compiles the Rust directed-graph library to WebAssembly,
// making it usable from JavaScript in any environment that supports WASM:
//
// - Browsers (Chrome, Firefox, Safari, Edge)
// - Node.js (via WebAssembly.instantiate or wasm-pack generated bindings)
// - Deno (native WASM support)
// - Cloudflare Workers, Vercel Edge, etc.
// - Any WASM runtime (wasmtime, wasmer) — can be loaded from Python, Ruby, etc.
//
// # How wasm-bindgen works
//
// Rust and JavaScript have different type systems. wasm-bindgen generates
// JavaScript "glue code" that handles the conversion:
//
// - Rust `String` ↔ JS `string` (copied across the WASM boundary)
// - Rust `Vec<String>` → JS `string[]` (via serde-wasm-bindgen)
// - Rust `HashSet<String>` → JS `string[]` (via serde-wasm-bindgen)
// - Rust `Vec<Vec<String>>` → JS `string[][]` (via serde-wasm-bindgen)
// - Rust errors → JS `Error` (thrown as exceptions)
//
// # Performance note
//
// Strings cross the WASM boundary by copy (WASM has linear memory, JS has
// its own string storage). For a graph library where operations are
// algorithm-heavy and calls are infrequent, this overhead is negligible.
// The graph traversal itself runs at native WASM speed.
//
// # Usage from JavaScript
//
// ```javascript
// import init, { DirectedGraph } from './directed_graph_wasm.js';
//
// await init();  // load the WASM module
//
// const g = new DirectedGraph();
// g.addEdge("compile", "link");
// g.addEdge("link", "package");
//
// console.log(g.topologicalSort());    // ['compile', 'link', 'package']
// console.log(g.independentGroups());  // [['compile'], ['link'], ['package']]
// ```

use std::collections::HashSet;

use directed_graph::graph::{Graph, GraphError};
use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// Error conversion
// ---------------------------------------------------------------------------

/// Convert a Rust GraphError into a JavaScript Error.
fn to_js_err(err: GraphError) -> JsError {
    JsError::new(&err.to_string())
}

// ---------------------------------------------------------------------------
// DirectedGraph — the main WASM class
// ---------------------------------------------------------------------------

/// A directed graph compiled to WebAssembly.
///
/// This class provides the same algorithms as the Python and Ruby native
/// extensions, but runs in any WASM-capable environment (browsers, Node.js,
/// Deno, edge runtimes, or standalone WASM runtimes).
#[wasm_bindgen]
pub struct DirectedGraph {
    inner: Graph,
}

#[wasm_bindgen]
impl DirectedGraph {
    // -- Constructor -------------------------------------------------------

    /// Create a new empty directed graph.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        DirectedGraph {
            inner: Graph::new(),
        }
    }

    // -- Node operations ---------------------------------------------------

    /// Add a node to the graph. If the node already exists, this is a no-op.
    #[wasm_bindgen(js_name = addNode)]
    pub fn add_node(&mut self, node: &str) {
        self.inner.add_node(node);
    }

    /// Remove a node and all its edges from the graph.
    /// Throws an Error if the node does not exist.
    #[wasm_bindgen(js_name = removeNode)]
    pub fn remove_node(&mut self, node: &str) -> Result<(), JsError> {
        self.inner.remove_node(node).map_err(to_js_err)
    }

    /// Check whether a node exists in the graph.
    #[wasm_bindgen(js_name = hasNode)]
    pub fn has_node(&self, node: &str) -> bool {
        self.inner.has_node(node)
    }

    /// Return a sorted array of all nodes in the graph.
    #[wasm_bindgen]
    pub fn nodes(&self) -> JsValue {
        let nodes = self.inner.nodes();
        serde_wasm_bindgen::to_value(&nodes).unwrap_or(JsValue::NULL)
    }

    /// Return the number of nodes in the graph.
    #[wasm_bindgen]
    pub fn size(&self) -> usize {
        self.inner.size()
    }

    // -- Edge operations ---------------------------------------------------

    /// Add a directed edge from `fromNode` to `toNode`.
    /// Both nodes are created if they don't exist yet.
    /// Throws an Error if fromNode === toNode (self-loops not allowed).
    #[wasm_bindgen(js_name = addEdge)]
    pub fn add_edge(&mut self, from_node: &str, to_node: &str) -> Result<(), JsError> {
        self.inner.add_edge(from_node, to_node).map_err(to_js_err)
    }

    /// Remove a directed edge.
    /// Throws an Error if the edge does not exist.
    #[wasm_bindgen(js_name = removeEdge)]
    pub fn remove_edge(&mut self, from_node: &str, to_node: &str) -> Result<(), JsError> {
        self.inner
            .remove_edge(from_node, to_node)
            .map_err(to_js_err)
    }

    /// Check whether a directed edge exists.
    #[wasm_bindgen(js_name = hasEdge)]
    pub fn has_edge(&self, from_node: &str, to_node: &str) -> bool {
        self.inner.has_edge(from_node, to_node)
    }

    /// Return all edges as an array of [from, to] pairs, sorted.
    #[wasm_bindgen]
    pub fn edges(&self) -> JsValue {
        let edges = self.inner.edges();
        serde_wasm_bindgen::to_value(&edges).unwrap_or(JsValue::NULL)
    }

    // -- Neighbor queries --------------------------------------------------

    /// Return the predecessors (parents) of a node.
    /// Throws an Error if the node does not exist.
    #[wasm_bindgen]
    pub fn predecessors(&self, node: &str) -> Result<JsValue, JsError> {
        let preds = self.inner.predecessors(node).map_err(to_js_err)?;
        Ok(serde_wasm_bindgen::to_value(&preds).unwrap_or(JsValue::NULL))
    }

    /// Return the successors (children) of a node.
    /// Throws an Error if the node does not exist.
    #[wasm_bindgen]
    pub fn successors(&self, node: &str) -> Result<JsValue, JsError> {
        let succs = self.inner.successors(node).map_err(to_js_err)?;
        Ok(serde_wasm_bindgen::to_value(&succs).unwrap_or(JsValue::NULL))
    }

    // -- Algorithms --------------------------------------------------------

    /// Return a topological ordering of the graph (Kahn's algorithm).
    /// Throws an Error if the graph contains a cycle.
    #[wasm_bindgen(js_name = topologicalSort)]
    pub fn topological_sort(&self) -> Result<JsValue, JsError> {
        let order = self.inner.topological_sort().map_err(to_js_err)?;
        Ok(serde_wasm_bindgen::to_value(&order).unwrap_or(JsValue::NULL))
    }

    /// Check whether the graph contains a cycle.
    #[wasm_bindgen(js_name = hasCycle)]
    pub fn has_cycle(&self) -> bool {
        self.inner.has_cycle()
    }

    /// Return the set of all nodes reachable from `node` (transitive closure).
    /// Does NOT include the node itself.
    /// Throws an Error if the node does not exist.
    #[wasm_bindgen(js_name = transitiveClosure)]
    pub fn transitive_closure(&self, node: &str) -> Result<JsValue, JsError> {
        let closure = self.inner.transitive_closure(node).map_err(to_js_err)?;
        // Convert HashSet to sorted Vec for deterministic JS output.
        let mut sorted: Vec<String> = closure.into_iter().collect();
        sorted.sort();
        Ok(serde_wasm_bindgen::to_value(&sorted).unwrap_or(JsValue::NULL))
    }

    /// Given a set of changed nodes, return all affected nodes.
    /// Input is a JavaScript array of strings.
    #[wasm_bindgen(js_name = affectedNodes)]
    pub fn affected_nodes(&self, changed: JsValue) -> Result<JsValue, JsError> {
        let changed_vec: Vec<String> = serde_wasm_bindgen::from_value(changed)
            .map_err(|e| JsError::new(&format!("invalid input: {}", e)))?;
        let changed_set: HashSet<String> = changed_vec.into_iter().collect();
        let affected = self.inner.affected_nodes(&changed_set);
        let mut sorted: Vec<String> = affected.into_iter().collect();
        sorted.sort();
        Ok(serde_wasm_bindgen::to_value(&sorted).unwrap_or(JsValue::NULL))
    }

    /// Partition the graph into independent groups (parallel execution levels).
    /// Throws an Error if the graph contains a cycle.
    #[wasm_bindgen(js_name = independentGroups)]
    pub fn independent_groups(&self) -> Result<JsValue, JsError> {
        let groups = self.inner.independent_groups().map_err(to_js_err)?;
        Ok(serde_wasm_bindgen::to_value(&groups).unwrap_or(JsValue::NULL))
    }
}

// ---------------------------------------------------------------------------
// Tests (run with wasm-pack test)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_node_and_size() {
        let mut g = DirectedGraph::new();
        assert_eq!(g.size(), 0);
        g.add_node("A");
        assert_eq!(g.size(), 1);
        g.add_node("B");
        assert_eq!(g.size(), 2);
    }

    #[test]
    fn test_add_and_has_edge() {
        let mut g = DirectedGraph::new();
        g.add_edge("A", "B").unwrap();
        assert!(g.has_edge("A", "B"));
        assert!(!g.has_edge("B", "A"));
    }

    #[test]
    fn test_has_node() {
        let mut g = DirectedGraph::new();
        g.add_node("A");
        assert!(g.has_node("A"));
        assert!(!g.has_node("B"));
    }

    #[test]
    fn test_remove_node() {
        let mut g = DirectedGraph::new();
        g.add_edge("A", "B").unwrap();
        g.remove_node("A").unwrap();
        assert!(!g.has_node("A"));
        assert!(!g.has_edge("A", "B"));
    }

    #[test]
    fn test_self_loop_rejected() {
        let mut g = DirectedGraph::new();
        assert!(g.add_edge("A", "A").is_err());
    }

    #[test]
    fn test_has_cycle() {
        let mut g = DirectedGraph::new();
        g.add_edge("A", "B").unwrap();
        g.add_edge("B", "A").unwrap();
        assert!(g.has_cycle());
    }

    #[test]
    fn test_no_cycle() {
        let mut g = DirectedGraph::new();
        g.add_edge("A", "B").unwrap();
        g.add_edge("B", "C").unwrap();
        assert!(!g.has_cycle());
    }
}
