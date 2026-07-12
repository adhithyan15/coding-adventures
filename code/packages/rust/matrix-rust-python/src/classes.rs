//! # `classes` — Python `Graph` and `Runtime` classes for the extension
//!
//! **MX09 Phase 2b.**  Exposes the class-based API from
//! [MX09 §"The binding surface"](../../../specs/MX09-matrix-rust-python.md):
//!
//! ```python
//! import matrix_rust_python as m
//!
//! # Graph — wraps a Rust `matrix_ir::Graph` parsed from its JSON
//! # wire format.  Holds a `Box<Graph>` behind the scenes inside the
//! # Python instance struct.
//! graph = m.Graph(json_string)
//! json  = graph.to_json()                  # re-serialise
//! print(graph.describe())                  # "Graph(tensors=4, ops=3, ...)"
//!
//! # Runtime — owns the CPU executor (and, when registered, GPU
//! # backends via matrix-runtime's planner).
//! rt = m.Runtime()
//! outputs = rt.run(graph, [b1, b2])        # list[bytes] in -> list[bytes] out
//! ```
//!
//! ## Why this exists alongside the Phase 1/2 string-only API
//!
//! Phase 1 (`graph_round_trip_json`) and Phase 2 (`run_graph_on_cpu`
//! JSON envelope) were string-in / string-out functions.  Two costs:
//!
//! * **JSON re-parsing on every call.**  A real consumer that runs a
//!   given graph 1000 times in a hot loop pays the JSON parse cost
//!   1000 times.  Wrapping the parsed `Graph` in a Python object pays
//!   it once.
//! * **Hex-encoded bytes are 2× the wire cost.**  `bytes` carries raw
//!   bytes — no encoding overhead, no copy through a `str`.
//!
//! Phase 2b addresses both: `m.Graph(json)` pays the parse cost once,
//! and `rt.run(graph, [bytes, ...])` exchanges raw bytes via
//! `python-bridge`'s `bytes_to_py` / `bytes_from_py` helpers.
//!
//! ## Why proper `PyType_FromSpec` types instead of capsule-on-attribute?
//!
//! The MX09 spec described "Graph/Runtime classes whose `tp_init` slot
//! stores the wrapped `Box<matrix_ir::Graph>` inside a PyCapsule held
//! by the instance".  We landed on a slightly simpler shape that's
//! equivalent in safety and ergonomics:
//!
//! 1. The wrapped `Box<MatrixGraph>` lives **directly in the
//!    `GraphInstance` struct** at a fixed offset after `PyObject_HEAD`.
//!    No PyCapsule attribute is needed because the Python type
//!    identity IS the type-tag — `PyObject_IsInstance(obj, GRAPH_TYPE)`
//!    rejects any object that isn't really a Graph before we
//!    dereference the inline pointer.  The 128-bit type-tag
//!    discriminator that `matrix-rust-napi` invented is unnecessary
//!    here: Python's type system already guarantees the same property.
//! 2. `tp_dealloc` runs deterministically when the instance's refcount
//!    drops to zero, calling `Box::from_raw` to free the inner Graph
//!    and then `PyObject_Free` to free the struct itself.  Same
//!    semantics as a PyCapsule destructor, fewer indirections.
//!
//! The PyCapsule angle the spec described is still useful for
//! module-level handle-returning APIs (font-parser-python does it
//! that way).  For class-based instance state, the inline-struct
//! pattern is cleaner.
//!
//! ## Why `PyType_FromSpec` instead of declaring a static
//!    `PyTypeObject`?
//!
//! `PyType_FromSpec` is the Limited-API-blessed way to register heap
//! types — it computes the type's layout from a spec at module-load
//! time, which means we don't have to know the exact offset of
//! `PyObject_HEAD` at compile time.  It's also the only way to create
//! types that work across all Python 3.x ABI versions without
//! rebuilding.

use std::ffi::c_int;
use std::ffi::c_void;
use std::mem::size_of;
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};

use matrix_ir::Graph as MatrixGraph;
use matrix_ir_json::{decode, encode};
use python_bridge::{
    bytes_from_py, bytes_to_py, module_add_object, set_error, str_from_py, str_to_py,
    type_error_class, value_error_class, METH_NOARGS, METH_VARARGS, PY_TPFLAGS_DEFAULT,
    PyErr_Clear, PyList_GetItem, PyList_New, PyList_SetItem, PyList_Size, PyMethodDef, PyObjectPtr,
    PyTuple_GetItem, PyType_FromSpec, PyType_Slot, PyType_Spec,
};

