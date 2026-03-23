// lib.rs -- DirectedGraph Ruby native extension using ruby-bridge
// ================================================================
//
// This is a Ruby C extension written in Rust. It wraps the `directed-graph`
// crate's `Graph` struct and exposes it to Ruby as:
//
//   CodingAdventures::DirectedGraphNative::Graph
//
// # Architecture
//
// 1. `Init_directed_graph_native()` is called by Ruby when the .so is loaded
// 2. We define the module hierarchy and a `Graph` class under it
// 3. Each Graph instance wraps a Rust `directed_graph::Graph` via `wrap_data`
// 4. Methods extract the Graph pointer via `unwrap_data`, call Rust, marshal results
//
// # The ruby-bridge approach
//
// Instead of Magnus or rb-sys, we use our own `ruby-bridge` crate that
// declares Ruby's C API functions via `extern "C"`. This gives us:
// - Zero dependencies beyond libruby (linked at load time)
// - Complete visibility into every C API call
// - No build-time header requirements
//
// # Method signatures
//
// Ruby's `rb_define_method` uses a C function pointer + argc count:
// - argc=0: `extern "C" fn(self_val: VALUE) -> VALUE`
// - argc=1: `extern "C" fn(self_val: VALUE, arg: VALUE) -> VALUE`
// - argc=2: `extern "C" fn(self_val: VALUE, arg1: VALUE, arg2: VALUE) -> VALUE`
//
// The `self_val` is always the Ruby receiver (the Graph object).

use std::collections::HashSet;
use std::ffi::c_void;

use directed_graph::graph::{Graph, GraphError};
use ruby_bridge::VALUE;

// ---------------------------------------------------------------------------
// Global: the Graph class VALUE
// ---------------------------------------------------------------------------
//
// We need to remember the Ruby class so that the alloc function can create
// instances. This is set once during Init and never changes.
static mut GRAPH_CLASS: VALUE = 0;

// ---------------------------------------------------------------------------
// Alloc function — called by Ruby before `initialize`
// ---------------------------------------------------------------------------
//
// Ruby's object creation follows a two-step pattern:
//   1. `allocate` creates the raw object (this function)
//   2. `initialize` fills it in (our `graph_initialize`)
//
// We wrap a fresh, empty Graph here. The `initialize` method is a no-op
// since Graph::new() already gives us a usable empty graph.

unsafe extern "C" fn graph_alloc(klass: VALUE) -> VALUE {
    ruby_bridge::wrap_data(klass, Graph::new())
}

// ---------------------------------------------------------------------------
// initialize — Ruby constructor (argc=0, no-op since alloc creates the Graph)
// ---------------------------------------------------------------------------

extern "C" fn graph_initialize(self_val: VALUE) -> VALUE {
    // The alloc function already created a Graph::new(). Nothing to do.
    self_val
}

// ---------------------------------------------------------------------------
// Graph access helpers
// ---------------------------------------------------------------------------
//
// Every method needs to extract the Rust Graph from the Ruby VALUE.
// We use `unwrap_data` which reads the data pointer from Ruby's RData struct.

unsafe fn get_graph(self_val: VALUE) -> &'static Graph {
    ruby_bridge::unwrap_data::<Graph>(self_val)
}

unsafe fn get_graph_mut(self_val: VALUE) -> &'static mut Graph {
    ruby_bridge::unwrap_data_mut::<Graph>(self_val)
}

// ---------------------------------------------------------------------------
// Error handling helper
// ---------------------------------------------------------------------------
//
// Translates Rust GraphError variants into Ruby exceptions.
// Each variant maps to a different exception type:
// - SelfLoop -> ArgumentError (invalid input)
// - NodeNotFound, EdgeNotFound, CycleError -> RuntimeError

fn raise_graph_error(err: GraphError) -> ! {
    match err {
        GraphError::SelfLoop(node) => {
            ruby_bridge::raise_arg_error(&format!("self-loop not allowed: {}", node))
        }
        GraphError::NodeNotFound(node) => {
            ruby_bridge::raise_runtime_error(&format!("node not found: {}", node))
        }
        GraphError::EdgeNotFound(from, to) => {
            ruby_bridge::raise_runtime_error(&format!("edge not found: {} -> {}", from, to))
        }
        GraphError::CycleError => {
            ruby_bridge::raise_runtime_error("graph contains a cycle")
        }
    }
}

