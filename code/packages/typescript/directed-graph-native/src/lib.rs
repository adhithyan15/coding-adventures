// lib.rs -- DirectedGraph Node.js native extension using node-bridge
// ==================================================================
//
// This crate exposes the Rust `directed_graph::Graph` to Node.js via N-API,
// using our zero-dependency `node-bridge` crate. No napi-rs, no napi-sys,
// no build-time header requirements -- just raw N-API calls through
// node-bridge's safe wrappers.
//
// # Architecture
//
// 1. `napi_register_module_v1()` is the entry point called by Node.js when
//    the addon is loaded. It defines a "DirectedGraph" class on the exports
//    object with all graph methods.
//
// 2. The constructor (`graph_new`) creates a Rust `Graph` and wraps it
//    inside the JS object using `node_bridge::wrap_data()`. N-API's wrap
//    mechanism stores the pointer and calls our destructor when the JS
//    object is garbage collected.
//
// 3. Each method callback extracts `this` and args via `get_cb_info()`,
//    unwraps the Graph pointer, calls the Rust graph method, marshals
//    the result back to a JS value, and returns it.
//
// 4. Errors from the graph (CycleError, NodeNotFound, etc.) are turned
//    into JS exceptions via `throw_error()` + returning `undefined()`.
//
// # Method naming
//
// All methods use camelCase to follow JavaScript conventions:
//   addNode, removeNode, hasNode, nodes, size,
//   addEdge, removeEdge, hasEdge, edges,
//   predecessors, successors,
//   topologicalSort, hasCycle, transitiveClosure,
//   affectedNodes, independentGroups

use std::collections::HashSet;

use directed_graph::graph::{Graph, GraphError};
use node_bridge::*;

// ---------------------------------------------------------------------------
// Helper: convert GraphError to a JS exception and return undefined
// ---------------------------------------------------------------------------
//
// When a graph operation fails, we throw a JS Error with a descriptive
// message and return `undefined` (which N-API interprets as "an exception
// is pending, don't use the return value").

fn throw_graph_error(env: napi_env, err: GraphError) -> napi_value {
    let msg = match err {
        GraphError::CycleError => "graph contains a cycle".to_string(),
        GraphError::NodeNotFound(node) => format!("node not found: {}", node),
        GraphError::EdgeNotFound(from, to) => format!("edge not found: {} -> {}", from, to),
        GraphError::SelfLoop(node) => format!("self-loop not allowed: {}", node),
    };
    throw_error(env, &msg);
    undefined(env)
}

// ---------------------------------------------------------------------------
// Helper: convert HashSet<String> to a JS array of strings
// ---------------------------------------------------------------------------
//
// The graph's `transitive_closure` and `affected_nodes` return HashSets.
// Node-bridge has `vec_str_to_js` for Vec<String>, so we collect into a
// Vec first. The order is not guaranteed (hash set iteration order), but
// that matches the semantics -- these are *sets* of nodes.

fn hash_set_to_js(env: napi_env, set: &HashSet<String>) -> napi_value {
    let v: Vec<String> = set.iter().cloned().collect();
    vec_str_to_js(env, &v)
}

// ---------------------------------------------------------------------------
// Helper: read a JS array of strings into a HashSet<String>
// ---------------------------------------------------------------------------
//
// The `affectedNodes` method takes a JS array of changed node names and
// needs to pass them as a HashSet to the Rust graph.

fn hash_set_from_js(env: napi_env, val: napi_value) -> HashSet<String> {
    vec_str_from_js(env, val).into_iter().collect()
}

// ---------------------------------------------------------------------------
// Constructor: new DirectedGraph()
// ---------------------------------------------------------------------------
//
// Called when JS code does `new DirectedGraph()`. We:
// 1. Extract `this` from the callback info (N-API passes it automatically)
// 2. Create a new Rust Graph
// 3. Wrap it inside `this` so methods can unwrap it later

unsafe extern "C" fn graph_new(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    wrap_data(env, this, Graph::new());
    this
}

// ---------------------------------------------------------------------------
// Node operations
// ---------------------------------------------------------------------------

/// graph.addNode(name) -- adds a node (idempotent, no error if exists)
unsafe extern "C" fn graph_add_node(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    let name = match str_from_js(env, args[0]) {
        Some(s) => s,
        None => {
            throw_error(env, "addNode requires a string argument");
            return undefined(env);
        }
    };
    let graph = unwrap_data_mut::<Graph>(env, this);
    graph.add_node(&name);
    undefined(env)
}

