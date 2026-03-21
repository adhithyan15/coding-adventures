// lib.rs -- Magnus wrapper for the Rust directed-graph crate
// ===========================================================
//
// This is a thin wrapper that exposes the Rust `directed_graph::Graph` type
// to Ruby via Magnus. The wrapper handles type conversion at the boundary:
//
// - Rust `String` <-> Ruby `String`
// - Rust `Vec<String>` <-> Ruby `Array` of `String`
// - Rust `Vec<Vec<String>>` <-> Ruby `Array` of `Array` of `String`
// - Rust `HashSet<String>` <-> Ruby `Array` of `String` (input/output)
// - Rust `GraphError` -> Ruby exceptions
//
// The core algorithms (topological sort, cycle detection, transitive closure,
// independent groups, affected nodes) all live in the Rust crate. This file
// contains zero algorithm logic -- it's pure glue code.
//
// # Why a native extension?
//
// The pure Ruby `directed_graph` package works fine, but for large graphs
// (thousands of nodes) the Rust implementation is significantly faster because:
//
// 1. No GC pressure -- Rust manages memory directly.
// 2. Cache-friendly data structures -- BTreeSet for sorted iteration.
// 3. Zero-cost abstractions -- iterators compile to tight loops.
//
// # Architecture comparison: PyO3 vs Magnus
//
// PyO3 (Python) uses `#[pymethods]` on an impl block with `&mut self`.
// Magnus (Ruby) uses `#[magnus::wrap]` on a struct and defines methods via
// `RefCell` for interior mutability, since Ruby's object model doesn't have
// Rust's ownership semantics. We borrow the inner `Graph` mutably only
// when needed, and immutably for read-only operations.
//
// # Usage from Ruby
//
// ```ruby
// require "coding_adventures_directed_graph_native"
//
// g = CodingAdventures::DirectedGraphNative::DirectedGraph.new
// g.add_edge("A", "B")
// g.add_edge("B", "C")
//
// g.topological_sort    # => ["A", "B", "C"]
// g.independent_groups  # => [["A"], ["B"], ["C"]]
// ```

use std::cell::RefCell;
use std::collections::HashSet;

use directed_graph::graph::{Graph, GraphError};
use magnus::{
    define_module, exception, function, method,
    prelude::*, Error, ExceptionClass, Ruby, Value,
};

// ---------------------------------------------------------------------------
// Error conversion
// ---------------------------------------------------------------------------
//
// Magnus uses `magnus::Error` for all Ruby exceptions. We define helper
// functions to create errors with the appropriate Ruby exception class.
//
// Unlike PyO3 where you define exception types with macros, Magnus requires
// us to look up or define exception classes at init time and store them.
// For simplicity, we look up the classes each time we need them via the
// module hierarchy. This is slightly less efficient than caching, but it's
// only called on error paths, so it doesn't matter.

/// Convert a Rust `GraphError` into a `magnus::Error` with the appropriate
/// Ruby exception class under `CodingAdventures::DirectedGraphNative`.
fn to_rb_err(ruby: &Ruby, err: GraphError) -> Error {
    // Look up the module and exception classes. If these fail, we fall back
    // to RuntimeError (which should never happen if init ran correctly).
    let module = ruby
        .class_object()
        .funcall::<_, _, Value>("const_get", ("CodingAdventures",))
        .and_then(|m| m.funcall::<_, _, Value>("const_get", ("DirectedGraphNative",)));

    match err {
        GraphError::CycleError => {
            let cls = module
                .and_then(|m| m.funcall::<_, _, Value>("const_get", ("CycleError",)))
                .ok();
            if let Some(c) = cls {
                // Safety: we know CycleError is an exception class because we defined it.
                let exc_class = unsafe { ExceptionClass::from_value_unchecked(c) };
                Error::new(exc_class, "graph contains a cycle")
            } else {
                Error::new(exception::runtime_error(), "graph contains a cycle")
            }
        }
        GraphError::NodeNotFound(node) => {
            let msg = format!("node not found: {}", node);
            let cls = module
                .and_then(|m| m.funcall::<_, _, Value>("const_get", ("NodeNotFoundError",)))
                .ok();
            if let Some(c) = cls {
                let exc_class = unsafe { ExceptionClass::from_value_unchecked(c) };
                Error::new(exc_class, msg)
            } else {
                Error::new(exception::runtime_error(), msg)
            }
        }
        GraphError::EdgeNotFound(from, to) => {
            let msg = format!("edge not found: {} -> {}", from, to);
            let cls = module
                .and_then(|m| m.funcall::<_, _, Value>("const_get", ("EdgeNotFoundError",)))
                .ok();
            if let Some(c) = cls {
                let exc_class = unsafe { ExceptionClass::from_value_unchecked(c) };
                Error::new(exc_class, msg)
            } else {
                Error::new(exception::runtime_error(), msg)
            }
        }
        GraphError::SelfLoop(node) => {
            let msg = format!("self-loop not allowed: {}", node);
            Error::new(exception::arg_error(), msg)
        }
    }
}