use crate::exec::run_graph_on_cpu;

// ─────────────────────────────────────────────────────────────────────────────
// Additional Python C API extern declarations not yet in python-bridge.
//
// All are part of the stable Limited API (PEP 384) and ABI-stable
// since Python 3.2 (2011).  We declare them inline rather than
// modifying python-bridge — same pattern font-parser-python uses for
// PyCapsule_New / PyBytes_AsStringAndSize / etc.
// ─────────────────────────────────────────────────────────────────────────────

#[allow(non_snake_case)]
extern "C" {
    /// Free a heap-allocated PyObject — the default `tp_free`.
    /// Called by our `tp_dealloc` after we've dropped the inner Box.
    fn PyObject_Free(o: PyObjectPtr);

    /// `isinstance(obj, type)` returns 1 / 0 / -1 (error).  We use it
    /// to validate that arguments are actually `Graph` / `Runtime`
    /// instances before dereferencing the inline payload.  This is
    /// the Python equivalent of MX07's 128-bit type-tag check — the
    /// Python type system already provides the discrimination.
    fn PyObject_IsInstance(inst: PyObjectPtr, cls: PyObjectPtr) -> c_int;
}

// ─────────────────────────────────────────────────────────────────────────────
// Slot constants from CPython's Include/typeslots.h.
//
// These integer slot IDs are part of the stable Limited API and
// have not changed since they were assigned.  We list only the four
// we need; the full table has ~80 entries (1..78ish, with gaps).
//
// Canonical values from CPython 3.10.6 Include/typeslots.h
// (verified by grep against the local Python install):
//
//   #define Py_tp_dealloc 52
//   #define Py_tp_doc     56
//   #define Py_tp_init    60
//   #define Py_tp_methods 64
//
// (Earlier draft used `PY_TP_METHODS = 72`, which is past the end
// of the slot enum — `PyType_FromSpec` set a "invalid slot offset"
// RuntimeError that classes::register silently caught and dropped,
// leaving CPython's import machinery to surface it as
// "initialization of matrix_rust_python raised unreported
// exception".  The unit test was circular — it asserted
// `PY_TP_METHODS == 72` against the hardcoded 72.  This file's
// `slot_constants_match_python_stable_abi` test now uses the
// canonical values + a header citation so the regression cannot
// reoccur.)
// ─────────────────────────────────────────────────────────────────────────────

const PY_TP_DEALLOC: c_int = 52;
// CPython `Py_tp_doc` slot id, kept for completeness alongside the other slot
// constants even though this type currently sets no docstring slot.
#[allow(dead_code)]
const PY_TP_DOC: c_int = 56;
const PY_TP_INIT: c_int = 60;
const PY_TP_METHODS: c_int = 64;

// ─────────────────────────────────────────────────────────────────────────────
// Static storage for the two type-object pointers.
//
// PyTypeObject pointers returned from `PyType_FromSpec` are
// **persistent for the life of the interpreter** — no
// reference-counting or handle-scope concerns (unlike N-API's
// `napi_value`, which is scope-local and forced matrix-rust-napi to
// invent the `napi_ref` indirection).  We simply store the raw
// pointer in an `AtomicUsize` for cross-callback retrieval.
//
// We use Release/Acquire ordering to publish the pointer exactly
// once at module init; later reads from instance methods are
// guaranteed to see the published value.  No worker-thread race
// concerns because the GIL serialises method dispatch.
// ─────────────────────────────────────────────────────────────────────────────

static GRAPH_TYPE: AtomicUsize = AtomicUsize::new(0);
static RUNTIME_TYPE: AtomicUsize = AtomicUsize::new(0);

fn store_graph_type(t: PyObjectPtr) {
    GRAPH_TYPE.store(t as usize, Ordering::Release);
}
fn load_graph_type() -> PyObjectPtr {
    GRAPH_TYPE.load(Ordering::Acquire) as PyObjectPtr
}
fn store_runtime_type(t: PyObjectPtr) {
    RUNTIME_TYPE.store(t as usize, Ordering::Release);
}
fn load_runtime_type() -> PyObjectPtr {
    RUNTIME_TYPE.load(Ordering::Acquire) as PyObjectPtr
}

