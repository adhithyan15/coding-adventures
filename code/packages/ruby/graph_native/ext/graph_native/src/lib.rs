use std::ffi::{c_int, c_void};
use std::slice;

use graph::{
    bfs, connected_components, dfs, has_cycle, is_connected, minimum_spanning_tree, shortest_path,
    Graph, GraphError, GraphRepr,
};
use ruby_bridge::VALUE;

static mut NODE_NOT_FOUND_ERROR: VALUE = 0;
static mut EDGE_NOT_FOUND_ERROR: VALUE = 0;
static mut NOT_CONNECTED_ERROR: VALUE = 0;

fn parse_repr(value: &str) -> Option<GraphRepr> {
    match value {
        "adjacency_list" => Some(GraphRepr::AdjacencyList),
        "adjacency_matrix" => Some(GraphRepr::AdjacencyMatrix),
        _ => None,
    }
}

fn repr_name(repr: GraphRepr) -> &'static str {
    match repr {
        GraphRepr::AdjacencyList => "adjacency_list",
        GraphRepr::AdjacencyMatrix => "adjacency_matrix",
    }
}

unsafe fn get_graph(self_val: VALUE) -> &'static Graph {
    ruby_bridge::unwrap_data::<Graph>(self_val)
}

unsafe fn get_graph_mut(self_val: VALUE) -> &'static mut Graph {
    ruby_bridge::unwrap_data_mut::<Graph>(self_val)
}

fn raise_graph_error(err: GraphError) -> ! {
    match err {
        GraphError::NodeNotFound(node) => unsafe {
            ruby_bridge::raise_error(NODE_NOT_FOUND_ERROR, &format!("node not found: {}", node))
        },
        GraphError::EdgeNotFound(left, right) => unsafe {
            ruby_bridge::raise_error(
                EDGE_NOT_FOUND_ERROR,
                &format!("edge not found: {} -- {}", left, right),
            )
        },
        GraphError::NotConnected => unsafe {
            ruby_bridge::raise_error(NOT_CONNECTED_ERROR, "graph is not connected")
        },
    }
}

unsafe extern "C" fn graph_alloc(klass: VALUE) -> VALUE {
    ruby_bridge::wrap_data(klass, Graph::new(GraphRepr::AdjacencyList))
}

extern "C" fn graph_initialize(argc: c_int, argv: *const VALUE, self_val: VALUE) -> VALUE {
    let args = unsafe { slice::from_raw_parts(argv, argc as usize) };
    if args.len() > 1 {
        ruby_bridge::raise_arg_error("Graph.new accepts at most one repr argument");
    }

    let repr = if let Some(value) = args.first() {
        let repr_name = ruby_bridge::str_from_rb(*value)
            .unwrap_or_else(|| ruby_bridge::raise_arg_error("repr must be a String"));
        parse_repr(&repr_name).unwrap_or_else(|| {
            ruby_bridge::raise_arg_error(
                "repr must be 'adjacency_list' or 'adjacency_matrix'",
            )
        })
    } else {
        GraphRepr::AdjacencyList
    };

    unsafe {
        *get_graph_mut(self_val) = Graph::new(repr);
    }
    self_val
}

extern "C" fn graph_repr(self_val: VALUE) -> VALUE {
    ruby_bridge::str_to_rb(repr_name(unsafe { get_graph(self_val).repr() }))
}

extern "C" fn graph_add_node(self_val: VALUE, node: VALUE) -> VALUE {
    let node = ruby_bridge::str_from_rb(node)
        .unwrap_or_else(|| ruby_bridge::raise_arg_error("add_node expects a String"));
    unsafe { get_graph_mut(self_val).add_node(node) };
    ruby_bridge::QNIL
}

