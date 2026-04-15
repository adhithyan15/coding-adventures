use std::ffi::{c_char, c_int, c_void, CString};
use std::ptr;

use graph::{
    bfs, connected_components, dfs, has_cycle, is_connected, minimum_spanning_tree, shortest_path,
    Graph, GraphError, GraphRepr,
};
use python_bridge::*;

const PY_TP_DEALLOC: c_int = 52;
const PY_TP_REPR: c_int = 66;
const PY_TP_METHODS: c_int = 64;
const PY_TP_NEW: c_int = 65;
const PY_SQ_LENGTH: c_int = 45;
const PY_SQ_CONTAINS: c_int = 41;

#[repr(C)]
struct GraphObject {
    ob_refcnt: isize,
    ob_type: PyObjectPtr,
    graph: *mut Graph,
}

static mut NODE_NOT_FOUND_ERROR: PyObjectPtr = ptr::null_mut();
static mut EDGE_NOT_FOUND_ERROR: PyObjectPtr = ptr::null_mut();

unsafe fn set_graph_error(err: GraphError) -> PyObjectPtr {
    match err {
        GraphError::NodeNotFound(node) => {
            set_error(NODE_NOT_FOUND_ERROR, &format!("node not found: {}", node));
        }
        GraphError::EdgeNotFound(left, right) => {
            set_error(
                EDGE_NOT_FOUND_ERROR,
                &format!("edge not found: {} -- {}", left, right),
            );
        }
        GraphError::NotConnected => {
            set_error(value_error_class(), "graph is not connected");
        }
    }
    ptr::null_mut()
}

unsafe fn get_graph(slf: PyObjectPtr) -> &'static Graph {
    &*((slf as *mut GraphObject).read().graph)
}

unsafe fn get_graph_mut(slf: PyObjectPtr) -> &'static mut Graph {
    &mut *(*(slf as *mut GraphObject)).graph
}

extern "C" {
    fn PyType_GenericAlloc(type_obj: PyObjectPtr, nitems: isize) -> PyObjectPtr;
    fn PyObject_Free(ptr: *mut c_void);
}

fn cstr(s: &str) -> *const c_char {
    CString::new(s).expect("no NUL").into_raw()
}

fn parse_repr(value: &str) -> Option<GraphRepr> {
    match value {
        "adjacency_list" => Some(GraphRepr::AdjacencyList),
        "adjacency_matrix" => Some(GraphRepr::AdjacencyMatrix),
        _ => None,
    }
}

unsafe fn parse_weight_arg(args: PyObjectPtr) -> Option<f64> {
    let weight_arg = PyTuple_GetItem(args, 2);
    if weight_arg.is_null() {
        PyErr_Clear();
        return Some(1.0);
    }
    match f64_from_py(weight_arg) {
        Some(weight) => Some(weight),
        None => {
            set_error(type_error_class(), "weight must be numeric");
            None
        }
    }
}

unsafe extern "C" fn graph_new(
    type_obj: PyObjectPtr,
    args: PyObjectPtr,
    _kwargs: PyObjectPtr,
) -> PyObjectPtr {
    let repr = match parse_arg_str(args, 0) {
        Some(value) => match parse_repr(&value) {
            Some(repr) => repr,
            None => {
                set_error(
                    value_error_class(),
                    "repr must be 'adjacency_list' or 'adjacency_matrix'",
                );
                return ptr::null_mut();
            }
        },
        None => GraphRepr::AdjacencyList,
    };

    let obj = PyType_GenericAlloc(type_obj, 0);
    if obj.is_null() {
        return ptr::null_mut();
    }
    (*(obj as *mut GraphObject)).graph = Box::into_raw(Box::new(Graph::new(repr)));
    obj
}

unsafe extern "C" fn graph_dealloc(obj: PyObjectPtr) {
    let graph_obj = obj as *mut GraphObject;
    if !(*graph_obj).graph.is_null() {
        let _ = Box::from_raw((*graph_obj).graph);
        (*graph_obj).graph = ptr::null_mut();
    }
    PyObject_Free(obj as *mut c_void);
}