// ---------------------------------------------------------------------------
// RbDirectedGraph -- the main Ruby class
// ---------------------------------------------------------------------------
//
// We wrap the Rust `Graph` in a `RefCell` to provide interior mutability.
// Ruby's object model allows any method to mutate the receiver, but Rust's
// borrow checker requires us to be explicit about mutability. `RefCell`
// gives us runtime borrow checking: we call `.borrow()` for read-only
// access and `.borrow_mut()` for mutable access.
//
// This is safe because Ruby is single-threaded (GVL), so we'll never have
// concurrent borrows. If someone somehow manages to re-enter a method
// while a borrow is active, `RefCell` will panic -- which is better than
// undefined behavior.

/// A directed graph backed by Rust's directed-graph crate.
///
/// This class provides the same API as the pure Ruby `DirectedGraph::Graph`,
/// but the algorithms run in Rust for better performance on large graphs.
#[magnus::wrap(class = "CodingAdventures::DirectedGraphNative::DirectedGraph")]
struct RbDirectedGraph {
    inner: RefCell<Graph>,
}

impl RbDirectedGraph {
    // -- Constructor -------------------------------------------------------

    /// Create a new empty directed graph.
    fn new() -> Self {
        RbDirectedGraph {
            inner: RefCell::new(Graph::new()),
        }
    }

    // -- Node operations ---------------------------------------------------

    /// Add a node to the graph. If the node already exists, this is a no-op.
    fn add_node(&self, node: String) {
        self.inner.borrow_mut().add_node(&node);
    }

    /// Remove a node and all its edges from the graph.
    ///
    /// Raises `NodeNotFoundError` if the node does not exist.
    fn remove_node(&self, ruby: &Ruby, node: String) -> Result<(), Error> {
        self.inner
            .borrow_mut()
            .remove_node(&node)
            .map_err(|e| to_rb_err(ruby, e))
    }

    /// Check whether a node exists in the graph.
    fn has_node(&self, node: String) -> bool {
        self.inner.borrow().has_node(&node)
    }

    /// Return a sorted list of all nodes in the graph.
    fn nodes(&self) -> Vec<String> {
        self.inner.borrow().nodes()
    }

    // -- Edge operations ---------------------------------------------------

    /// Add a directed edge from `from_node` to `to_node`.
    ///
    /// Both nodes are created if they don't exist yet.
    ///
    /// Raises `ArgumentError` if `from_node == to_node` (self-loops not allowed).
    fn add_edge(&self, ruby: &Ruby, from_node: String, to_node: String) -> Result<(), Error> {
        self.inner
            .borrow_mut()
            .add_edge(&from_node, &to_node)
            .map_err(|e| to_rb_err(ruby, e))
    }

    /// Remove a directed edge from `from_node` to `to_node`.
    ///
    /// Raises `EdgeNotFoundError` if the edge does not exist.
    fn remove_edge(&self, ruby: &Ruby, from_node: String, to_node: String) -> Result<(), Error> {
        self.inner
            .borrow_mut()
            .remove_edge(&from_node, &to_node)
            .map_err(|e| to_rb_err(ruby, e))
    }