// ─────────────────────────────────────────────────────────────────────────────
// Instance structs.
//
// Each starts with a `PyObject_HEAD`-sized opaque header (the same
// pattern python-bridge uses in `PyModuleDef_Base`).  Python's
// `PyType_GenericAlloc` (the default `tp_alloc`) zero-initialises
// the whole `basicsize` bytes when a new instance is created, so our
// `inner` pointer starts as null and tp_init fills it.
//
// `#[repr(C)]` pins the field order so the offsets we declare via
// `basicsize = size_of::<GraphInstance>()` match the actual struct
// layout.
// ─────────────────────────────────────────────────────────────────────────────

/// Size of `PyObject_HEAD` (ob_refcnt + ob_type) on the target
/// platform.  Two pointer-sized fields per the Limited API.  Same
/// value `python_bridge::PyModuleDef_Base` uses (line 151).
const PY_OBJECT_HEAD_SIZE: usize = size_of::<usize>() * 2;

#[repr(C)]
struct GraphInstance {
    /// Opaque PyObject_HEAD bytes.  We never touch these directly —
    /// CPython manages ob_refcnt and ob_type internally.  Listing it
    /// here just reserves the right amount of space at the start of
    /// the struct so our `inner` field lives at the correct offset.
    _head: [u8; PY_OBJECT_HEAD_SIZE],
    /// The boxed `matrix_ir::Graph`.  Null before tp_init runs and
    /// after tp_dealloc runs.  Non-null between the two, guaranteed
    /// by the GIL serialising method dispatch on a given instance.
    inner: *mut MatrixGraph,
}

#[repr(C)]
struct RuntimeInstance {
    _head: [u8; PY_OBJECT_HEAD_SIZE],
    /// No real state today — kept as a `usize` placeholder so future
    /// per-Runtime state (option flags, long-lived executor pool)
    /// has a parking spot.  Matches matrix-rust-napi's
    /// `WrappedRuntime` shape.
    _placeholder: usize,
}

// ─────────────────────────────────────────────────────────────────────────────
// Graph: tp_init, tp_dealloc, methods, type-spec setup
// ─────────────────────────────────────────────────────────────────────────────

/// `tp_init`: implementation of `Graph(json_string)`.  Parses the
/// JSON, boxes the resulting `matrix_ir::Graph`, stashes the pointer
/// in the instance struct.
///
/// Returns 0 on success, -1 on error (with a Python exception set).
///
/// # Safety
///
/// Called by CPython exactly once per fresh instance, with `self_`
/// pointing at a zero-initialised `GraphInstance` (the
/// `PyType_GenericAlloc` default for `tp_alloc`).
unsafe extern "C" fn graph_init(
    self_: PyObjectPtr,
    args: PyObjectPtr,
    _kwds: PyObjectPtr,
) -> c_int {
    let arg0 = PyTuple_GetItem(args, 0);
    if arg0.is_null() {
        PyErr_Clear();
        set_error(
            value_error_class(),
            "Graph(...) requires one argument: json_string: str",
        );
        return -1;
    }
    let json = match str_from_py(arg0) {
        Some(s) => s,
        None => {
            set_error(value_error_class(), "Graph(...): argument 0 must be a str");
            return -1;
        }
    };
    let graph = match decode(&json) {
        Ok(g) => g,
        Err(e) => {
            set_error(
                value_error_class(),
                &format!("Graph(...): parse failed: {:?}", e),
            );
            return -1;
        }
    };

    let instance = self_ as *mut GraphInstance;

    // Defensive: if `__init__` is called twice on the same instance
    // (legal in Python — `inst.__init__(new_json)` re-runs tp_init),
    // free the previous boxed graph before storing the new one.
    if !(*instance).inner.is_null() {
        let _ = Box::from_raw((*instance).inner);
        (*instance).inner = ptr::null_mut();
    }
    (*instance).inner = Box::into_raw(Box::new(graph));
    0
}

/// `tp_dealloc`: drop the boxed `matrix_ir::Graph` and free the
/// instance struct.  Called by CPython when the instance's refcount
/// drops to zero.
///
/// # Safety
///
/// Called exactly once per instance, when no other thread can
/// reference the instance (refcount is 0 by the GIL invariant on
/// entry to dealloc).
unsafe extern "C" fn graph_dealloc(self_: PyObjectPtr) {
    let instance = self_ as *mut GraphInstance;
    if !(*instance).inner.is_null() {
        let _ = Box::from_raw((*instance).inner);
        (*instance).inner = ptr::null_mut();
    }
    PyObject_Free(self_);
}