// ---------------------------------------------------------------------------
// Method implementations
// ---------------------------------------------------------------------------
//
// Each method follows the same pattern:
// 1. Extract the Graph from self_val
// 2. Convert Ruby arguments to Rust types
// 3. Call the Rust method on Graph
// 4. Convert the Rust result back to a Ruby VALUE

// -- add_node(name) -> nil ------------------------------------------------
//
// Adds a node to the graph. Idempotent — adding an existing node is a no-op.
extern "C" fn graph_add_node(self_val: VALUE, name: VALUE) -> VALUE {
    let node = match ruby_bridge::str_from_rb(name) {
        Some(s) => s,
        None => ruby_bridge::raise_arg_error("add_node: argument must be a String"),
    };
    unsafe { get_graph_mut(self_val).add_node(&node) };
    ruby_bridge::QNIL
}

// -- remove_node(name) -> nil ---------------------------------------------
//
// Removes a node and all its edges. Raises RuntimeError if node not found.
extern "C" fn graph_remove_node(self_val: VALUE, name: VALUE) -> VALUE {
    let node = match ruby_bridge::str_from_rb(name) {
        Some(s) => s,
        None => ruby_bridge::raise_arg_error("remove_node: argument must be a String"),
    };
    match unsafe { get_graph_mut(self_val).remove_node(&node) } {
        Ok(()) => ruby_bridge::QNIL,
        Err(e) => raise_graph_error(e),
    }
}

// -- has_node?(name) -> true/false ----------------------------------------
extern "C" fn graph_has_node(self_val: VALUE, name: VALUE) -> VALUE {
    let node = match ruby_bridge::str_from_rb(name) {
        Some(s) => s,
        None => ruby_bridge::raise_arg_error("has_node?: argument must be a String"),
    };
    ruby_bridge::bool_to_rb(unsafe { get_graph(self_val).has_node(&node) })
}

// -- nodes -> Array<String> -----------------------------------------------
//
// Returns a sorted array of all node names.
extern "C" fn graph_nodes(self_val: VALUE) -> VALUE {
    let nodes = unsafe { get_graph(self_val).nodes() };
    ruby_bridge::vec_str_to_rb(&nodes)
}

// -- size -> Integer ------------------------------------------------------
extern "C" fn graph_size(self_val: VALUE) -> VALUE {
    ruby_bridge::usize_to_rb(unsafe { get_graph(self_val).size() })
}

// -- add_edge(from, to) -> nil --------------------------------------------
//
// Adds a directed edge. Both nodes are auto-created if missing.
// Raises ArgumentError on self-loop.
extern "C" fn graph_add_edge(self_val: VALUE, from: VALUE, to: VALUE) -> VALUE {
    let from_str = match ruby_bridge::str_from_rb(from) {
        Some(s) => s,
        None => ruby_bridge::raise_arg_error("add_edge: first argument must be a String"),
    };
    let to_str = match ruby_bridge::str_from_rb(to) {
        Some(s) => s,
        None => ruby_bridge::raise_arg_error("add_edge: second argument must be a String"),
    };
    match unsafe { get_graph_mut(self_val).add_edge(&from_str, &to_str) } {
        Ok(()) => ruby_bridge::QNIL,
        Err(e) => raise_graph_error(e),
    }
}

// -- remove_edge(from, to) -> nil -----------------------------------------
//
// Removes an edge. Raises RuntimeError if nodes or edge not found.
extern "C" fn graph_remove_edge(self_val: VALUE, from: VALUE, to: VALUE) -> VALUE {
    let from_str = match ruby_bridge::str_from_rb(from) {
        Some(s) => s,
        None => ruby_bridge::raise_arg_error("remove_edge: first argument must be a String"),
    };
    let to_str = match ruby_bridge::str_from_rb(to) {
        Some(s) => s,
        None => ruby_bridge::raise_arg_error("remove_edge: second argument must be a String"),
    };
    match unsafe { get_graph_mut(self_val).remove_edge(&from_str, &to_str) } {
        Ok(()) => ruby_bridge::QNIL,
        Err(e) => raise_graph_error(e),
    }
}