    /// Check whether a directed edge exists from `from_node` to `to_node`.
    fn has_edge(&self, from_node: String, to_node: String) -> bool {
        self.inner.borrow().has_edge(&from_node, &to_node)
    }

    /// Return a list of all edges as `[from, to]` arrays, sorted.
    fn edges(&self) -> Vec<(String, String)> {
        self.inner.borrow().edges()
    }

    // -- Neighbor queries --------------------------------------------------

    /// Return the predecessors (parents) of a node -- nodes that point TO it.
    ///
    /// Raises `NodeNotFoundError` if the node does not exist.
    fn predecessors(&self, ruby: &Ruby, node: String) -> Result<Vec<String>, Error> {
        self.inner
            .borrow()
            .predecessors(&node)
            .map_err(|e| to_rb_err(ruby, e))
    }

    /// Return the successors (children) of a node -- nodes it points TO.
    ///
    /// Raises `NodeNotFoundError` if the node does not exist.
    fn successors(&self, ruby: &Ruby, node: String) -> Result<Vec<String>, Error> {
        self.inner
            .borrow()
            .successors(&node)
            .map_err(|e| to_rb_err(ruby, e))
    }

    // -- Graph properties --------------------------------------------------

    /// Return the number of nodes in the graph.
    fn size(&self) -> usize {
        self.inner.borrow().size()
    }

    /// Return a human-readable representation of the graph.
    fn inspect(&self) -> String {
        let g = self.inner.borrow();
        format!(
            "#<CodingAdventures::DirectedGraphNative::DirectedGraph nodes={} edges={}>",
            g.size(),
            g.edges().len()
        )
    }

    /// Return a string representation (same as inspect for debugging).
    fn to_s(&self) -> String {
        self.inspect()
    }

    // -- Algorithms --------------------------------------------------------

    /// Return a topological ordering of the graph (Kahn's algorithm).
    ///
    /// Every node appears after all of its dependencies. This gives a valid
    /// build order for the dependency graph.
    ///
    /// Raises `CycleError` if the graph contains a cycle.
    fn topological_sort(&self, ruby: &Ruby) -> Result<Vec<String>, Error> {
        self.inner
            .borrow()
            .topological_sort()
            .map_err(|e| to_rb_err(ruby, e))
    }

    /// Check whether the graph contains a cycle (DFS with 3-color marking).
    fn has_cycle(&self) -> bool {
        self.inner.borrow().has_cycle()
    }

    /// Return the set of all nodes reachable from `node` (transitive closure).
    ///
    /// Does NOT include the node itself. Returns an Array of strings.
    ///
    /// Raises `NodeNotFoundError` if the node does not exist.
    fn transitive_closure(&self, ruby: &Ruby, node: String) -> Result<Vec<String>, Error> {
        self.inner
            .borrow()
            .transitive_closure(&node)
            .map(|set| {
                let mut v: Vec<String> = set.into_iter().collect();
                v.sort();
                v
            })
            .map_err(|e| to_rb_err(ruby, e))
    }

    /// Given an array of changed nodes, return all nodes that are transitively
    /// affected (the changed nodes plus everything that depends on them).
    ///
    /// This is the core of incremental build detection: "if these packages
    /// changed, what else needs rebuilding?"
    ///
    /// Returns an Array of strings, sorted.
    fn affected_nodes(&self, changed: Vec<String>) -> Vec<String> {
        let changed_set: HashSet<String> = changed.into_iter().collect();
        let result = self.inner.borrow().affected_nodes(&changed_set);
        let mut v: Vec<String> = result.into_iter().collect();
        v.sort();
        v
    }

    /// Partition the graph into independent groups (parallel execution levels).
    ///
    /// Level 0 contains nodes with no dependencies. Level 1 depends only on
    /// level 0. And so on. Nodes within the same level can be executed in
    /// parallel because they have no dependencies between them.
    ///
    /// Raises `CycleError` if the graph contains a cycle.
    fn independent_groups(&self, ruby: &Ruby) -> Result<Vec<Vec<String>>, Error> {
        self.inner
            .borrow()
            .independent_groups()
            .map_err(|e| to_rb_err(ruby, e))
    }
}