extern "C" fn graph_remove_node(self_val: VALUE, node: VALUE) -> VALUE {
    let node = ruby_bridge::str_from_rb(node)
        .unwrap_or_else(|| ruby_bridge::raise_arg_error("remove_node expects a String"));
    match unsafe { get_graph_mut(self_val).remove_node(&node) } {
        Ok(()) => ruby_bridge::QNIL,
        Err(err) => raise_graph_error(err),
    }
}

extern "C" fn graph_has_node(self_val: VALUE, node: VALUE) -> VALUE {
    let node = ruby_bridge::str_from_rb(node)
        .unwrap_or_else(|| ruby_bridge::raise_arg_error("has_node? expects a String"));
    ruby_bridge::bool_to_rb(unsafe { get_graph(self_val).has_node(&node) })
}

extern "C" fn graph_nodes(self_val: VALUE) -> VALUE {
    ruby_bridge::vec_str_to_rb(&unsafe { get_graph(self_val).nodes() })
}

extern "C" fn graph_size(self_val: VALUE) -> VALUE {
    ruby_bridge::usize_to_rb(unsafe { get_graph(self_val).size() })
}

extern "C" fn graph_add_edge(argc: c_int, argv: *const VALUE, self_val: VALUE) -> VALUE {
    let args = unsafe { slice::from_raw_parts(argv, argc as usize) };
    if !(args.len() == 2 || args.len() == 3) {
        ruby_bridge::raise_arg_error("add_edge expects left, right, and optional weight");
    }

    let left = ruby_bridge::str_from_rb(args[0])
        .unwrap_or_else(|| ruby_bridge::raise_arg_error("left node must be a String"));
    let right = ruby_bridge::str_from_rb(args[1])
        .unwrap_or_else(|| ruby_bridge::raise_arg_error("right node must be a String"));
    let weight = args
        .get(2)
        .map(|value| ruby_bridge::f64_from_rb(*value))
        .unwrap_or(1.0);

    unsafe { get_graph_mut(self_val).add_edge(left, right, weight) };
    ruby_bridge::QNIL
}

extern "C" fn graph_remove_edge(self_val: VALUE, left: VALUE, right: VALUE) -> VALUE {
    let left = ruby_bridge::str_from_rb(left)
        .unwrap_or_else(|| ruby_bridge::raise_arg_error("left node must be a String"));
    let right = ruby_bridge::str_from_rb(right)
        .unwrap_or_else(|| ruby_bridge::raise_arg_error("right node must be a String"));
    match unsafe { get_graph_mut(self_val).remove_edge(&left, &right) } {
        Ok(()) => ruby_bridge::QNIL,
        Err(err) => raise_graph_error(err),
    }
}

extern "C" fn graph_has_edge(self_val: VALUE, left: VALUE, right: VALUE) -> VALUE {
    let left = ruby_bridge::str_from_rb(left)
        .unwrap_or_else(|| ruby_bridge::raise_arg_error("left node must be a String"));
    let right = ruby_bridge::str_from_rb(right)
        .unwrap_or_else(|| ruby_bridge::raise_arg_error("right node must be a String"));
    ruby_bridge::bool_to_rb(unsafe { get_graph(self_val).has_edge(&left, &right) })
}

extern "C" fn graph_edges(self_val: VALUE) -> VALUE {
    ruby_bridge::vec_tuple3_str_f64_to_rb(&unsafe { get_graph(self_val).edges() })
}

extern "C" fn graph_edge_weight(self_val: VALUE, left: VALUE, right: VALUE) -> VALUE {
    let left = ruby_bridge::str_from_rb(left)
        .unwrap_or_else(|| ruby_bridge::raise_arg_error("left node must be a String"));
    let right = ruby_bridge::str_from_rb(right)
        .unwrap_or_else(|| ruby_bridge::raise_arg_error("right node must be a String"));
    match unsafe { get_graph(self_val).edge_weight(&left, &right) } {
        Ok(weight) => ruby_bridge::f64_to_rb(weight),
        Err(err) => raise_graph_error(err),
    }
}

