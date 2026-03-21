// lib.rs -- Magnus 0.7 wrapper for the Rust directed-graph crate
// ================================================================
//
// Thin glue between Ruby and the Rust directed-graph library.
// All algorithms run in Rust; this file handles type conversion only.
//
// Magnus 0.7 API notes:
// - method!() expects methods that DON'T take &Ruby as a parameter
// - define_module() is a free function (no ruby parameter)
// - Error conversion uses exception::runtime_error() and friends
// - ExceptionClass::from_value() returns Option (safe, no unchecked)

use std::cell::RefCell;
use std::collections::HashSet;

use directed_graph::graph::{Graph, GraphError};
use magnus::{define_module, exception, function, method, prelude::*, Error};

// ---------------------------------------------------------------------------
// Error conversion
// ---------------------------------------------------------------------------

fn to_rb_err(err: GraphError) -> Error {
    match err {
        GraphError::CycleError => {
            Error::new(exception::runtime_error(), "graph contains a cycle")
        }
        GraphError::NodeNotFound(node) => {
            Error::new(exception::runtime_error(), format!("node not found: {}", node))
        }
        GraphError::EdgeNotFound(from, to) => {
            Error::new(exception::runtime_error(), format!("edge not found: {} -> {}", from, to))
        }
        GraphError::SelfLoop(node) => {
            Error::new(exception::arg_error(), format!("self-loop not allowed: {}", node))
        }
    }
}

// ---------------------------------------------------------------------------
// RbDirectedGraph
// ---------------------------------------------------------------------------

#[magnus::wrap(class = "CodingAdventures::DirectedGraphNative::DirectedGraph")]
struct RbDirectedGraph {
    inner: RefCell<Graph>,
}

impl RbDirectedGraph {
    fn new() -> Self {
        RbDirectedGraph {
            inner: RefCell::new(Graph::new()),
        }
    }

    fn add_node(&self, node: String) {
        self.inner.borrow_mut().add_node(&node);
    }

    fn remove_node(&self, node: String) -> Result<(), Error> {
        self.inner.borrow_mut().remove_node(&node).map_err(to_rb_err)
    }

    fn has_node(&self, node: String) -> bool {
        self.inner.borrow().has_node(&node)
    }

    fn nodes(&self) -> Vec<String> {
        self.inner.borrow().nodes()
    }

    fn add_edge(&self, from_node: String, to_node: String) -> Result<(), Error> {
        self.inner.borrow_mut().add_edge(&from_node, &to_node).map_err(to_rb_err)
    }

    fn remove_edge(&self, from_node: String, to_node: String) -> Result<(), Error> {
        self.inner.borrow_mut().remove_edge(&from_node, &to_node).map_err(to_rb_err)
    }

    fn has_edge(&self, from_node: String, to_node: String) -> bool {
        self.inner.borrow().has_edge(&from_node, &to_node)
    }

    fn edges(&self) -> Vec<(String, String)> {
        self.inner.borrow().edges()
    }

    fn predecessors(&self, node: String) -> Result<Vec<String>, Error> {
        self.inner.borrow().predecessors(&node).map_err(to_rb_err)
    }

    fn successors(&self, node: String) -> Result<Vec<String>, Error> {
        self.inner.borrow().successors(&node).map_err(to_rb_err)
    }

    fn size(&self) -> usize {
        self.inner.borrow().size()
    }

    fn inspect(&self) -> String {
        let g = self.inner.borrow();
        format!(
            "#<DirectedGraph nodes={} edges={}>",
            g.size(),
            g.edges().len()
        )
    }

    fn to_s(&self) -> String {
        self.inspect()
    }

    fn topological_sort(&self) -> Result<Vec<String>, Error> {
        self.inner.borrow().topological_sort().map_err(to_rb_err)
    }

    fn has_cycle(&self) -> bool {
        self.inner.borrow().has_cycle()
    }

    fn transitive_closure(&self, node: String) -> Result<Vec<String>, Error> {
        self.inner
            .borrow()
            .transitive_closure(&node)
            .map(|set| {
                let mut v: Vec<String> = set.into_iter().collect();
                v.sort();
                v
            })
            .map_err(to_rb_err)
    }

    fn affected_nodes(&self, changed: Vec<String>) -> Vec<String> {
        let changed_set: HashSet<String> = changed.into_iter().collect();
        let result = self.inner.borrow().affected_nodes(&changed_set);
        let mut v: Vec<String> = result.into_iter().collect();
        v.sort();
        v
    }

    fn independent_groups(&self) -> Result<Vec<Vec<String>>, Error> {
        self.inner.borrow().independent_groups().map_err(to_rb_err)
    }
}

// ---------------------------------------------------------------------------
// Module init
// ---------------------------------------------------------------------------

#[magnus::init]
fn init_directed_graph_native() -> Result<(), Error> {
    let coding_adventures = define_module("CodingAdventures")?;
    let module = coding_adventures.define_module("DirectedGraphNative")?;

    let klass = module.define_class("DirectedGraph", magnus::class::object())?;

    klass.define_singleton_method("new", function!(RbDirectedGraph::new, 0))?;

    klass.define_method("add_node", method!(RbDirectedGraph::add_node, 1))?;
    klass.define_method("remove_node", method!(RbDirectedGraph::remove_node, 1))?;
    klass.define_method("has_node?", method!(RbDirectedGraph::has_node, 1))?;
    klass.define_method("nodes", method!(RbDirectedGraph::nodes, 0))?;

    klass.define_method("add_edge", method!(RbDirectedGraph::add_edge, 2))?;
    klass.define_method("remove_edge", method!(RbDirectedGraph::remove_edge, 2))?;
    klass.define_method("has_edge?", method!(RbDirectedGraph::has_edge, 2))?;
    klass.define_method("edges", method!(RbDirectedGraph::edges, 0))?;

    klass.define_method("predecessors", method!(RbDirectedGraph::predecessors, 1))?;
    klass.define_method("successors", method!(RbDirectedGraph::successors, 1))?;

    klass.define_method("size", method!(RbDirectedGraph::size, 0))?;
    klass.define_method("inspect", method!(RbDirectedGraph::inspect, 0))?;
    klass.define_method("to_s", method!(RbDirectedGraph::to_s, 0))?;

    klass.define_method("topological_sort", method!(RbDirectedGraph::topological_sort, 0))?;
    klass.define_method("has_cycle?", method!(RbDirectedGraph::has_cycle, 0))?;
    klass.define_method("transitive_closure", method!(RbDirectedGraph::transitive_closure, 1))?;
    klass.define_method("affected_nodes", method!(RbDirectedGraph::affected_nodes, 1))?;
    klass.define_method("independent_groups", method!(RbDirectedGraph::independent_groups, 0))?;

    Ok(())
}