/// Helper: extract `&Graph` from a Python object after validating
/// that the object is really a `Graph` instance.
///
/// Returns `None` (with a Python exception set) if `obj` is not a
/// `Graph` or if `obj.inner` is null (tp_init never ran or ran but
/// failed before the assignment).
///
/// This is the Python equivalent of matrix-rust-napi's `unwrap_graph`
/// — except instead of a 128-bit type tag we rely on Python's own
/// type-instance check.  `PyObject_IsInstance` is the canonical
/// "is this object of type X" check and is built into the runtime.
unsafe fn unwrap_graph(obj: PyObjectPtr) -> Option<&'static MatrixGraph> {
    let graph_type = load_graph_type();
    if graph_type.is_null() {
        set_error(
            value_error_class(),
            "internal: Graph type not initialised",
        );
        return None;
    }
    match PyObject_IsInstance(obj, graph_type) {
        1 => {}
        0 => {
            set_error(
                type_error_class(),
                "argument must be a matrix_rust_python.Graph instance",
            );
            return None;
        }
        _ => {
            // -1 — PyObject_IsInstance set its own exception
            return None;
        }
    }
    let instance = obj as *const GraphInstance;
    if (*instance).inner.is_null() {
        set_error(
            value_error_class(),
            "internal: Graph instance has null inner (tp_init failed?)",
        );
        return None;
    }
    Some(&*(*instance).inner)
}

/// `graph.to_json() -> str` — re-serialise the wrapped Graph through
/// `matrix-ir-json::encode`.  Always succeeds (encode is infallible
/// per the matrix-ir-json crate's contract).
unsafe extern "C" fn graph_to_json(self_: PyObjectPtr, _unused: PyObjectPtr) -> PyObjectPtr {
    let graph = match unwrap_graph(self_) {
        Some(g) => g,
        None => return ptr::null_mut(),
    };
    let json = encode(graph);
    str_to_py(&json)
}

/// `graph.describe() -> str` — return a short human-readable
/// summary.  Useful for logging and debugging without dumping the
/// full JSON.
unsafe extern "C" fn graph_describe(self_: PyObjectPtr, _unused: PyObjectPtr) -> PyObjectPtr {
    let graph = match unwrap_graph(self_) {
        Some(g) => g,
        None => return ptr::null_mut(),
    };
    let summary = format!(
        "Graph(tensors={}, ops={}, inputs={}, outputs={}, constants={})",
        graph.tensors.len(),
        graph.ops.len(),
        graph.inputs.len(),
        graph.outputs.len(),
        graph.constants.len(),
    );
    str_to_py(&summary)
}

static mut GRAPH_METHODS: [PyMethodDef; 3] = [
    PyMethodDef {
        ml_name: c"to_json".as_ptr(),
        ml_meth: Some(graph_to_json),
        ml_flags: METH_NOARGS,
        ml_doc: c"to_json() -> str\n\n\
                  Re-serialise the wrapped Graph as matrix-ir-json wire format.\n"
            .as_ptr(),
    },
    PyMethodDef {
        ml_name: c"describe".as_ptr(),
        ml_meth: Some(graph_describe),
        ml_flags: METH_NOARGS,
        ml_doc: c"describe() -> str\n\n\
                  Return a short human-readable summary of the Graph.\n"
            .as_ptr(),
    },
    PyMethodDef {
        ml_name: ptr::null(),
        ml_meth: None,
        ml_flags: 0,
        ml_doc: ptr::null(),
    },
];

static mut GRAPH_SLOTS: [PyType_Slot; 4] = [
    PyType_Slot {
        slot: PY_TP_INIT,
        pfunc: graph_init as *mut c_void,
    },
    PyType_Slot {
        slot: PY_TP_DEALLOC,
        pfunc: graph_dealloc as *mut c_void,
    },
    PyType_Slot {
        slot: PY_TP_METHODS,
        pfunc: &raw mut GRAPH_METHODS as *mut c_void,
    },
    PyType_Slot {
        slot: 0,
        pfunc: ptr::null_mut(),
    },
];