extern "C" fn graph_neighbors(self_val: VALUE, node: VALUE) -> VALUE {
    let node = ruby_bridge::str_from_rb(node)
        .unwrap_or_else(|| ruby_bridge::raise_arg_error("neighbors expects a String"));
    match unsafe { get_graph(self_val).neighbors(&node) } {
        Ok(nodes) => ruby_bridge::vec_str_to_rb(&nodes),
        Err(err) => raise_graph_error(err),
    }
}

extern "C" fn graph_neighbors_weighted_entries(self_val: VALUE, node: VALUE) -> VALUE {
    let node = ruby_bridge::str_from_rb(node).unwrap_or_else(|| {
        ruby_bridge::raise_arg_error("neighbors_weighted_entries expects a String")
    });
    match unsafe { get_graph(self_val).neighbors_weighted(&node) } {
        Ok(entries) => {
            let pairs: Vec<(String, f64)> = entries.into_iter().collect();
            ruby_bridge::vec_tuple2_str_f64_to_rb(&pairs)
        }
        Err(err) => raise_graph_error(err),
    }
}

extern "C" fn graph_degree(self_val: VALUE, node: VALUE) -> VALUE {
    let node = ruby_bridge::str_from_rb(node)
        .unwrap_or_else(|| ruby_bridge::raise_arg_error("degree expects a String"));
    match unsafe { get_graph(self_val).degree(&node) } {
        Ok(size) => ruby_bridge::usize_to_rb(size),
        Err(err) => raise_graph_error(err),
    }
}

extern "C" fn graph_bfs(self_val: VALUE, start: VALUE) -> VALUE {
    let start = ruby_bridge::str_from_rb(start)
        .unwrap_or_else(|| ruby_bridge::raise_arg_error("bfs expects a String"));
    match bfs(unsafe { get_graph(self_val) }, &start) {
        Ok(nodes) => ruby_bridge::vec_str_to_rb(&nodes),
        Err(err) => raise_graph_error(err),
    }
}

extern "C" fn graph_dfs(self_val: VALUE, start: VALUE) -> VALUE {
    let start = ruby_bridge::str_from_rb(start)
        .unwrap_or_else(|| ruby_bridge::raise_arg_error("dfs expects a String"));
    match dfs(unsafe { get_graph(self_val) }, &start) {
        Ok(nodes) => ruby_bridge::vec_str_to_rb(&nodes),
        Err(err) => raise_graph_error(err),
    }
}

extern "C" fn graph_is_connected(self_val: VALUE) -> VALUE {
    ruby_bridge::bool_to_rb(is_connected(unsafe { get_graph(self_val) }))
}

extern "C" fn graph_connected_components(self_val: VALUE) -> VALUE {
    ruby_bridge::vec_vec_str_to_rb(&connected_components(unsafe { get_graph(self_val) }))
}

extern "C" fn graph_has_cycle(self_val: VALUE) -> VALUE {
    ruby_bridge::bool_to_rb(has_cycle(unsafe { get_graph(self_val) }))
}

extern "C" fn graph_shortest_path(self_val: VALUE, start: VALUE, finish: VALUE) -> VALUE {
    let start = ruby_bridge::str_from_rb(start)
        .unwrap_or_else(|| ruby_bridge::raise_arg_error("shortest_path start must be a String"));
    let finish = ruby_bridge::str_from_rb(finish)
        .unwrap_or_else(|| ruby_bridge::raise_arg_error("shortest_path finish must be a String"));
    ruby_bridge::vec_str_to_rb(&shortest_path(unsafe { get_graph(self_val) }, &start, &finish))
}

extern "C" fn graph_minimum_spanning_tree(self_val: VALUE) -> VALUE {
    match minimum_spanning_tree(unsafe { get_graph(self_val) }) {
        Ok(edges) => ruby_bridge::vec_tuple3_str_f64_to_rb(&edges),
        Err(err) => raise_graph_error(err),
    }
}

