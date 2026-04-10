use graph::{
    bfs, connected_components, dfs, has_cycle, is_connected, minimum_spanning_tree, shortest_path,
    Graph, GraphError, GraphRepr,
};
use node_bridge::*;

macro_rules! unwrap_ref {
    ($env:expr, $value:expr, $ty:ty) => {
        unwrap_data::<$ty>($env, $value)
            .as_ref()
            .expect(concat!(stringify!($ty), " should always be wrapped"))
    };
}

macro_rules! unwrap_mut {
    ($env:expr, $value:expr, $ty:ty) => {
        unwrap_data_mut::<$ty>($env, $value)
            .as_mut()
            .expect(concat!(stringify!($ty), " should always be wrapped"))
    };
}

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

fn throw_graph_error(env: napi_env, err: GraphError) -> napi_value {
    throw_error(env, &err.to_string());
    undefined(env)
}

unsafe extern "C" fn graph_new(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    let repr = if args.is_empty() {
        GraphRepr::AdjacencyList
    } else {
        let value = match str_from_js(env, args[0]) {
            Some(value) => value,
            None => {
                throw_error(env, "Graph constructor expects an optional repr string");
                return undefined(env);
            }
        };
        match parse_repr(&value) {
            Some(repr) => repr,
            None => {
                throw_error(env, "repr must be 'adjacency_list' or 'adjacency_matrix'");
                return undefined(env);
            }
        }
    };

    wrap_data(env, this, Graph::new(repr));
    this
}

unsafe extern "C" fn graph_repr(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    str_to_js(env, repr_name(unwrap_ref!(env, this, Graph).repr()))
}

unsafe extern "C" fn graph_add_node(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    let node = match args.first().and_then(|value| str_from_js(env, *value)) {
        Some(node) => node,
        None => {
            throw_error(env, "addNode requires a string argument");
            return undefined(env);
        }
    };
    unwrap_mut!(env, this, Graph).add_node(node);
    undefined(env)
}

unsafe extern "C" fn graph_remove_node(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    let node = match args.first().and_then(|value| str_from_js(env, *value)) {
        Some(node) => node,
        None => {
            throw_error(env, "removeNode requires a string argument");
            return undefined(env);
        }
    };
    match unwrap_mut!(env, this, Graph).remove_node(&node) {
        Ok(()) => undefined(env),
        Err(err) => throw_graph_error(env, err),
    }
}

unsafe extern "C" fn graph_has_node(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    let node = match args.first().and_then(|value| str_from_js(env, *value)) {
        Some(node) => node,
        None => {
            throw_error(env, "hasNode requires a string argument");
            return undefined(env);
        }
    };
    bool_to_js(env, unwrap_ref!(env, this, Graph).has_node(&node))
}

unsafe extern "C" fn graph_nodes(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    vec_str_to_js(env, &unwrap_ref!(env, this, Graph).nodes())
}

unsafe extern "C" fn graph_size(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    usize_to_js(env, unwrap_ref!(env, this, Graph).size())
}

unsafe extern "C" fn graph_add_edge(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 3);
    if args.len() < 2 {
        throw_error(env, "addEdge requires left and right string arguments");
        return undefined(env);
    }

    let left = match str_from_js(env, args[0]) {
        Some(value) => value,
        None => {
            throw_error(env, "left node must be a string");
            return undefined(env);
        }
    };
    let right = match str_from_js(env, args[1]) {
        Some(value) => value,
        None => {
            throw_error(env, "right node must be a string");
            return undefined(env);
        }
    };
    let weight = if args.len() >= 3 {
        match f64_from_js(env, args[2]) {
            Some(weight) => weight,
            None => {
                throw_error(env, "weight must be numeric");
                return undefined(env);
            }
        }
    } else {
        1.0
    };

    unwrap_mut!(env, this, Graph).add_edge(left, right, weight);
    undefined(env)
}

