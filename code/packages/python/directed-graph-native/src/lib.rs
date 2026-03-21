// lib.rs -- DirectedGraph Python extension using python-bridge
// =============================================================
//
// First native extension built on our zero-dependency python-bridge.
// Every C API call is visible, every type conversion is explicit.
//
// # Architecture
//
// 1. PyInit_directed_graph_native() creates the module
// 2. A PyTypeObject "DirectedGraph" is created via PyType_FromSpec
// 3. Each instance holds a pointer to a Rust Graph in its object body
// 4. Method calls extract the Graph pointer, call Rust, marshal the result
// 5. Custom exception classes are defined for CycleError, NodeNotFoundError, etc.

use std::ffi::{c_char, c_int, c_void, CString};
use std::ptr;

use directed_graph::graph::{Graph, GraphError};
use python_bridge::*;

// ---------------------------------------------------------------------------
// Slot numbers from CPython's typeslots.h (stable ABI)
// ---------------------------------------------------------------------------

const PY_TP_DEALLOC: c_int = 52;
const PY_TP_REPR: c_int = 66;
const PY_TP_METHODS: c_int = 64;
const PY_TP_NEW: c_int = 65;
const PY_SQ_LENGTH: c_int = 38;
const PY_SQ_CONTAINS: c_int = 37;

// ---------------------------------------------------------------------------
// Instance layout: GraphObject = PyObject_HEAD + graph pointer
// ---------------------------------------------------------------------------

#[repr(C)]
struct GraphObject {
    ob_refcnt: isize,
    ob_type: PyObjectPtr,
    graph: *mut Graph,
}

// ---------------------------------------------------------------------------
// Exception class globals
// ---------------------------------------------------------------------------

static mut CYCLE_ERROR: PyObjectPtr = ptr::null_mut();
static mut NODE_NOT_FOUND_ERROR: PyObjectPtr = ptr::null_mut();
static mut EDGE_NOT_FOUND_ERROR: PyObjectPtr = ptr::null_mut();

unsafe fn set_graph_error(err: GraphError) -> PyObjectPtr {
    match err {
        GraphError::CycleError => {
            set_error(CYCLE_ERROR, "graph contains a cycle");
        }
        GraphError::NodeNotFound(node) => {
            set_error(NODE_NOT_FOUND_ERROR, &format!("node not found: {}", node));
        }
        GraphError::EdgeNotFound(from, to) => {
            set_error(EDGE_NOT_FOUND_ERROR, &format!("edge not found: {} -> {}", from, to));
        }
        GraphError::SelfLoop(node) => {
            set_error(value_error_class(), &format!("self-loop not allowed: {}", node));
        }
    }
    ptr::null_mut()
}

// ---------------------------------------------------------------------------
// Graph access helpers
// ---------------------------------------------------------------------------

unsafe fn get_graph(slf: PyObjectPtr) -> &'static Graph {
    &*((slf as *mut GraphObject).read().graph)
}

unsafe fn get_graph_mut(slf: PyObjectPtr) -> &'static mut Graph {
    &mut *(*(slf as *mut GraphObject)).graph
}

// ---------------------------------------------------------------------------
// tp_new and tp_dealloc
// ---------------------------------------------------------------------------

extern "C" {
    fn PyType_GenericAlloc(type_obj: PyObjectPtr, nitems: isize) -> PyObjectPtr;
    fn PyObject_Free(ptr: *mut c_void);
}