/// graph.removeNode(name) -- removes a node and all its edges
unsafe extern "C" fn graph_remove_node(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    let name = match str_from_js(env, args[0]) {
        Some(s) => s,
        None => {
            throw_error(env, "removeNode requires a string argument");
            return undefined(env);
        }
    };
    let graph = unwrap_data_mut::<Graph>(env, this);
    match graph.remove_node(&name) {
        Ok(()) => undefined(env),
        Err(e) => throw_graph_error(env, e),
    }
}

/// graph.hasNode(name) -- returns true if the node exists
unsafe extern "C" fn graph_has_node(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    let name = match str_from_js(env, args[0]) {
        Some(s) => s,
        None => {
            throw_error(env, "hasNode requires a string argument");
            return undefined(env);
        }
    };
    let graph = unwrap_data::<Graph>(env, this);
    bool_to_js(env, graph.has_node(&name))
}

/// graph.nodes() -- returns an array of all node names
unsafe extern "C" fn graph_nodes(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    let graph = unwrap_data::<Graph>(env, this);
    vec_str_to_js(env, &graph.nodes())
}

/// graph.size() -- returns the number of nodes
unsafe extern "C" fn graph_size(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    let graph = unwrap_data::<Graph>(env, this);
    usize_to_js(env, graph.size())
}

// ---------------------------------------------------------------------------
// Edge operations
// ---------------------------------------------------------------------------

/// graph.addEdge(from, to) -- adds a directed edge (creates nodes if needed)
unsafe extern "C" fn graph_add_edge(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 2);
    let from = match str_from_js(env, args[0]) {
        Some(s) => s,
        None => {
            throw_error(env, "addEdge requires two string arguments");
            return undefined(env);
        }
    };
    let to = match str_from_js(env, args[1]) {
        Some(s) => s,
        None => {
            throw_error(env, "addEdge requires two string arguments");
            return undefined(env);
        }
    };
    let graph = unwrap_data_mut::<Graph>(env, this);
    match graph.add_edge(&from, &to) {
        Ok(()) => undefined(env),
        Err(e) => throw_graph_error(env, e),
    }
}

/// graph.removeEdge(from, to) -- removes a directed edge
unsafe extern "C" fn graph_remove_edge(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 2);
    let from = match str_from_js(env, args[0]) {
        Some(s) => s,
        None => {
            throw_error(env, "removeEdge requires two string arguments");
            return undefined(env);
        }
    };
    let to = match str_from_js(env, args[1]) {
        Some(s) => s,
        None => {
            throw_error(env, "removeEdge requires two string arguments");
            return undefined(env);
        }
    };
    let graph = unwrap_data_mut::<Graph>(env, this);
    match graph.remove_edge(&from, &to) {
        Ok(()) => undefined(env),
        Err(e) => throw_graph_error(env, e),
    }
}

/// graph.hasEdge(from, to) -- returns true if the directed edge exists
unsafe extern "C" fn graph_has_edge(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 2);
    let from = match str_from_js(env, args[0]) {
        Some(s) => s,
        None => {
            throw_error(env, "hasEdge requires two string arguments");
            return undefined(env);
        }
    };
    let to = match str_from_js(env, args[1]) {
        Some(s) => s,
        None => {
            throw_error(env, "hasEdge requires two string arguments");
            return undefined(env);
        }
    };
    let graph = unwrap_data::<Graph>(env, this);
    bool_to_js(env, graph.has_edge(&from, &to))
}

/// graph.edges() -- returns array of [from, to] pairs
unsafe extern "C" fn graph_edges(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    let graph = unwrap_data::<Graph>(env, this);
    vec_tuple2_str_to_js(env, &graph.edges())
}

// ---------------------------------------------------------------------------
// Traversal operations
// ---------------------------------------------------------------------------

/// graph.predecessors(node) -- returns array of nodes that point TO this node
unsafe extern "C" fn graph_predecessors(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    let name = match str_from_js(env, args[0]) {
        Some(s) => s,
        None => {
            throw_error(env, "predecessors requires a string argument");
            return undefined(env);
        }
    };
    let graph = unwrap_data::<Graph>(env, this);
    match graph.predecessors(&name) {
        Ok(v) => vec_str_to_js(env, &v),
        Err(e) => throw_graph_error(env, e),
    }
}

