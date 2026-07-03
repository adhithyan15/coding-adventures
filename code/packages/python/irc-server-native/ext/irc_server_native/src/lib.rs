//! `irc_server_native` — a Python C extension that embeds the all-Rust IRC
//! engine (`irc-net-reactor`) and exposes a tiny lifecycle control surface.
//!
//! ## Design: control surface only, no callbacks
//!
//! All IRC and TCP logic lives in Rust (`irc-net-reactor` on the home-grown
//! kqueue/epoll reactor).  Python's only job is to *launch and control* the
//! server, so this module exposes exactly five module-level functions over an
//! opaque [`PyCapsule`] handle:
//!
//! | function                       | meaning                                    |
//! |--------------------------------|--------------------------------------------|
//! | `server_new(host, port, name, motd, oper_password, max_connections)` | build + bind, return a capsule |
//! | `server_serve(capsule)`        | run the event loop, **releasing the GIL**  |
//! | `server_stop(capsule)`         | signal the loop to stop (callable from another thread) |
//! | `server_local_host(capsule)` / `server_local_port(capsule)` | the bound address |
//! | `server_running(capsule)`      | is the loop currently running?             |
//! | `server_dispose(capsule)`      | drop the engine (must be stopped first)    |
//!
//! Unlike `conduit`, there is **no per-request dispatch back into Python**, so
//! none of the `PyGILState_Ensure`/TSFN machinery is needed — just a single
//! `PyEval_SaveThread`/`PyEval_RestoreThread` around the blocking `serve()`.
//!
//! ## GIL discipline
//!
//! `server_serve` releases the GIL with `PyEval_SaveThread` before calling the
//! blocking `IrcReactorServer::serve()` and re-acquires it with
//! `PyEval_RestoreThread` afterward.  This lets a *different* Python thread call
//! `server_stop` (which only touches the thread-safe stop handle) while the
//! event loop runs — the standard "serve on one thread, stop from another"
//! pattern, identical to how `conduit` drives `web-core`.

use std::ffi::{c_char, c_long, c_void, CString};
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use irc_net_reactor::{IrcConfig, IrcReactorServer};
use python_bridge::{
    method_def_sentinel, py_false, py_none, py_true, runtime_error_class, str_from_py, str_to_py,
    vec_str_from_py, PyErr_Clear, PyErr_SetString, PyLong_FromLong, PyMethodDef, PyModuleDef,
    PyModuleDef_Base, PyModule_Create2, PyObjectPtr, PyTuple_GetItem, METH_VARARGS,
    PYTHON_API_VERSION,
};

// ─────────────────────────────────────────────────────────────────────────────
// C API symbols not re-exported by python-bridge — declared inline (stable ABI).
// ─────────────────────────────────────────────────────────────────────────────
//
//   PyEval_SaveThread    → release the GIL; returns a *mut PyThreadState.
//   PyEval_RestoreThread → re-acquire the GIL from a saved thread state.
//   PyCapsule_New        → wrap a raw C pointer with a destructor.
//   PyCapsule_GetPointer → recover the raw pointer from a capsule.
//   PyLong_AsLong        → Python int → C long (-1 + set error on failure).
//   PyErr_Occurred       → non-null if a Python exception is currently set.
#[allow(non_snake_case)]
extern "C" {
    fn PyEval_SaveThread() -> *mut c_void;
    fn PyEval_RestoreThread(state: *mut c_void);
    fn PyCapsule_New(
        pointer: *mut c_void,
        name: *const c_char,
        destructor: Option<unsafe extern "C" fn(PyObjectPtr)>,
    ) -> PyObjectPtr;
    fn PyCapsule_GetPointer(capsule: PyObjectPtr, name: *const c_char) -> *mut c_void;
    fn PyLong_AsLong(o: PyObjectPtr) -> c_long;
    fn PyErr_Occurred() -> PyObjectPtr;
}

/// The capsule's type name — `PyCapsule_GetPointer` checks it on every recovery,
/// so a capsule from a different module can never be mistaken for ours.
const CAPSULE_NAME: &[u8] = b"irc_server_native.server\0";