// -- has_edge?(from, to) -> true/false ------------------------------------
extern "C" fn graph_has_edge(self_val: VALUE, from: VALUE, to: VALUE) -> VALUE {
    let from_str = match ruby_bridge::str_from_rb(from) {
        Some(s) => s,
        None => ruby_bridge::raise_arg_error("has_edge?: first argument must be a String"),
    };
    let to_str = match ruby_bridge::str_from_rb(to) {
        Some(s) => s,
        None => ruby_bridge::raise_arg_error("has_edge?: second argument must be a String"),
    };
    ruby_bridge::bool_to_rb(unsafe { get_graph(self_val).has_edge(&from_str, &to_str) })
}

// -- edges -> Array<[from, to]> -------------------------------------------
//
// Returns all edges as an array of two-element arrays.
extern "C" fn graph_edges(self_val: VALUE) -> VALUE {
    let edges = unsafe { get_graph(self_val).edges() };
    ruby_bridge::vec_tuple2_str_to_rb(&edges)
}

// -- predecessors(node) -> Array<String> ----------------------------------
//
// Returns sorted predecessors of a node. Raises RuntimeError if not found.
extern "C" fn graph_predecessors(self_val: VALUE, name: VALUE) -> VALUE {
    let node = match ruby_bridge::str_from_rb(name) {
        Some(s) => s,
        None => ruby_bridge::raise_arg_error("predecessors: argument must be a String"),
    };
    match unsafe { get_graph(self_val).predecessors(&node) } {
        Ok(v) => ruby_bridge::vec_str_to_rb(&v),
        Err(e) => raise_graph_error(e),
    }
}

// -- successors(node) -> Array<String> ------------------------------------
//
// Returns sorted successors of a node. Raises RuntimeError if not found.
extern "C" fn graph_successors(self_val: VALUE, name: VALUE) -> VALUE {
    let node = match ruby_bridge::str_from_rb(name) {
        Some(s) => s,
        None => ruby_bridge::raise_arg_error("successors: argument must be a String"),
    };
    match unsafe { get_graph(self_val).successors(&node) } {
        Ok(v) => ruby_bridge::vec_str_to_rb(&v),
        Err(e) => raise_graph_error(e),
    }
}

// -- topological_sort -> Array<String> ------------------------------------
//
// Returns nodes in topological order. Raises RuntimeError if cycle exists.
extern "C" fn graph_topological_sort(self_val: VALUE) -> VALUE {
    match unsafe { get_graph(self_val).topological_sort() } {
        Ok(v) => ruby_bridge::vec_str_to_rb(&v),
        Err(e) => raise_graph_error(e),
    }
}

// -- has_cycle? -> true/false ---------------------------------------------
extern "C" fn graph_has_cycle(self_val: VALUE) -> VALUE {
    ruby_bridge::bool_to_rb(unsafe { get_graph(self_val).has_cycle() })
}

// -- transitive_closure(node) -> Array<String> ----------------------------
//
// Returns all nodes reachable from the given node (sorted).
// Raises RuntimeError if node not found.
extern "C" fn graph_transitive_closure(self_val: VALUE, name: VALUE) -> VALUE {
    let node = match ruby_bridge::str_from_rb(name) {
        Some(s) => s,
        None => ruby_bridge::raise_arg_error("transitive_closure: argument must be a String"),
    };
    match unsafe { get_graph(self_val).transitive_closure(&node) } {
        Ok(set) => {
            // Convert HashSet to sorted Vec for deterministic output
            let mut sorted: Vec<String> = set.into_iter().collect();
            sorted.sort();
            ruby_bridge::vec_str_to_rb(&sorted)
        }
        Err(e) => raise_graph_error(e),
    }
}

// -- affected_nodes(changed) -> Array<String> -----------------------------
//
// Given a Ruby Array of changed node names, returns all affected nodes
// (the changed nodes plus their transitive dependents), sorted.
extern "C" fn graph_affected_nodes(self_val: VALUE, changed_ary: VALUE) -> VALUE {
    // Convert Ruby Array<String> to HashSet<String>
    let changed_vec = ruby_bridge::vec_str_from_rb(changed_ary);
    let changed: HashSet<String> = changed_vec.into_iter().collect();

    let result = unsafe { get_graph(self_val).affected_nodes(&changed) };
    // Convert HashSet to sorted Vec for deterministic output
    let mut sorted: Vec<String> = result.into_iter().collect();
    sorted.sort();
    ruby_bridge::vec_str_to_rb(&sorted)
}

