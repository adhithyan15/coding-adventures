// lib.rs -- PyO3 wrapper for the Rust directed-graph crate
// =========================================================
//
// This is a thin wrapper that exposes the Rust `directed_graph::Graph` type
// to Python via PyO3. The wrapper handles type conversion at the boundary:
//
// - Rust `String` ↔ Python `str`
// - Rust `Vec<String>` ↔ Python `list[str]`
// - Rust `Vec<Vec<String>>` ↔ Python `list[list[str]]`
// - Rust `HashSet<String>` ↔ Python `set[str]`
// - Rust `GraphError` → Python exceptions
//
// The core algorithms (topological sort, cycle detection, transitive closure,
// independent groups, affected nodes) all live in the Rust crate. This file
// contains zero algorithm logic — it's pure glue code.
//
// # Why a native extension?
//
// The pure Python `directed_graph` package works fine, but for large graphs
// (thousands of nodes) the Rust implementation is significantly faster because:
//
// 1. No GC pressure — Rust manages memory directly.
// 2. Cache-friendly data structures — BTreeSet for sorted iteration.
// 3. Zero-cost abstractions — iterators compile to tight loops.
//
// # Usage from Python
//
// ```python
// from directed_graph_native import DirectedGraph, CycleError
//
// g = DirectedGraph()
// g.add_edge("A", "B")
// g.add_edge("B", "C")
//
// print(g.topological_sort())    # ['A', 'B', 'C']
// print(g.independent_groups())  # [['A'], ['B'], ['C']]
// ```

use std::collections::HashSet;

use directed_graph::graph::{Graph, GraphError};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

// ---------------------------------------------------------------------------
// Python exception types
// ---------------------------------------------------------------------------
//
// We define custom Python exceptions that mirror the Rust error variants.
// These allow Python users to catch specific errors:
//
//     try:
//         g.topological_sort()
//     except CycleError:
//         print("graph has a cycle!")

pyo3::create_exception!(directed_graph_native, CycleError, pyo3::exceptions::PyException);
pyo3::create_exception!(directed_graph_native, NodeNotFoundError, pyo3::exceptions::PyException);
pyo3::create_exception!(directed_graph_native, EdgeNotFoundError, pyo3::exceptions::PyException);

/// Convert a Rust GraphError into the appropriate Python exception.
fn to_py_err(err: GraphError) -> PyErr {
    match err {
        GraphError::CycleError => CycleError::new_err("graph contains a cycle"),
        GraphError::NodeNotFound(node) => {
            NodeNotFoundError::new_err(format!("node not found: {}", node))
        }
        GraphError::EdgeNotFound(from, to) => {
            EdgeNotFoundError::new_err(format!("edge not found: {} -> {}", from, to))
        }
        GraphError::SelfLoop(node) => {
            PyValueError::new_err(format!("self-loop not allowed: {}", node))
        }
    }
}

// ---------------------------------------------------------------------------
// DirectedGraph — the main Python class
// ---------------------------------------------------------------------------

/// A directed graph backed by Rust's directed-graph crate.
///
/// This class provides the same API as the pure Python ``DirectedGraph``,
/// but the algorithms run in Rust for better performance on large graphs.
///
/// Example:
///     >>> g = DirectedGraph()
///     >>> g.add_edge("compile", "link")
///     >>> g.add_edge("link", "package")
///     >>> g.topological_sort()
///     ['compile', 'link', 'package']
#[pyclass(name = "DirectedGraph")]
struct PyDirectedGraph {
    inner: Graph,
}

#[pymethods]
impl PyDirectedGraph {
    // -- Constructor -------------------------------------------------------

    /// Create a new empty directed graph.
    #[new]
    fn new() -> Self {
        PyDirectedGraph {
            inner: Graph::new(),
        }
    }

    // -- Node operations ---------------------------------------------------

    /// Add a node to the graph. If the node already exists, this is a no-op.
    fn add_node(&mut self, node: &str) {
        self.inner.add_node(node);
    }

    /// Remove a node and all its edges from the graph.
    ///
    /// Raises:
    ///     NodeNotFoundError: If the node does not exist.
    fn remove_node(&mut self, node: &str) -> PyResult<()> {
        self.inner.remove_node(node).map_err(to_py_err)
    }

    /// Check whether a node exists in the graph.
    fn has_node(&self, node: &str) -> bool {
        self.inner.has_node(node)
    }

    /// Return a sorted list of all nodes in the graph.
    fn nodes(&self) -> Vec<String> {
        self.inner.nodes()
    }

    // -- Edge operations ---------------------------------------------------