/// The boxed state behind the capsule.  `running` mirrors the engine's serve
/// state so `server_dispose` can refuse to drop a live server.
struct PyIrcServer {
    server: Option<IrcReactorServer>,
    running: Arc<AtomicBool>,
}

/// Called by Python when the capsule is garbage-collected: reconstruct and drop
/// the `Box`, which drops the `IrcReactorServer` (closing the listener).  Python
/// holds the GIL here, so this is safe.
unsafe extern "C" fn capsule_destructor(capsule: PyObjectPtr) {
    let ptr = PyCapsule_GetPointer(capsule, CAPSULE_NAME.as_ptr() as *const c_char);
    if !ptr.is_null() {
        drop(Box::from_raw(ptr as *mut PyIrcServer));
    }
}

/// Recover the `*mut PyIrcServer` from a capsule, setting a Python error and
/// returning null on mismatch.
unsafe fn get_state(capsule: PyObjectPtr) -> *mut PyIrcServer {
    let ptr = PyCapsule_GetPointer(capsule, CAPSULE_NAME.as_ptr() as *const c_char);
    if ptr.is_null() {
        // PyCapsule_GetPointer already set a TypeError on mismatch.
        return ptr::null_mut();
    }
    ptr as *mut PyIrcServer
}

/// Set a `RuntimeError` from a Rust string (falls back to a static message if
/// the string contains an interior NUL).
///
/// Clears any pending exception first: a too-short argument tuple leaves an
/// `IndexError` set by `PyTuple_GetItem`, and we want our `RuntimeError` to be
/// the single, clean exception the caller sees rather than chaining onto it.
unsafe fn set_runtime_error(message: String) {
    PyErr_Clear();
    let c_msg =
        CString::new(message).unwrap_or_else(|_| CString::new("irc_server_native error").unwrap());
    PyErr_SetString(runtime_error_class(), c_msg.as_ptr());
}

// ─────────────────────────────────────────────────────────────────────────────
// server_new(host, port, server_name, motd, oper_password, max_connections)
// ─────────────────────────────────────────────────────────────────────────────