unsafe extern "C" fn graph_sq_length(slf: PyObjectPtr) -> isize {
    get_graph(slf).size() as isize
}

unsafe extern "C" fn graph_sq_contains(slf: PyObjectPtr, key: PyObjectPtr) -> c_int {
    match str_from_py(key) {
        Some(node) => {
            if get_graph(slf).has_node(&node) {
                1
            } else {
                0
            }
        }
        None => -1,
    }
}

unsafe extern "C" fn graph_add_node(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let node = match parse_arg_str(args, 0) {
        Some(node) => node,
        None => return ptr::null_mut(),
    };
    get_graph_mut(slf).add_node(node);
    py_none()
}

unsafe extern "C" fn graph_remove_node(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let node = match parse_arg_str(args, 0) {
        Some(node) => node,
        None => return ptr::null_mut(),
    };
    match get_graph_mut(slf).remove_node(&node) {
        Ok(()) => py_none(),
        Err(err) => set_graph_error(err),
    }
}

unsafe extern "C" fn graph_has_node(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let node = match parse_arg_str(args, 0) {
        Some(node) => node,
        None => return ptr::null_mut(),
    };
    bool_to_py(get_graph(slf).has_node(&node))
}

unsafe extern "C" fn graph_nodes(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    vec_str_to_py(&get_graph(slf).nodes())
}

unsafe extern "C" fn graph_add_edge(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let (left, right) = match parse_args_2str(args) {
        Some(pair) => pair,
        None => return ptr::null_mut(),
    };
    let weight = match parse_weight_arg(args) {
        Some(weight) => weight,
        None => return ptr::null_mut(),
    };
    get_graph_mut(slf).add_edge(left, right, weight);
    py_none()
}

unsafe extern "C" fn graph_remove_edge(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let (left, right) = match parse_args_2str(args) {
        Some(pair) => pair,
        None => return ptr::null_mut(),
    };
    match get_graph_mut(slf).remove_edge(&left, &right) {
        Ok(()) => py_none(),
        Err(err) => set_graph_error(err),
    }
}

unsafe extern "C" fn graph_has_edge(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let (left, right) = match parse_args_2str(args) {
        Some(pair) => pair,
        None => return ptr::null_mut(),
    };
    bool_to_py(get_graph(slf).has_edge(&left, &right))
}

unsafe extern "C" fn graph_edges(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    vec_tuple3_str_f64_to_py(&get_graph(slf).edges())
}

unsafe extern "C" fn graph_edge_weight(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let (left, right) = match parse_args_2str(args) {
        Some(pair) => pair,
        None => return ptr::null_mut(),
    };
    match get_graph(slf).edge_weight(&left, &right) {
        Ok(weight) => f64_to_py(weight),
        Err(err) => set_graph_error(err),
    }
}

unsafe extern "C" fn graph_neighbors(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let node = match parse_arg_str(args, 0) {
        Some(node) => node,
        None => return ptr::null_mut(),
    };
    match get_graph(slf).neighbors(&node) {
        Ok(neighbors) => vec_str_to_py(&neighbors),
        Err(err) => set_graph_error(err),
    }
}

unsafe extern "C" fn graph_neighbors_weighted(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let node = match parse_arg_str(args, 0) {
        Some(node) => node,
        None => return ptr::null_mut(),
    };
    match get_graph(slf).neighbors_weighted(&node) {
        Ok(neighbors) => map_str_f64_to_py(&neighbors),
        Err(err) => set_graph_error(err),
    }
}

unsafe extern "C" fn graph_degree(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let node = match parse_arg_str(args, 0) {
        Some(node) => node,
        None => return ptr::null_mut(),
    };
    match get_graph(slf).degree(&node) {
        Ok(degree) => usize_to_py(degree),
        Err(err) => set_graph_error(err),
    }
}

unsafe extern "C" fn graph_bfs(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let start = match parse_arg_str(args, 0) {
        Some(node) => node,
        None => return ptr::null_mut(),
    };
    match bfs(get_graph(slf), &start) {
        Ok(nodes) => vec_str_to_py(&nodes),
        Err(err) => set_graph_error(err),
    }
}