unsafe extern "C" fn graph_new(
    type_obj: PyObjectPtr,
    _args: PyObjectPtr,
    _kwargs: PyObjectPtr,
) -> PyObjectPtr {
    let obj = PyType_GenericAlloc(type_obj, 0);
    if obj.is_null() {
        return ptr::null_mut();
    }
    (*(obj as *mut GraphObject)).graph = Box::into_raw(Box::new(Graph::new()));
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

// ---------------------------------------------------------------------------
// sq_length and sq_contains (for __len__ and __contains__)
// ---------------------------------------------------------------------------

unsafe extern "C" fn graph_sq_length(slf: PyObjectPtr) -> isize {
    get_graph(slf).size() as isize
}

unsafe extern "C" fn graph_sq_contains(slf: PyObjectPtr, key: PyObjectPtr) -> c_int {
    match str_from_py(key) {
        Some(node) => if get_graph(slf).has_node(&node) { 1 } else { 0 },
        None => -1,
    }
}

// ---------------------------------------------------------------------------
// Method implementations (all take self + args as PyObjectPtr)
// ---------------------------------------------------------------------------

unsafe extern "C" fn graph_add_node(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let node = match parse_arg_str(args, 0) { Some(s) => s, None => return ptr::null_mut() };
    get_graph_mut(slf).add_node(&node);
    py_none()
}

unsafe extern "C" fn graph_remove_node(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let node = match parse_arg_str(args, 0) { Some(s) => s, None => return ptr::null_mut() };
    match get_graph_mut(slf).remove_node(&node) {
        Ok(()) => py_none(),
        Err(e) => set_graph_error(e),
    }
}

unsafe extern "C" fn graph_has_node(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let node = match parse_arg_str(args, 0) { Some(s) => s, None => return ptr::null_mut() };
    bool_to_py(get_graph(slf).has_node(&node))
}

unsafe extern "C" fn graph_nodes(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    vec_str_to_py(&get_graph(slf).nodes())
}

unsafe extern "C" fn graph_add_edge(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let (from, to) = match parse_args_2str(args) { Some(p) => p, None => return ptr::null_mut() };
    match get_graph_mut(slf).add_edge(&from, &to) {
        Ok(()) => py_none(),
        Err(e) => set_graph_error(e),
    }
}

unsafe extern "C" fn graph_remove_edge(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let (from, to) = match parse_args_2str(args) { Some(p) => p, None => return ptr::null_mut() };
    match get_graph_mut(slf).remove_edge(&from, &to) {
        Ok(()) => py_none(),
        Err(e) => set_graph_error(e),
    }
}

unsafe extern "C" fn graph_has_edge(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let (from, to) = match parse_args_2str(args) { Some(p) => p, None => return ptr::null_mut() };
    bool_to_py(get_graph(slf).has_edge(&from, &to))
}

unsafe extern "C" fn graph_edges(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    vec_tuple2_str_to_py(&get_graph(slf).edges())
}

unsafe extern "C" fn graph_predecessors(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let node = match parse_arg_str(args, 0) { Some(s) => s, None => return ptr::null_mut() };
    match get_graph(slf).predecessors(&node) {
        Ok(v) => vec_str_to_py(&v),
        Err(e) => set_graph_error(e),
    }
}

unsafe extern "C" fn graph_successors(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let node = match parse_arg_str(args, 0) { Some(s) => s, None => return ptr::null_mut() };
    match get_graph(slf).successors(&node) {
        Ok(v) => vec_str_to_py(&v),
        Err(e) => set_graph_error(e),
    }
}

unsafe extern "C" fn graph_repr(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    let g = get_graph(slf);
    str_to_py(&format!("DirectedGraph(nodes={}, edges={})", g.size(), g.edges().len()))
}

unsafe extern "C" fn graph_topological_sort(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    match get_graph(slf).topological_sort() {
        Ok(v) => vec_str_to_py(&v),
        Err(e) => set_graph_error(e),
    }
}

unsafe extern "C" fn graph_has_cycle(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    bool_to_py(get_graph(slf).has_cycle())
}

unsafe extern "C" fn graph_transitive_closure(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let node = match parse_arg_str(args, 0) { Some(s) => s, None => return ptr::null_mut() };
    match get_graph(slf).transitive_closure(&node) {
        Ok(closure) => set_str_to_py(&closure),
        Err(e) => set_graph_error(e),
    }
}

unsafe extern "C" fn graph_affected_nodes(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let arg = PyTuple_GetItem(args, 0);
    let changed = match set_str_from_py(arg) { Some(s) => s, None => return ptr::null_mut() };
    set_str_to_py(&get_graph(slf).affected_nodes(&changed))
}

unsafe extern "C" fn graph_independent_groups(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    match get_graph(slf).independent_groups() {
        Ok(v) => vec_vec_str_to_py(&v),
        Err(e) => set_graph_error(e),
    }
}

// ---------------------------------------------------------------------------
// Leaked CString helper (method tables need static lifetime)
// ---------------------------------------------------------------------------

fn cstr(s: &str) -> *const c_char {
    CString::new(s).expect("no NUL").into_raw()
}

// ---------------------------------------------------------------------------
// Module init
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn PyInit_directed_graph_native() -> PyObjectPtr {
    // -- Method table (leaked — must live forever) --------------------------
    static mut METHODS: [PyMethodDef; 16] = [
        PyMethodDef { ml_name: ptr::null(), ml_meth: None, ml_flags: 0, ml_doc: ptr::null() }; 16
    ];

    // We initialize at runtime because CString can't be const.
    METHODS[0] = PyMethodDef { ml_name: cstr("add_node"), ml_meth: Some(graph_add_node), ml_flags: METH_VARARGS, ml_doc: ptr::null() };
    METHODS[1] = PyMethodDef { ml_name: cstr("remove_node"), ml_meth: Some(graph_remove_node), ml_flags: METH_VARARGS, ml_doc: ptr::null() };
    METHODS[2] = PyMethodDef { ml_name: cstr("has_node"), ml_meth: Some(graph_has_node), ml_flags: METH_VARARGS, ml_doc: ptr::null() };
    METHODS[3] = PyMethodDef { ml_name: cstr("nodes"), ml_meth: Some(graph_nodes), ml_flags: METH_NOARGS, ml_doc: ptr::null() };
    METHODS[4] = PyMethodDef { ml_name: cstr("add_edge"), ml_meth: Some(graph_add_edge), ml_flags: METH_VARARGS, ml_doc: ptr::null() };
    METHODS[5] = PyMethodDef { ml_name: cstr("remove_edge"), ml_meth: Some(graph_remove_edge), ml_flags: METH_VARARGS, ml_doc: ptr::null() };
    METHODS[6] = PyMethodDef { ml_name: cstr("has_edge"), ml_meth: Some(graph_has_edge), ml_flags: METH_VARARGS, ml_doc: ptr::null() };
    METHODS[7] = PyMethodDef { ml_name: cstr("edges"), ml_meth: Some(graph_edges), ml_flags: METH_NOARGS, ml_doc: ptr::null() };
    METHODS[8] = PyMethodDef { ml_name: cstr("predecessors"), ml_meth: Some(graph_predecessors), ml_flags: METH_VARARGS, ml_doc: ptr::null() };
    METHODS[9] = PyMethodDef { ml_name: cstr("successors"), ml_meth: Some(graph_successors), ml_flags: METH_VARARGS, ml_doc: ptr::null() };
    METHODS[10] = PyMethodDef { ml_name: cstr("topological_sort"), ml_meth: Some(graph_topological_sort), ml_flags: METH_NOARGS, ml_doc: ptr::null() };
    METHODS[11] = PyMethodDef { ml_name: cstr("has_cycle"), ml_meth: Some(graph_has_cycle), ml_flags: METH_NOARGS, ml_doc: ptr::null() };
    METHODS[12] = PyMethodDef { ml_name: cstr("transitive_closure"), ml_meth: Some(graph_transitive_closure), ml_flags: METH_VARARGS, ml_doc: ptr::null() };
    METHODS[13] = PyMethodDef { ml_name: cstr("affected_nodes"), ml_meth: Some(graph_affected_nodes), ml_flags: METH_VARARGS, ml_doc: ptr::null() };
    METHODS[14] = PyMethodDef { ml_name: cstr("independent_groups"), ml_meth: Some(graph_independent_groups), ml_flags: METH_NOARGS, ml_doc: ptr::null() };
    METHODS[15] = method_def_sentinel();

    // -- Type slots ---------------------------------------------------------
    static mut SLOTS: [PyType_Slot; 7] = [
        PyType_Slot { slot: 0, pfunc: ptr::null_mut() }; 7
    ];

    SLOTS[0] = PyType_Slot { slot: PY_TP_NEW, pfunc: graph_new as *mut c_void };
    SLOTS[1] = PyType_Slot { slot: PY_TP_DEALLOC, pfunc: graph_dealloc as *mut c_void };
    SLOTS[2] = PyType_Slot { slot: PY_TP_METHODS, pfunc: (&raw mut METHODS) as *mut c_void };
    SLOTS[3] = PyType_Slot { slot: PY_TP_REPR, pfunc: graph_repr as *mut c_void };
    SLOTS[4] = PyType_Slot { slot: PY_SQ_LENGTH, pfunc: graph_sq_length as *mut c_void };
    SLOTS[5] = PyType_Slot { slot: PY_SQ_CONTAINS, pfunc: graph_sq_contains as *mut c_void };
    SLOTS[6] = type_slot_sentinel();

    // -- Type spec ----------------------------------------------------------
    static mut SPEC: PyType_Spec = PyType_Spec {
        name: ptr::null(),
        basicsize: 0,
        itemsize: 0,
        flags: 0,
        slots: ptr::null_mut(),
    };

    SPEC.name = cstr("directed_graph_native.DirectedGraph");
    SPEC.basicsize = std::mem::size_of::<GraphObject>() as c_int;
    SPEC.flags = PY_TPFLAGS_DEFAULT;
    SPEC.slots = (&raw mut SLOTS) as *mut PyType_Slot;

    let type_obj = PyType_FromSpec(&raw mut SPEC);
    if type_obj.is_null() {
        return ptr::null_mut();
    }

    // -- Module definition --------------------------------------------------
    static mut MODULE_DEF: PyModuleDef = PyModuleDef {
        m_base: PyModuleDef_Base {
            ob_base: [0; std::mem::size_of::<usize>() * 4],
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
    MODULE_DEF.m_name = cstr("directed_graph_native");

    let module = PyModule_Create2(&raw mut MODULE_DEF, PYTHON_API_VERSION);
    if module.is_null() {
        return ptr::null_mut();
    }

    // -- Add class to module ------------------------------------------------
    Py_IncRef(type_obj);
    module_add_object(module, "DirectedGraph", type_obj);

    // -- Create exception classes -------------------------------------------
    CYCLE_ERROR = new_exception("directed_graph_native", "CycleError", exception_class());
    NODE_NOT_FOUND_ERROR = new_exception("directed_graph_native", "NodeNotFoundError", exception_class());
    EDGE_NOT_FOUND_ERROR = new_exception("directed_graph_native", "EdgeNotFoundError", exception_class());

    Py_IncRef(CYCLE_ERROR);
    Py_IncRef(NODE_NOT_FOUND_ERROR);
    Py_IncRef(EDGE_NOT_FOUND_ERROR);

    module_add_object(module, "CycleError", CYCLE_ERROR);
    module_add_object(module, "NodeNotFoundError", NODE_NOT_FOUND_ERROR);
    module_add_object(module, "EdgeNotFoundError", EDGE_NOT_FOUND_ERROR);

    module
}