unsafe extern "C" fn server_new(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let host_py = PyTuple_GetItem(args, 0);
    let port_py = PyTuple_GetItem(args, 1);
    let name_py = PyTuple_GetItem(args, 2);
    let motd_py = PyTuple_GetItem(args, 3);
    let oper_py = PyTuple_GetItem(args, 4);
    let max_py = PyTuple_GetItem(args, 5);

    if [host_py, port_py, name_py, motd_py, oper_py, max_py]
        .iter()
        .any(|p| p.is_null())
    {
        set_runtime_error(
            "server_new requires (host, port, server_name, motd, oper_password, max_connections)"
                .to_string(),
        );
        return ptr::null_mut();
    }

    let Some(host) = str_from_py(host_py) else {
        set_runtime_error("host must be a string".to_string());
        return ptr::null_mut();
    };
    let Some(server_name) = str_from_py(name_py) else {
        set_runtime_error("server_name must be a string".to_string());
        return ptr::null_mut();
    };
    let Some(oper_password) = str_from_py(oper_py) else {
        set_runtime_error("oper_password must be a string".to_string());
        return ptr::null_mut();
    };
    let Some(motd) = vec_str_from_py(motd_py) else {
        set_runtime_error("motd must be a list of strings".to_string());
        return ptr::null_mut();
    };

    // Port: validate the 0–65535 range explicitly.  PyLong_AsLong returns -1 on
    // a non-integer and sets an error, which we surface rather than mask.
    let port_raw = PyLong_AsLong(port_py);
    if !PyErr_Occurred().is_null() {
        return ptr::null_mut();
    }
    if !(0..=65535).contains(&port_raw) {
        set_runtime_error(format!("port must be between 0 and 65535, got {port_raw}"));
        return ptr::null_mut();
    }
    let port = port_raw as u16;

    let max_raw = PyLong_AsLong(max_py);
    if !PyErr_Occurred().is_null() {
        return ptr::null_mut();
    }
    if max_raw < 1 {
        set_runtime_error(format!("max_connections must be >= 1, got {max_raw}"));
        return ptr::null_mut();
    }
    let max_connections = max_raw as usize;

    let config = IrcConfig {
        host,
        port,
        server_name,
        motd,
        oper_password,
        max_connections,
    };

    let server = match IrcReactorServer::bind(config) {
        Ok(server) => server,
        Err(err) => {
            set_runtime_error(format!("failed to bind IRC server: {err}"));
            return ptr::null_mut();
        }
    };

    let state = Box::new(PyIrcServer {
        server: Some(server),
        running: Arc::new(AtomicBool::new(false)),
    });
    PyCapsule_New(
        Box::into_raw(state) as *mut c_void,
        CAPSULE_NAME.as_ptr() as *const c_char,
        Some(capsule_destructor),
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// server_serve(capsule) — blocks, releasing the GIL
// ─────────────────────────────────────────────────────────────────────────────

unsafe extern "C" fn server_serve(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let capsule = PyTuple_GetItem(args, 0);
    if capsule.is_null() {
        set_runtime_error("server_serve(capsule)".to_string());
        return ptr::null_mut();
    }
    let state = get_state(capsule);
    if state.is_null() {
        return ptr::null_mut();
    }

    // Hold an OWNED clone of the engine across the blocking call rather than a
    // raw pointer into the boxed state.  `IrcReactorServer` is `Clone` over
    // `Arc`s, so this clone keeps the runtime alive even if another thread calls
    // `server_dispose` (which only drops the box's stored copy) while we serve —
    // closing a use-after-free window that a raw pointer would leave open.  We
    // also flip `running` to true *before* releasing the GIL, so `server_dispose`
    // (which runs only with the GIL held) always observes the running state and
    // refuses to dispose a live server.
    let server = match (*state).server.as_ref() {
        Some(server) => server.clone(),
        None => {
            set_runtime_error("server has been disposed".to_string());
            return ptr::null_mut();
        }
    };

    (*state).running.store(true, Ordering::SeqCst);
    let thread_state = PyEval_SaveThread();
    let result = server.serve();
    PyEval_RestoreThread(thread_state);
    (*state).running.store(false, Ordering::SeqCst);

    match result {
        Ok(()) => py_none(),
        Err(err) => {
            set_runtime_error(format!("IRC server error: {err}"));
            ptr::null_mut()
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// server_stop(capsule)
// ─────────────────────────────────────────────────────────────────────────────

unsafe extern "C" fn server_stop(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let capsule = PyTuple_GetItem(args, 0);
    if capsule.is_null() {
        set_runtime_error("server_stop(capsule)".to_string());
        return ptr::null_mut();
    }
    let state = get_state(capsule);
    if state.is_null() {
        return ptr::null_mut();
    }
    match (*state).server.as_ref() {
        Some(server) => {
            server.stop();
            py_none()
        }
        None => {
            set_runtime_error("server has been disposed".to_string());
            ptr::null_mut()
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// server_local_host / server_local_port / server_running / server_dispose
// ─────────────────────────────────────────────────────────────────────────────

unsafe extern "C" fn server_local_host(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let capsule = PyTuple_GetItem(args, 0);
    if capsule.is_null() {
        set_runtime_error("server_local_host(capsule)".to_string());
        return ptr::null_mut();
    }
    let state = get_state(capsule);
    if state.is_null() {
        return ptr::null_mut();
    }
    match (*state).server.as_ref() {
        Some(server) => str_to_py(&server.local_addr().ip().to_string()),
        None => {
            set_runtime_error("server has been disposed".to_string());
            ptr::null_mut()
        }
    }
}

unsafe extern "C" fn server_local_port(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let capsule = PyTuple_GetItem(args, 0);
    if capsule.is_null() {
        set_runtime_error("server_local_port(capsule)".to_string());
        return ptr::null_mut();
    }
    let state = get_state(capsule);
    if state.is_null() {
        return ptr::null_mut();
    }
    match (*state).server.as_ref() {
        Some(server) => PyLong_FromLong(server.local_addr().port() as c_long),
        None => {
            set_runtime_error("server has been disposed".to_string());
            ptr::null_mut()
        }
    }
}

unsafe extern "C" fn server_running(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let capsule = PyTuple_GetItem(args, 0);
    if capsule.is_null() {
        set_runtime_error("server_running(capsule)".to_string());
        return ptr::null_mut();
    }
    let state = get_state(capsule);
    if state.is_null() {
        return ptr::null_mut();
    }
    if (*state).running.load(Ordering::SeqCst) {
        py_true()
    } else {
        py_false()
    }
}

unsafe extern "C" fn server_dispose(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let capsule = PyTuple_GetItem(args, 0);
    if capsule.is_null() {
        set_runtime_error("server_dispose(capsule)".to_string());
        return ptr::null_mut();
    }
    let state = get_state(capsule);
    if state.is_null() {
        return ptr::null_mut();
    }
    if (*state).running.load(Ordering::SeqCst) {
        set_runtime_error("cannot dispose a running server; call stop() first".to_string());
        return ptr::null_mut();
    }
    // Drop the engine now (closing the listener) rather than waiting for GC.
    (*state).server.take();
    py_none()
}

// ─────────────────────────────────────────────────────────────────────────────
// Module definition
// ─────────────────────────────────────────────────────────────────────────────

static mut MODULE_METHODS: [PyMethodDef; 8] = [
    PyMethodDef {
        ml_name: c"server_new".as_ptr(),
        ml_meth: Some(server_new),
        ml_flags: METH_VARARGS,
        ml_doc:
            c"server_new(host, port, server_name, motd, oper_password, max_connections) -> capsule"
                .as_ptr(),
    },
    PyMethodDef {
        ml_name: c"server_serve".as_ptr(),
        ml_meth: Some(server_serve),
        ml_flags: METH_VARARGS,
        ml_doc: c"server_serve(capsule) -> None  # blocks; releases the GIL".as_ptr(),
    },
    PyMethodDef {
        ml_name: c"server_stop".as_ptr(),
        ml_meth: Some(server_stop),
        ml_flags: METH_VARARGS,
        ml_doc: c"server_stop(capsule) -> None".as_ptr(),
    },
    PyMethodDef {
        ml_name: c"server_local_host".as_ptr(),
        ml_meth: Some(server_local_host),
        ml_flags: METH_VARARGS,
        ml_doc: c"server_local_host(capsule) -> str".as_ptr(),
    },
    PyMethodDef {
        ml_name: c"server_local_port".as_ptr(),
        ml_meth: Some(server_local_port),
        ml_flags: METH_VARARGS,
        ml_doc: c"server_local_port(capsule) -> int".as_ptr(),
    },
    PyMethodDef {
        ml_name: c"server_running".as_ptr(),
        ml_meth: Some(server_running),
        ml_flags: METH_VARARGS,
        ml_doc: c"server_running(capsule) -> bool".as_ptr(),
    },
    PyMethodDef {
        ml_name: c"server_dispose".as_ptr(),
        ml_meth: Some(server_dispose),
        ml_flags: METH_VARARGS,
        ml_doc: c"server_dispose(capsule) -> None".as_ptr(),
    },
    method_def_sentinel(),
];

static mut MODULE_DEF: PyModuleDef = PyModuleDef {
    m_base: PyModuleDef_Base {
        ob_base: [0u8; std::mem::size_of::<usize>() * 2],
        m_init: None,
        m_index: 0,
        m_copy: ptr::null_mut(),
    },
    m_name: c"irc_server_native".as_ptr(),
    m_doc: c"Native control surface for the all-Rust irc-net-reactor IRC engine".as_ptr(),
    m_size: -1,
    m_methods: ptr::null_mut(), // set in PyInit_ below
    m_slots: ptr::null_mut(),
    m_traverse: ptr::null_mut(),
    m_clear: ptr::null_mut(),
    m_free: ptr::null_mut(),
};

/// Module initializer — Python calls this when the extension is imported.
///
/// # Safety
/// Called exactly once by the CPython import machinery with the GIL held.
#[no_mangle]
pub unsafe extern "C" fn PyInit_irc_server_native() -> PyObjectPtr {
    #[allow(static_mut_refs)]
    {
        MODULE_DEF.m_methods = MODULE_METHODS.as_mut_ptr();
    }
    PyModule_Create2(&raw mut MODULE_DEF, PYTHON_API_VERSION)
}