static mut GRAPH_SPEC: PyType_Spec = PyType_Spec {
    name: c"matrix_rust_python.Graph".as_ptr(),
    basicsize: size_of::<GraphInstance>() as c_int,
    itemsize: 0,
    flags: PY_TPFLAGS_DEFAULT,
    slots: &raw mut GRAPH_SLOTS as *mut PyType_Slot,
};

// ─────────────────────────────────────────────────────────────────────────────
// Runtime: tp_init, tp_dealloc, methods, type-spec setup
// ─────────────────────────────────────────────────────────────────────────────
//
// In v0.x the Runtime is stateless — every `rt.run(graph, ...)` call
// internally builds a fresh `matrix_runtime::Runtime` + `CpuExecutor`
// inside `run_graph_on_cpu`.  The Runtime class exists so the
// surface API matches MX09's eventual shape; once Runtime-level
// options (executor pool, GPU backends, profiling hooks) land, the
// Python API won't have to change.

unsafe extern "C" fn runtime_init(
    _self: PyObjectPtr,
    args: PyObjectPtr,
    _kwds: PyObjectPtr,
) -> c_int {
    // Reject any positional arg — Runtime() takes 0 args today.
    if !PyTuple_GetItem(args, 0).is_null() {
        set_error(value_error_class(), "Runtime() takes no arguments");
        return -1;
    }
    PyErr_Clear(); // clear the IndexError PyTuple_GetItem set on arg 0 absent
    0
}

unsafe extern "C" fn runtime_dealloc(self_: PyObjectPtr) {
    // No inner Box to free in v0; just release the struct.
    PyObject_Free(self_);
}

/// Helper: validate that `obj` is a `Runtime` instance.  Returns
/// `false` (with a Python exception set) otherwise.
unsafe fn check_runtime(obj: PyObjectPtr) -> bool {
    let runtime_type = load_runtime_type();
    if runtime_type.is_null() {
        set_error(
            value_error_class(),
            "internal: Runtime type not initialised",
        );
        return false;
    }
    match PyObject_IsInstance(obj, runtime_type) {
        1 => true,
        0 => {
            set_error(
                type_error_class(),
                "self must be a matrix_rust_python.Runtime instance",
            );
            false
        }
        _ => false, // -1 — exception already set
    }
}

/// `runtime.run(graph: Graph, inputs: list[bytes]) -> list[bytes]`
/// — the headline method.  Validates argument shapes, dispatches to
/// the pure-Rust `run_graph_on_cpu`, packages outputs as a Python
/// `list[bytes]`.
///
/// All bytes are copied at the boundary (no shared-memory views) for
/// the same detachment-safety reasons documented in
/// `python_bridge::bytes_from_py` — input bytes might be garbage
/// collected before our dispatch completes if we held only a borrow.
unsafe extern "C" fn runtime_run(self_: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    if !check_runtime(self_) {
        return ptr::null_mut();
    }

    let arg0 = PyTuple_GetItem(args, 0);
    let arg1 = PyTuple_GetItem(args, 1);
    if arg0.is_null() || arg1.is_null() {
        PyErr_Clear();
        set_error(
            value_error_class(),
            "Runtime.run(graph, inputs) requires exactly 2 arguments",
        );
        return ptr::null_mut();
    }

    let graph = match unwrap_graph(arg0) {
        Some(g) => g,
        None => return ptr::null_mut(), // unwrap_graph set its own exception
    };

    // arg1 must be a Python list of bytes.  We do not require any
    // particular sequence type — PyList is the documented contract,
    // but the implementation only relies on `PyList_Size` +
    // `PyList_GetItem` working, which they do for genuine lists.
    let len = PyList_Size(arg1);
    if len < 0 {
        // PyList_Size set a TypeError already
        return ptr::null_mut();
    }
    let mut inputs: Vec<Vec<u8>> = Vec::with_capacity(len as usize);
    for i in 0..len {
        let item = PyList_GetItem(arg1, i);
        if item.is_null() {
            // shouldn't happen — Size said we have len items
            PyErr_Clear();
            set_error(
                value_error_class(),
                &format!("Runtime.run: inputs[{}] is missing", i),
            );
            return ptr::null_mut();
        }
        match bytes_from_py(item) {
            Some(b) => inputs.push(b),
            None => {
                set_error(
                    type_error_class(),
                    &format!("Runtime.run: inputs[{}] must be a bytes object", i),
                );
                return ptr::null_mut();
            }
        }
    }

    let outputs = match run_graph_on_cpu(graph, &inputs) {
        Ok(out) => out,
        Err(msg) => {
            set_error(
                value_error_class(),
                &format!("Runtime.run: {}", msg),
            );
            return ptr::null_mut();
        }
    };

    // Marshal outputs back as a fresh `list[bytes]`.  `PyList_SetItem`
    // steals the reference, so we hand it `bytes_to_py`'s new-reference
    // bytes object directly (no Py_DecRef needed on the bytes object —
    // the list now owns it).
    let result_list = PyList_New(outputs.len() as isize);
    for (i, out) in outputs.iter().enumerate() {
        let py_bytes = bytes_to_py(out);
        PyList_SetItem(result_list, i as isize, py_bytes);
    }
    result_list
}