// ---------------------------------------------------------------------------
// Module registration
// ---------------------------------------------------------------------------
//
// The `#[magnus::init]` function is called when Ruby loads the shared library.
// It's analogous to PyO3's `#[pymodule]` function.
//
// We set up:
// 1. The module hierarchy: CodingAdventures::DirectedGraphNative
// 2. The DirectedGraph class with all its methods
// 3. Custom exception classes (CycleError, NodeNotFoundError, EdgeNotFoundError)

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    // -----------------------------------------------------------------------
    // Module hierarchy
    // -----------------------------------------------------------------------
    //
    // We nest under CodingAdventures to match the pure Ruby gem's namespace.
    // The full class path is:
    //   CodingAdventures::DirectedGraphNative::DirectedGraph
    //
    // This lets users choose between the pure Ruby and native implementations:
    //   CodingAdventures::DirectedGraph::Graph       (pure Ruby)
    //   CodingAdventures::DirectedGraphNative::DirectedGraph  (Rust-backed)

    let coding_adventures = define_module(ruby, "CodingAdventures")?;
    let module = coding_adventures.define_module("DirectedGraphNative")?;

    // -----------------------------------------------------------------------
    // DirectedGraph class
    // -----------------------------------------------------------------------
    //
    // We define the class inside the module and register all methods.
    // Magnus uses `define_method` for instance methods.

    let klass = module.define_class("DirectedGraph", ruby.class_object())?;

    // Constructor
    klass.define_singleton_method("new", function!(RbDirectedGraph::new, 0))?;

    // Node operations
    klass.define_method("add_node", method!(RbDirectedGraph::add_node, 1))?;
    klass.define_method("remove_node", method!(RbDirectedGraph::remove_node, 1))?;
    klass.define_method("has_node?", method!(RbDirectedGraph::has_node, 1))?;
    klass.define_method("nodes", method!(RbDirectedGraph::nodes, 0))?;

    // Edge operations
    klass.define_method("add_edge", method!(RbDirectedGraph::add_edge, 2))?;
    klass.define_method("remove_edge", method!(RbDirectedGraph::remove_edge, 2))?;
    klass.define_method("has_edge?", method!(RbDirectedGraph::has_edge, 2))?;
    klass.define_method("edges", method!(RbDirectedGraph::edges, 0))?;

    // Neighbor queries
    klass.define_method("predecessors", method!(RbDirectedGraph::predecessors, 1))?;
    klass.define_method("successors", method!(RbDirectedGraph::successors, 1))?;

    // Graph properties
    klass.define_method("size", method!(RbDirectedGraph::size, 0))?;
    klass.define_method("inspect", method!(RbDirectedGraph::inspect, 0))?;
    klass.define_method("to_s", method!(RbDirectedGraph::to_s, 0))?;

    // Algorithms
    klass.define_method("topological_sort", method!(RbDirectedGraph::topological_sort, 0))?;
    klass.define_method("has_cycle?", method!(RbDirectedGraph::has_cycle, 0))?;
    klass.define_method("transitive_closure", method!(RbDirectedGraph::transitive_closure, 1))?;
    klass.define_method("affected_nodes", method!(RbDirectedGraph::affected_nodes, 1))?;
    klass.define_method("independent_groups", method!(RbDirectedGraph::independent_groups, 0))?;

    // -----------------------------------------------------------------------
    // Exception classes
    // -----------------------------------------------------------------------
    //
    // We define custom Ruby exception classes that mirror the Rust error
    // variants. These allow Ruby users to rescue specific errors:
    //
    //     begin
    //       g.topological_sort
    //     rescue CodingAdventures::DirectedGraphNative::CycleError
    //       puts "graph has a cycle!"
    //     end

    module.define_class("CycleError", ruby.exception_standard_error())?;
    module.define_class("NodeNotFoundError", ruby.exception_standard_error())?;
    module.define_class("EdgeNotFoundError", ruby.exception_standard_error())?;

    Ok(())
}