unsafe extern "C" fn graph_dfs(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let start = match parse_arg_str(args, 0) {
        Some(node) => node,
        None => return ptr::null_mut(),
    };
    match dfs(get_graph(slf), &start) {
        Ok(nodes) => vec_str_to_py(&nodes),
        Err(err) => set_graph_error(err),
    }
}

unsafe extern "C" fn graph_is_connected(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    bool_to_py(is_connected(get_graph(slf)))
}

unsafe extern "C" fn graph_connected_components(
    slf: PyObjectPtr,
    _args: PyObjectPtr,
) -> PyObjectPtr {
    vec_vec_str_to_py(&connected_components(get_graph(slf)))
}

unsafe extern "C" fn graph_has_cycle(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    bool_to_py(has_cycle(get_graph(slf)))
}

unsafe extern "C" fn graph_shortest_path(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let (start, end) = match parse_args_2str(args) {
        Some(pair) => pair,
        None => return ptr::null_mut(),
    };
    vec_str_to_py(&shortest_path(get_graph(slf), &start, &end))
}

unsafe extern "C" fn graph_minimum_spanning_tree(
    slf: PyObjectPtr,
    _args: PyObjectPtr,
) -> PyObjectPtr {
    match minimum_spanning_tree(get_graph(slf)) {
        Ok(edges) => vec_tuple3_str_f64_to_py(&edges),
        Err(err) => set_graph_error(err),
    }
}

unsafe extern "C" fn graph_repr(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    str_to_py(&format!("{}", get_graph(slf)))
}