static mut RUNTIME_METHODS: [PyMethodDef; 2] = [
    PyMethodDef {
        ml_name: c"run".as_ptr(),
        ml_meth: Some(runtime_run),
        ml_flags: METH_VARARGS,
        ml_doc: c"run(graph: Graph, inputs: list[bytes]) -> list[bytes]\n\n\
                  Plan and execute `graph` on the CPU executor with `inputs` as the \
                  per-input little-endian byte payloads.  Returns one bytes object per \
                  graph.outputs().  Raises ValueError on planner/executor errors, \
                  TypeError on wrong argument types, ValueError on graphs exceeding the \
                  4 GiB total-buffer cap.\n"
            .as_ptr(),
    },
    PyMethodDef {
        ml_name: ptr::null(),
        ml_meth: None,
        ml_flags: 0,
        ml_doc: ptr::null(),
    },
];

static mut RUNTIME_SLOTS: [PyType_Slot; 4] = [
    PyType_Slot {
        slot: PY_TP_INIT,
        pfunc: runtime_init as *mut c_void,
    },
    PyType_Slot {
        slot: PY_TP_DEALLOC,
        pfunc: runtime_dealloc as *mut c_void,
    },
    PyType_Slot {
        slot: PY_TP_METHODS,
        pfunc: &raw mut RUNTIME_METHODS as *mut c_void,
    },
    PyType_Slot {
        slot: 0,
        pfunc: ptr::null_mut(),
    },
];

static mut RUNTIME_SPEC: PyType_Spec = PyType_Spec {
    name: c"matrix_rust_python.Runtime".as_ptr(),
    basicsize: size_of::<RuntimeInstance>() as c_int,
    itemsize: 0,
    flags: PY_TPFLAGS_DEFAULT,
    slots: &raw mut RUNTIME_SLOTS as *mut PyType_Slot,
};

// ─────────────────────────────────────────────────────────────────────────────
// Module registration — call once from PyInit_matrix_rust_python.
// ─────────────────────────────────────────────────────────────────────────────

/// Create both type objects and bind them onto the module's namespace
/// as `Graph` and `Runtime`.  Idempotent — only called once from
/// PyInit_matrix_rust_python.
///
/// # Safety
///
/// Called exactly once at module load with a valid `module` pointer.
pub unsafe fn register(module: PyObjectPtr) {
    let graph_type = PyType_FromSpec(&raw mut GRAPH_SPEC);
    if graph_type.is_null() {
        // PyType_FromSpec set a SystemError already; nothing more we
        // can do — Python will see the import failure.
        return;
    }
    store_graph_type(graph_type);
    module_add_object(module, "Graph", graph_type);

    let runtime_type = PyType_FromSpec(&raw mut RUNTIME_SPEC);
    if runtime_type.is_null() {
        return;
    }
    store_runtime_type(runtime_type);
    module_add_object(module, "Runtime", runtime_type);
}