unsafe extern "C" fn graph_remove_edge(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 2);
    if args.len() < 2 {
        throw_error(env, "removeEdge requires two string arguments");
        return undefined(env);
    }
    let left = match str_from_js(env, args[0]) {
        Some(value) => value,
        None => {
            throw_error(env, "left node must be a string");
            return undefined(env);
        }
    };
    let right = match str_from_js(env, args[1]) {
        Some(value) => value,
        None => {
            throw_error(env, "right node must be a string");
            return undefined(env);
        }
    };
    match unwrap_mut!(env, this, Graph).remove_edge(&left, &right) {
        Ok(()) => undefined(env),
        Err(err) => throw_graph_error(env, err),
    }
}

unsafe extern "C" fn graph_has_edge(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 2);
    if args.len() < 2 {
        throw_error(env, "hasEdge requires two string arguments");
        return undefined(env);
    }
    let left = match str_from_js(env, args[0]) {
        Some(value) => value,
        None => {
            throw_error(env, "left node must be a string");
            return undefined(env);
        }
    };
    let right = match str_from_js(env, args[1]) {
        Some(value) => value,
        None => {
            throw_error(env, "right node must be a string");
            return undefined(env);
        }
    };
    bool_to_js(env, unwrap_ref!(env, this, Graph).has_edge(&left, &right))
}

unsafe extern "C" fn graph_edges(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    vec_tuple3_str_f64_to_js(env, &unwrap_ref!(env, this, Graph).edges())
}

unsafe extern "C" fn graph_edge_weight(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 2);
    if args.len() < 2 {
        throw_error(env, "edgeWeight requires two string arguments");
        return undefined(env);
    }
    let left = match str_from_js(env, args[0]) {
        Some(value) => value,
        None => {
            throw_error(env, "left node must be a string");
            return undefined(env);
        }
    };
    let right = match str_from_js(env, args[1]) {
        Some(value) => value,
        None => {
            throw_error(env, "right node must be a string");
            return undefined(env);
        }
    };
    match unwrap_ref!(env, this, Graph).edge_weight(&left, &right) {
        Ok(weight) => f64_to_js(env, weight),
        Err(err) => throw_graph_error(env, err),
    }
}

unsafe extern "C" fn graph_neighbors(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    let node = match args.first().and_then(|value| str_from_js(env, *value)) {
        Some(node) => node,
        None => {
            throw_error(env, "neighbors requires a string argument");
            return undefined(env);
        }
    };
    match unwrap_ref!(env, this, Graph).neighbors(&node) {
        Ok(nodes) => vec_str_to_js(env, &nodes),
        Err(err) => throw_graph_error(env, err),
    }
}

unsafe extern "C" fn graph_neighbors_weighted_entries(
    env: napi_env,
    info: napi_callback_info,
) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    let node = match args.first().and_then(|value| str_from_js(env, *value)) {
        Some(node) => node,
        None => {
            throw_error(env, "neighborsWeightedEntries requires a string argument");
            return undefined(env);
        }
    };
    match unwrap_ref!(env, this, Graph).neighbors_weighted(&node) {
        Ok(entries) => {
            let pairs: Vec<(String, f64)> = entries.into_iter().collect();
            vec_tuple2_str_f64_to_js(env, &pairs)
        }
        Err(err) => throw_graph_error(env, err),
    }
}

unsafe extern "C" fn graph_degree(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    let node = match args.first().and_then(|value| str_from_js(env, *value)) {
        Some(node) => node,
        None => {
            throw_error(env, "degree requires a string argument");
            return undefined(env);
        }
    };
    match unwrap_ref!(env, this, Graph).degree(&node) {
        Ok(degree) => usize_to_js(env, degree),
        Err(err) => throw_graph_error(env, err),
    }
}

unsafe extern "C" fn graph_bfs(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    let start = match args.first().and_then(|value| str_from_js(env, *value)) {
        Some(start) => start,
        None => {
            throw_error(env, "bfs requires a string argument");
            return undefined(env);
        }
    };
    match bfs(unwrap_ref!(env, this, Graph), &start) {
        Ok(nodes) => vec_str_to_js(env, &nodes),
        Err(err) => throw_graph_error(env, err),
    }
}