extern "C" fn graph_to_s(self_val: VALUE) -> VALUE {
    ruby_bridge::str_to_rb(&format!("{}", unsafe { get_graph(self_val) }))
}

#[no_mangle]
pub extern "C" fn Init_graph_native() {
    let coding_adventures = ruby_bridge::define_module("CodingAdventures");
    let graph_native = ruby_bridge::define_module_under(coding_adventures, "GraphNative");
    let native_graph = ruby_bridge::define_class_under(
        graph_native,
        "NativeGraph",
        ruby_bridge::object_class(),
    );

    unsafe {
        NODE_NOT_FOUND_ERROR =
            ruby_bridge::path2class("CodingAdventures::GraphNative::NodeNotFoundError");
        EDGE_NOT_FOUND_ERROR =
            ruby_bridge::path2class("CodingAdventures::GraphNative::EdgeNotFoundError");
        NOT_CONNECTED_ERROR =
            ruby_bridge::path2class("CodingAdventures::GraphNative::NotConnectedError");
    }

    ruby_bridge::define_alloc_func(native_graph, graph_alloc);
    ruby_bridge::define_method_raw(native_graph, "initialize", graph_initialize as *const c_void, -1);
    ruby_bridge::define_method_raw(native_graph, "repr", graph_repr as *const c_void, 0);
    ruby_bridge::define_method_raw(native_graph, "add_node", graph_add_node as *const c_void, 1);
    ruby_bridge::define_method_raw(native_graph, "remove_node", graph_remove_node as *const c_void, 1);
    ruby_bridge::define_method_raw(native_graph, "has_node?", graph_has_node as *const c_void, 1);
    ruby_bridge::define_method_raw(native_graph, "nodes", graph_nodes as *const c_void, 0);
    ruby_bridge::define_method_raw(native_graph, "size", graph_size as *const c_void, 0);
    ruby_bridge::define_method_raw(native_graph, "add_edge", graph_add_edge as *const c_void, -1);
    ruby_bridge::define_method_raw(native_graph, "remove_edge", graph_remove_edge as *const c_void, 2);
    ruby_bridge::define_method_raw(native_graph, "has_edge?", graph_has_edge as *const c_void, 2);
    ruby_bridge::define_method_raw(native_graph, "edges", graph_edges as *const c_void, 0);
    ruby_bridge::define_method_raw(native_graph, "edge_weight", graph_edge_weight as *const c_void, 2);
    ruby_bridge::define_method_raw(native_graph, "neighbors", graph_neighbors as *const c_void, 1);
    ruby_bridge::define_method_raw(
        native_graph,
        "neighbors_weighted_entries",
        graph_neighbors_weighted_entries as *const c_void,
        1,
    );
    ruby_bridge::define_method_raw(native_graph, "degree", graph_degree as *const c_void, 1);
    ruby_bridge::define_method_raw(native_graph, "bfs", graph_bfs as *const c_void, 1);
    ruby_bridge::define_method_raw(native_graph, "dfs", graph_dfs as *const c_void, 1);
    ruby_bridge::define_method_raw(native_graph, "is_connected?", graph_is_connected as *const c_void, 0);
    ruby_bridge::define_method_raw(
        native_graph,
        "connected_components",
        graph_connected_components as *const c_void,
        0,
    );
    ruby_bridge::define_method_raw(native_graph, "has_cycle?", graph_has_cycle as *const c_void, 0);
    ruby_bridge::define_method_raw(
        native_graph,
        "shortest_path",
        graph_shortest_path as *const c_void,
        2,
    );
    ruby_bridge::define_method_raw(
        native_graph,
        "minimum_spanning_tree",
        graph_minimum_spanning_tree as *const c_void,
        0,
    );
    ruby_bridge::define_method_raw(native_graph, "to_s", graph_to_s as *const c_void, 0);
    ruby_bridge::define_method_raw(native_graph, "inspect", graph_to_s as *const c_void, 0);
}