/// graph.successors(node) -- returns array of nodes this node points TO
unsafe extern "C" fn graph_successors(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    let name = match str_from_js(env, args[0]) {
        Some(s) => s,
        None => {
            throw_error(env, "successors requires a string argument");
            return undefined(env);
        }
    };
    let graph = unwrap_data::<Graph>(env, this);
    match graph.successors(&name) {
        Ok(v) => vec_str_to_js(env, &v),
        Err(e) => throw_graph_error(env, e),
    }
}

// ---------------------------------------------------------------------------
// Graph algorithms
// ---------------------------------------------------------------------------

/// graph.topologicalSort() -- returns nodes in dependency order
unsafe extern "C" fn graph_topological_sort(
    env: napi_env,
    info: napi_callback_info,
) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    let graph = unwrap_data::<Graph>(env, this);
    match graph.topological_sort() {
        Ok(v) => vec_str_to_js(env, &v),
        Err(e) => throw_graph_error(env, e),
    }
}

/// graph.hasCycle() -- returns true if the graph contains a cycle
unsafe extern "C" fn graph_has_cycle(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    let graph = unwrap_data::<Graph>(env, this);
    bool_to_js(env, graph.has_cycle())
}

/// graph.transitiveClosure(node) -- returns all nodes reachable from node
unsafe extern "C" fn graph_transitive_closure(
    env: napi_env,
    info: napi_callback_info,
) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    let name = match str_from_js(env, args[0]) {
        Some(s) => s,
        None => {
            throw_error(env, "transitiveClosure requires a string argument");
            return undefined(env);
        }
    };
    let graph = unwrap_data::<Graph>(env, this);
    match graph.transitive_closure(&name) {
        Ok(closure) => hash_set_to_js(env, &closure),
        Err(e) => throw_graph_error(env, e),
    }
}

/// graph.affectedNodes(changedArray) -- returns all nodes transitively
/// depending on any of the changed nodes
unsafe extern "C" fn graph_affected_nodes(
    env: napi_env,
    info: napi_callback_info,
) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    let changed = hash_set_from_js(env, args[0]);
    let graph = unwrap_data::<Graph>(env, this);
    hash_set_to_js(env, &graph.affected_nodes(&changed))
}

/// graph.independentGroups() -- returns array of arrays, where each inner
/// array is a group of nodes that can be processed in parallel
unsafe extern "C" fn graph_independent_groups(
    env: napi_env,
    info: napi_callback_info,
) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    let graph = unwrap_data::<Graph>(env, this);
    match graph.independent_groups() {
        Ok(groups) => vec_vec_str_to_js(env, &groups),
        Err(e) => throw_graph_error(env, e),
    }
}

// ---------------------------------------------------------------------------
// Module registration
// ---------------------------------------------------------------------------
//
// N-API calls this function when the addon is loaded via `require()`.
// We define a DirectedGraph class with all its methods and attach it
// to the exports object.

#[no_mangle]
pub unsafe extern "C" fn napi_register_module_v1(
    env: napi_env,
    exports: napi_value,
) -> napi_value {
    // Define all instance methods using node-bridge's method_property helper.
    // Each entry creates a napi_property_descriptor with the given name and
    // callback function.
    let properties = [
        // -- Node operations --
        method_property("addNode", Some(graph_add_node)),
        method_property("removeNode", Some(graph_remove_node)),
        method_property("hasNode", Some(graph_has_node)),
        method_property("nodes", Some(graph_nodes)),
        method_property("size", Some(graph_size)),
        // -- Edge operations --
        method_property("addEdge", Some(graph_add_edge)),
        method_property("removeEdge", Some(graph_remove_edge)),
        method_property("hasEdge", Some(graph_has_edge)),
        method_property("edges", Some(graph_edges)),
        // -- Traversal --
        method_property("predecessors", Some(graph_predecessors)),
        method_property("successors", Some(graph_successors)),
        // -- Algorithms --
        method_property("topologicalSort", Some(graph_topological_sort)),
        method_property("hasCycle", Some(graph_has_cycle)),
        method_property("transitiveClosure", Some(graph_transitive_closure)),
        method_property("affectedNodes", Some(graph_affected_nodes)),
        method_property("independentGroups", Some(graph_independent_groups)),
    ];

    // Create the class. `define_class` calls napi_define_class under the hood,
    // which registers the constructor and all prototype methods in one shot.
    let class = define_class(env, "DirectedGraph", Some(graph_new), &properties);

    // Attach the class constructor to exports so JS can do:
    //   const { DirectedGraph } = require('./directed_graph_native.node');
    set_named_property(env, exports, "DirectedGraph", class);

    exports
}