#[no_mangle]
pub unsafe extern "C" fn PyInit_graph_native() -> PyObjectPtr {
    static mut METHODS: [PyMethodDef; 20] = [PyMethodDef {
        ml_name: ptr::null(),
        ml_meth: None,
        ml_flags: 0,
        ml_doc: ptr::null(),
    }; 20];

    METHODS[0] = PyMethodDef {
        ml_name: cstr("add_node"),
        ml_meth: Some(graph_add_node),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[1] = PyMethodDef {
        ml_name: cstr("remove_node"),
        ml_meth: Some(graph_remove_node),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[2] = PyMethodDef {
        ml_name: cstr("has_node"),
        ml_meth: Some(graph_has_node),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[3] = PyMethodDef {
        ml_name: cstr("nodes"),
        ml_meth: Some(graph_nodes),
        ml_flags: METH_NOARGS,
        ml_doc: ptr::null(),
    };
    METHODS[4] = PyMethodDef {
        ml_name: cstr("add_edge"),
        ml_meth: Some(graph_add_edge),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[5] = PyMethodDef {
        ml_name: cstr("remove_edge"),
        ml_meth: Some(graph_remove_edge),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[6] = PyMethodDef {
        ml_name: cstr("has_edge"),
        ml_meth: Some(graph_has_edge),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[7] = PyMethodDef {
        ml_name: cstr("edges"),
        ml_meth: Some(graph_edges),
        ml_flags: METH_NOARGS,
        ml_doc: ptr::null(),
    };
    METHODS[8] = PyMethodDef {
        ml_name: cstr("edge_weight"),
        ml_meth: Some(graph_edge_weight),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[9] = PyMethodDef {
        ml_name: cstr("neighbors"),
        ml_meth: Some(graph_neighbors),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[10] = PyMethodDef {
        ml_name: cstr("neighbors_weighted"),
        ml_meth: Some(graph_neighbors_weighted),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[11] = PyMethodDef {
        ml_name: cstr("degree"),
        ml_meth: Some(graph_degree),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[12] = PyMethodDef {
        ml_name: cstr("bfs"),
        ml_meth: Some(graph_bfs),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[13] = PyMethodDef {
        ml_name: cstr("dfs"),
        ml_meth: Some(graph_dfs),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[14] = PyMethodDef {
        ml_name: cstr("is_connected"),
        ml_meth: Some(graph_is_connected),
        ml_flags: METH_NOARGS,
        ml_doc: ptr::null(),
    };
    METHODS[15] = PyMethodDef {
        ml_name: cstr("connected_components"),
        ml_meth: Some(graph_connected_components),
        ml_flags: METH_NOARGS,
        ml_doc: ptr::null(),
    };
    METHODS[16] = PyMethodDef {
        ml_name: cstr("has_cycle"),
        ml_meth: Some(graph_has_cycle),
        ml_flags: METH_NOARGS,
        ml_doc: ptr::null(),
    };
    METHODS[17] = PyMethodDef {
        ml_name: cstr("shortest_path"),
        ml_meth: Some(graph_shortest_path),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[18] = PyMethodDef {
        ml_name: cstr("minimum_spanning_tree"),
        ml_meth: Some(graph_minimum_spanning_tree),
        ml_flags: METH_NOARGS,
        ml_doc: ptr::null(),
    };
    METHODS[19] = method_def_sentinel();

    static mut SLOTS: [PyType_Slot; 7] = [PyType_Slot {
        slot: 0,
        pfunc: ptr::null_mut(),
    }; 7];
    SLOTS[0] = PyType_Slot {
        slot: PY_TP_NEW,
        pfunc: graph_new as *mut c_void,
    };
    SLOTS[1] = PyType_Slot {
        slot: PY_TP_DEALLOC,
        pfunc: graph_dealloc as *mut c_void,
    };
    SLOTS[2] = PyType_Slot {
        slot: PY_TP_METHODS,
        pfunc: (&raw mut METHODS) as *mut c_void,
    };
    SLOTS[3] = PyType_Slot {
        slot: PY_TP_REPR,
        pfunc: graph_repr as *mut c_void,
    };
    SLOTS[4] = PyType_Slot {
        slot: PY_SQ_LENGTH,
        pfunc: graph_sq_length as *mut c_void,
    };
    SLOTS[5] = PyType_Slot {
        slot: PY_SQ_CONTAINS,
        pfunc: graph_sq_contains as *mut c_void,
    };
    SLOTS[6] = type_slot_sentinel();

    static mut SPEC: PyType_Spec = PyType_Spec {
        name: ptr::null(),
        basicsize: 0,
        itemsize: 0,
        flags: 0,
        slots: ptr::null_mut(),
    };
    SPEC.name = cstr("graph_native.Graph");
    SPEC.basicsize = std::mem::size_of::<GraphObject>() as c_int;
    SPEC.flags = PY_TPFLAGS_DEFAULT;
    SPEC.slots = (&raw mut SLOTS) as *mut PyType_Slot;

    let type_obj = PyType_FromSpec(&raw mut SPEC);
    if type_obj.is_null() {
        return ptr::null_mut();
    }

    static mut MODULE_DEF: PyModuleDef = PyModuleDef {
        m_base: PyModuleDef_Base {
            ob_base: [0; std::mem::size_of::<usize>() * 2],
            m_init: None,
            m_index: 0,
            m_copy: ptr::null_mut(),
        },
        m_name: ptr::null(),
        m_doc: ptr::null(),
        m_size: -1,
        m_methods: ptr::null_mut(),
        m_slots: ptr::null_mut(),
        m_traverse: ptr::null_mut(),
        m_clear: ptr::null_mut(),
        m_free: ptr::null_mut(),
    };
    MODULE_DEF.m_name = cstr("graph_native");

    let module = PyModule_Create2(&raw mut MODULE_DEF, PYTHON_API_VERSION);
    if module.is_null() {
        return ptr::null_mut();
    }

    Py_IncRef(type_obj);
    module_add_object(module, "Graph", type_obj);

    NODE_NOT_FOUND_ERROR = new_exception("graph_native", "NodeNotFoundError", exception_class());
    EDGE_NOT_FOUND_ERROR = new_exception("graph_native", "EdgeNotFoundError", exception_class());

    Py_IncRef(NODE_NOT_FOUND_ERROR);
    Py_IncRef(EDGE_NOT_FOUND_ERROR);

    module_add_object(module, "NodeNotFoundError", NODE_NOT_FOUND_ERROR);
    module_add_object(module, "EdgeNotFoundError", EDGE_NOT_FOUND_ERROR);

    module
}