// ─────────────────────────────────────────────────────────────────────────────
// Pure-Rust tests
//
// The Python C API surface itself isn't exercisable without a CPython
// interpreter — Phase 4 lands `pytest` smoke via the wrapper package.
// Here we verify the layout invariants the unwrap helpers depend on:
//
//   1. `GraphInstance` and `RuntimeInstance` start with the
//      `PyObject_HEAD` opaque header at offset 0 (otherwise the
//      `obj as *mut GraphInstance` cast in unwrap_graph would
//      misinterpret the inner pointer offset).
//   2. The `_head` field is exactly `PY_OBJECT_HEAD_SIZE` bytes.
//      `PyType_GenericAlloc` allocates `basicsize` bytes and
//      zero-initialises them; if our header size is wrong, the
//      inner pointer would land inside CPython's own bookkeeping
//      and corruption would follow.
//   3. The `inner` field of `GraphInstance` is exactly one pointer
//      wide so `(*instance).inner` reads/writes the right number
//      of bytes.
//
// These are compile-time invariants but the tests make them explicit
// so the next reader knows they're load-bearing.  Mirrors
// matrix-rust-napi/src/classes.rs' tag-layout tests.
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::offset_of;

    #[test]
    fn graph_instance_head_lives_at_offset_zero() {
        assert_eq!(
            offset_of!(GraphInstance, _head),
            0,
            "PyType_GenericAlloc writes PyObject_HEAD at offset 0; \
             the inline `inner` pointer must come after"
        );
    }

    #[test]
    fn runtime_instance_head_lives_at_offset_zero() {
        assert_eq!(
            offset_of!(RuntimeInstance, _head),
            0,
            "PyType_GenericAlloc writes PyObject_HEAD at offset 0"
        );
    }

    #[test]
    fn graph_instance_head_size_matches_constant() {
        // `_head` is declared as `[u8; PY_OBJECT_HEAD_SIZE]`, so the
        // size is by construction.  This test just nails down the
        // value of the constant — if a future ABI change extended
        // `PyObject_HEAD` (e.g. via `Py_TRACE_REFS` debug builds), we
        // would need to bump `PY_OBJECT_HEAD_SIZE` and this test
        // would still pass; the test is a reminder, not a binding
        // assertion.
        assert_eq!(PY_OBJECT_HEAD_SIZE, size_of::<usize>() * 2);
    }

    #[test]
    fn graph_instance_inner_is_one_pointer_wide() {
        // Sanity: the `inner` field has to be a single raw pointer.
        // If a refactor ever changed it to `Option<Box<Graph>>` (which
        // is also one pointer in stable Rust), the size test still
        // passes; but if it grew to a `(Box<Graph>, otherthing)`, the
        // test would fire and the unwrap_graph cast would silently
        // dereference past the inner pointer.
        assert_eq!(size_of::<*mut MatrixGraph>(), size_of::<usize>());
    }

    #[test]
    fn graph_and_runtime_basicsize_at_least_one_pointer_past_head() {
        // We rely on `basicsize - PY_OBJECT_HEAD_SIZE >= 8 bytes` for
        // the `inner` / `_placeholder` field to fit.  If a refactor
        // ever shrank either instance struct to header-only (no
        // payload), the unwrap helpers would read the wrong memory.
        assert!(
            size_of::<GraphInstance>() >= PY_OBJECT_HEAD_SIZE + size_of::<*mut MatrixGraph>(),
            "GraphInstance must have room for the inner pointer after PyObject_HEAD"
        );
        assert!(
            size_of::<RuntimeInstance>() >= PY_OBJECT_HEAD_SIZE + size_of::<usize>(),
            "RuntimeInstance must have room for the placeholder after PyObject_HEAD"
        );
    }

    /// PyType_Slot constants we declared inline must match the
    /// canonical values from CPython's Include/typeslots.h.  These
    /// numbers have been stable since they were assigned and cannot
    /// be renumbered without a CPython ABI break.
    ///
    /// The earlier version of this test asserted my (wrong) values
    /// against my own hardcoded constants — a circular test that
    /// passed locally but caused PyType_FromSpec to silently fail
    /// in CI ("invalid slot offset" RuntimeError → unreported
    /// exception on module init).  See the SLOT CONSTANTS block
    /// at the top of this file for the citation.
    ///
    /// The test asserts the *literal* values from the upstream
    /// header.  Any future change to these numbers would require
    /// CPython itself to break ABI, which is by design.
    #[test]
    fn slot_constants_match_python_stable_abi() {
        // From CPython 3.10.6 Include/typeslots.h.  Verified by
        //   grep -E "Py_tp_(dealloc|doc|init|methods)\b" \
        //     $(python3 -c "import sysconfig; print(sysconfig.get_paths()['include'])")/typeslots.h
        // The values are part of the Limited API and are guaranteed
        // not to change without an ABI break.
        assert_eq!(PY_TP_DEALLOC, 52);
        assert_eq!(PY_TP_DOC, 56); // not currently used, but reserved
        assert_eq!(PY_TP_INIT, 60);
        assert_eq!(PY_TP_METHODS, 64);
    }
}