// -- independent_groups -> Array<Array<String>> ---------------------------
//
// Partitions nodes into layers for parallel execution.
// Raises RuntimeError if cycle exists.
extern "C" fn graph_independent_groups(self_val: VALUE) -> VALUE {
    match unsafe { get_graph(self_val).independent_groups() } {
        Ok(groups) => ruby_bridge::vec_vec_str_to_rb(&groups),
        Err(e) => raise_graph_error(e),
    }
}

// ---------------------------------------------------------------------------
// Init_directed_graph_native — Ruby extension entry point
// ---------------------------------------------------------------------------
//
// This function MUST be named `Init_directed_graph_native` because Ruby
// derives the init function name from the .so filename. When Ruby loads
// `directed_graph_native.so`, it calls `Init_directed_graph_native()`.
//
// We set up the module hierarchy and bind all methods here:
//
//   module CodingAdventures
//     module DirectedGraphNative
//       class Graph
//         # ... methods ...
//       end
//     end
//   end

#[no_mangle]
pub extern "C" fn Init_directed_graph_native() {
    // -- Module hierarchy ---------------------------------------------------
    let coding_adventures = ruby_bridge::define_module("CodingAdventures");
    let directed_graph_native =
        ruby_bridge::define_module_under(coding_adventures, "DirectedGraphNative");
    let graph_class = ruby_bridge::define_class_under(
        directed_graph_native,
        "Graph",
        ruby_bridge::object_class(),
    );

    // Store the class globally so the alloc function can access it
    unsafe { GRAPH_CLASS = graph_class };

    // -- Allocator ----------------------------------------------------------
    //
    // The alloc function creates a Graph::new() and wraps it in a Ruby object.
    // Ruby calls this before `initialize`.
    ruby_bridge::define_alloc_func(graph_class, graph_alloc);

    // -- Instance methods ---------------------------------------------------
    //
    // Each call to define_method_raw registers a C function as a Ruby method.
    // The argc parameter tells Ruby how many arguments the method expects.

    // initialize (argc=0) — no-op, alloc already created the Graph
    ruby_bridge::define_method_raw(
        graph_class,
        "initialize",
        graph_initialize as *const c_void,
        0,
    );

    // -- Node operations ----------------------------------------------------
    ruby_bridge::define_method_raw(
        graph_class,
        "add_node",
        graph_add_node as *const c_void,
        1,
    );
    ruby_bridge::define_method_raw(
        graph_class,
        "remove_node",
        graph_remove_node as *const c_void,
        1,
    );
    ruby_bridge::define_method_raw(
        graph_class,
        "has_node?",
        graph_has_node as *const c_void,
        1,
    );
    ruby_bridge::define_method_raw(
        graph_class,
        "nodes",
        graph_nodes as *const c_void,
        0,
    );
    ruby_bridge::define_method_raw(
        graph_class,
        "size",
        graph_size as *const c_void,
        0,
    );

    // -- Edge operations ----------------------------------------------------
    ruby_bridge::define_method_raw(
        graph_class,
        "add_edge",
        graph_add_edge as *const c_void,
        2,
    );
    ruby_bridge::define_method_raw(
        graph_class,
        "remove_edge",
        graph_remove_edge as *const c_void,
        2,
    );
    ruby_bridge::define_method_raw(
        graph_class,
        "has_edge?",
        graph_has_edge as *const c_void,
        2,
    );
    ruby_bridge::define_method_raw(
        graph_class,
        "edges",
        graph_edges as *const c_void,
        0,
    );

    // -- Neighbor queries ---------------------------------------------------
    ruby_bridge::define_method_raw(
        graph_class,
        "predecessors",
        graph_predecessors as *const c_void,
        1,
    );
    ruby_bridge::define_method_raw(
        graph_class,
        "successors",
        graph_successors as *const c_void,
        1,
    );

    // -- Algorithms ---------------------------------------------------------
    ruby_bridge::define_method_raw(
        graph_class,
        "topological_sort",
        graph_topological_sort as *const c_void,
        0,
    );
    ruby_bridge::define_method_raw(
        graph_class,
        "has_cycle?",
        graph_has_cycle as *const c_void,
        0,
    );
    ruby_bridge::define_method_raw(
        graph_class,
        "transitive_closure",
        graph_transitive_closure as *const c_void,
        1,
    );
    ruby_bridge::define_method_raw(
        graph_class,
        "affected_nodes",
        graph_affected_nodes as *const c_void,
        1,
    );
    ruby_bridge::define_method_raw(
        graph_class,
        "independent_groups",
        graph_independent_groups as *const c_void,
        0,
    );
}