unsafe extern "C" fn graph_dfs(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    let start = match args.first().and_then(|value| str_from_js(env, *value)) {
        Some(start) => start,
        None => {
            throw_error(env, "dfs requires a string argument");
            return undefined(env);
        }
    };
    match dfs(unwrap_ref!(env, this, Graph), &start) {
        Ok(nodes) => vec_str_to_js(env, &nodes),
        Err(err) => throw_graph_error(env, err),
    }
}

unsafe extern "C" fn graph_is_connected(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    bool_to_js(env, is_connected(unwrap_ref!(env, this, Graph)))
}

unsafe extern "C" fn graph_connected_components(
    env: napi_env,
    info: napi_callback_info,
) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    vec_vec_str_to_js(env, &connected_components(unwrap_ref!(env, this, Graph)))
}

unsafe extern "C" fn graph_has_cycle(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    bool_to_js(env, has_cycle(unwrap_ref!(env, this, Graph)))
}

unsafe extern "C" fn graph_shortest_path(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 2);
    if args.len() < 2 {
        throw_error(env, "shortestPath requires start and finish strings");
        return undefined(env);
    }
    let start = match str_from_js(env, args[0]) {
        Some(value) => value,
        None => {
            throw_error(env, "start node must be a string");
            return undefined(env);
        }
    };
    let finish = match str_from_js(env, args[1]) {
        Some(value) => value,
        None => {
            throw_error(env, "finish node must be a string");
            return undefined(env);
        }
    };
    vec_str_to_js(env, &shortest_path(unwrap_ref!(env, this, Graph), &start, &finish))
}

unsafe extern "C" fn graph_minimum_spanning_tree(
    env: napi_env,
    info: napi_callback_info,
) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    match minimum_spanning_tree(unwrap_ref!(env, this, Graph)) {
        Ok(edges) => vec_tuple3_str_f64_to_js(env, &edges),
        Err(err) => throw_graph_error(env, err),
    }
}

unsafe extern "C" fn graph_to_string(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    str_to_js(env, &format!("{}", unwrap_ref!(env, this, Graph)))
}

#[no_mangle]
pub unsafe extern "C" fn napi_register_module_v1(
    env: napi_env,
    exports: napi_value,
) -> napi_value {
    let properties = [
        method_property("repr", Some(graph_repr)),
        method_property("addNode", Some(graph_add_node)),
        method_property("removeNode", Some(graph_remove_node)),
        method_property("hasNode", Some(graph_has_node)),
        method_property("nodes", Some(graph_nodes)),
        method_property("size", Some(graph_size)),
        method_property("addEdge", Some(graph_add_edge)),
        method_property("removeEdge", Some(graph_remove_edge)),
        method_property("hasEdge", Some(graph_has_edge)),
        method_property("edges", Some(graph_edges)),
        method_property("edgeWeight", Some(graph_edge_weight)),
        method_property("neighbors", Some(graph_neighbors)),
        method_property("neighborsWeightedEntries", Some(graph_neighbors_weighted_entries)),
        method_property("degree", Some(graph_degree)),
        method_property("bfs", Some(graph_bfs)),
        method_property("dfs", Some(graph_dfs)),
        method_property("isConnected", Some(graph_is_connected)),
        method_property("connectedComponents", Some(graph_connected_components)),
        method_property("hasCycle", Some(graph_has_cycle)),
        method_property("shortestPath", Some(graph_shortest_path)),
        method_property("minimumSpanningTree", Some(graph_minimum_spanning_tree)),
        method_property("toString", Some(graph_to_string)),
    ];

    let native_graph = define_class(env, "NativeGraph", Some(graph_new), &properties);
    set_named_property(env, exports, "NativeGraph", native_graph);
    exports
}