    /// Add a directed edge from ``from_node`` to ``to_node``.
    ///
    /// Both nodes are created if they don't exist yet.
    ///
    /// Raises:
    ///     ValueError: If ``from_node == to_node`` (self-loops not allowed).
    fn add_edge(&mut self, from_node: &str, to_node: &str) -> PyResult<()> {
        self.inner.add_edge(from_node, to_node).map_err(to_py_err)
    }

    /// Remove a directed edge from ``from_node`` to ``to_node``.
    ///
    /// Raises:
    ///     EdgeNotFoundError: If the edge does not exist.
    fn remove_edge(&mut self, from_node: &str, to_node: &str) -> PyResult<()> {
        self.inner
            .remove_edge(from_node, to_node)
            .map_err(to_py_err)
    }

    /// Check whether a directed edge exists from ``from_node`` to ``to_node``.
    fn has_edge(&self, from_node: &str, to_node: &str) -> bool {
        self.inner.has_edge(from_node, to_node)
    }

    /// Return a list of all edges as ``(from, to)`` tuples, sorted.
    fn edges(&self) -> Vec<(String, String)> {
        self.inner.edges()
    }

    // -- Neighbor queries --------------------------------------------------

    /// Return the predecessors (parents) of a node — nodes that point TO it.
    ///
    /// Raises:
    ///     NodeNotFoundError: If the node does not exist.
    fn predecessors(&self, node: &str) -> PyResult<Vec<String>> {
        self.inner.predecessors(node).map_err(to_py_err)
    }

    /// Return the successors (children) of a node — nodes it points TO.
    ///
    /// Raises:
    ///     NodeNotFoundError: If the node does not exist.
    fn successors(&self, node: &str) -> PyResult<Vec<String>> {
        self.inner.successors(node).map_err(to_py_err)
    }

    // -- Graph properties --------------------------------------------------

    /// Return the number of nodes in the graph.
    fn __len__(&self) -> usize {
        self.inner.size()
    }

    /// Check whether a node exists (supports ``"A" in graph``).
    fn __contains__(&self, node: &str) -> bool {
        self.inner.has_node(node)
    }

    /// Return a human-readable representation of the graph.
    fn __repr__(&self) -> String {
        format!("DirectedGraph(nodes={}, edges={})", self.inner.size(), self.inner.edges().len())
    }

    // -- Algorithms --------------------------------------------------------

    /// Return a topological ordering of the graph (Kahn's algorithm).
    ///
    /// Every node appears after all of its dependencies. This gives a valid
    /// build order for the dependency graph.
    ///
    /// Raises:
    ///     CycleError: If the graph contains a cycle.
    fn topological_sort(&self) -> PyResult<Vec<String>> {
        self.inner.topological_sort().map_err(to_py_err)
    }

    /// Check whether the graph contains a cycle (DFS with 3-color marking).
    fn has_cycle(&self) -> bool {
        self.inner.has_cycle()
    }

    /// Return the set of all nodes reachable from ``node`` (transitive closure).
    ///
    /// Does NOT include the node itself.
    ///
    /// Raises:
    ///     NodeNotFoundError: If the node does not exist.
    fn transitive_closure(&self, node: &str) -> PyResult<HashSet<String>> {
        self.inner.transitive_closure(node).map_err(to_py_err)
    }

    /// Given a set of changed nodes, return all nodes that are transitively
    /// affected (the changed nodes plus everything that depends on them).
    ///
    /// This is the core of incremental build detection: "if these packages
    /// changed, what else needs rebuilding?"
    fn affected_nodes(&self, changed: HashSet<String>) -> HashSet<String> {
        self.inner.affected_nodes(&changed)
    }

    /// Partition the graph into independent groups (parallel execution levels).
    ///
    /// Level 0 contains nodes with no dependencies. Level 1 depends only on
    /// level 0. And so on. Nodes within the same level can be executed in
    /// parallel because they have no dependencies between them.
    ///
    /// Raises:
    ///     CycleError: If the graph contains a cycle.
    fn independent_groups(&self) -> PyResult<Vec<Vec<String>>> {
        self.inner.independent_groups().map_err(to_py_err)
    }
}

// ---------------------------------------------------------------------------
// Module registration
// ---------------------------------------------------------------------------

/// The Python module. This is the entry point that Python calls when you do
/// ``import directed_graph_native``.
#[pymodule]
fn directed_graph_native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyDirectedGraph>()?;
    m.add("CycleError", m.py().get_type::<CycleError>())?;
    m.add("NodeNotFoundError", m.py().get_type::<NodeNotFoundError>())?;
    m.add("EdgeNotFoundError", m.py().get_type::<EdgeNotFoundError>())?;
    Ok(())
}
