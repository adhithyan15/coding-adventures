# DS01 — FFI Bridges

## Overview

Every language runtime (CPython, CRuby, Node.js) exposes a C API for writing native extensions. Third-party frameworks like PyO3, Magnus, and napi-rs wrap these C APIs in layers of procedural macros, trait implementations, and generated code. The result is convenient but opaque: when something goes wrong, you are debugging 15,000-50,000 lines of framework internals rather than the 20-line C API call you actually needed.

This spec defines three bridge crates — `python-bridge`, `ruby-bridge`, and `node-bridge` — that wrap **only** the raw C API bindings each runtime provides. No macros. No trait magic. No code generation. Each bridge is 300-400 lines of explicit, greppable Rust that a developer can read top-to-bottom in one sitting.

### Why Not Use PyO3/Magnus/napi-rs?

Three reasons:

1. **Debuggability.** When a segfault occurs in a native extension, the stack trace should show your code calling `Py_INCREF`, not 14 layers of macro-generated trait dispatch. With a thin bridge, the call stack is shallow and every function is visible in the source.

2. **Comprehension.** The C APIs for Python, Ruby, and Node.js are well-documented, stable, and small. A developer who understands the bridge understands the actual FFI mechanism — not an abstraction over it. This is educational infrastructure, and hiding the mechanism defeats the purpose.

3. **Dependency weight.** PyO3 pulls in `proc-macro2`, `quote`, `syn`, `unicode-ident`, `unindent`, `indoc`, and more. Magnus pulls in `bindgen`, `clang-sys`, `rb-sys-build`, and more. Each framework is a build-time dependency tree of 20-40 crates. Our bridges have **zero Rust dependencies** — not even the raw `-sys` bindings.

4. **Build portability.** The `-sys` crates (pyo3-ffi, rb-sys, napi-sys) require language development headers and often `bindgen`/`clang` at build time. This fails on Windows where Ruby is built with MinGW but Rust targets MSVC. By declaring the C API functions ourselves with `extern "C"`, our bridges compile on **any platform with just a Rust toolchain** — no Python headers, no Ruby headers, no clang. The functions are resolved by the dynamic linker at load time against the running interpreter.

5. **ABI stability.** The C API functions we declare are part of each language's stable ABI:
   - **Python**: Limited API (PEP 384), stable since Python 3.2 (2011)
   - **Ruby**: `rb_define_method`, `rb_ary_push`, etc. — unchanged since Ruby 1.8 (2003)
   - **Node.js**: N-API, designed specifically for ABI stability across Node versions

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Native Extension Crate                       │
│               e.g. directed-graph-python-native                     │
│                                                                     │
│   Uses: python_bridge::{module, types, marshal, error}              │
│   Uses: directed_graph::{Graph, Node, Edge}                         │
│                                                                     │
│   Exports: #[no_mangle] pub extern "C" fn PyInit_directed_graph()   │
└──────────────┬──────────────────────────────────┬───────────────────┘
               │                                  │
               ▼                                  ▼
┌──────────────────────────┐       ┌──────────────────────────┐
│     python-bridge        │       │     directed-graph        │
│                          │       │     (pure Rust core)      │
│  Safe wrappers around    │       │                          │
│  raw extern "C" decls    │       │  Zero FFI knowledge.     │
│                          │       │  No Python types.        │
│  ~350 lines of Rust      │       │  No Ruby types.          │
└──────────┬───────────────┘       │  No Node types.          │
           │                       │                          │
           ▼                       │  Just data structures    │
┌──────────────────────────┐       │  and algorithms.         │
│       pyo3-ffi           │       └──────────────────────────┘
│  (raw C API bindings)    │
│  Generated from Python.h │
└──────────────────────────┘
```

The same pattern repeats for Ruby and Node.js:

```
native-extension ──► ruby-bridge ──► rb-sys       ──► ruby.h
native-extension ──► node-bridge ──► napi-sys     ──► node_api.h
native-extension ──► wasm-bridge ──► wasm-bindgen ──► (browser/WASI)
```

### Key Constraint: Core Crates Have Zero FFI Knowledge

The core Rust crate (`directed-graph`, `logic-gates`, `arithmetic`, etc.) must never depend on any bridge crate. It knows nothing about Python objects, Ruby VALUEs, or N-API environments. This is what makes the core crate testable, portable, and reusable. The native extension crate is the only place where bridge and core meet.

## Design Principles

### 1. Zero Third-Party Dependencies Beyond Raw C API Bindings

Each bridge crate has exactly one dependency:

| Bridge          | Sole dependency | What it provides                            |
|-----------------|-----------------|---------------------------------------------|
| `python-bridge` | `pyo3-ffi`      | `Py_*`, `PyObject`, `PyModule_*` functions  |
| `ruby-bridge`   | `rb-sys`        | `rb_*`, `VALUE`, `ID` functions             |
| `node-bridge`   | `napi-sys`      | `napi_*`, `napi_env`, `napi_value` functions|

No `syn`. No `quote`. No `proc-macro2`. No `serde`. No `thiserror`. Nothing.

### 2. No Macros — Explicit, Greppable Code

Macros hide control flow. When you write `#[pyfunction]` in PyO3, a procedural macro generates an entire wrapper function you cannot see, cannot step through in a debugger, and cannot `grep` for. Our bridges use plain functions:

```rust
// PyO3 approach — macro hides the actual FFI call:
#[pyfunction]
fn my_function(py: Python, name: &str) -> PyResult<String> {
    Ok(format!("hello {}", name))
}

// Our approach — every C API call is visible:
#[no_mangle]
pub extern "C" fn my_function(
    _self: *mut ffi::PyObject,
    args: *mut ffi::PyObject,
) -> *mut ffi::PyObject {
    let name = python_bridge::str_from_py(args);
    let result = format!("hello {}", name);
    python_bridge::str_to_py(&result)
}
```

The second version is longer. It is also completely transparent. Every line does exactly what it says, and you can set a breakpoint on any of them.

### 3. Safe Rust Wrappers Around Unsafe C Calls

Every raw C API call is `unsafe` because it dereferences raw pointers, relies on the GIL being held, or has preconditions the compiler cannot verify. The bridge crates encapsulate this unsafety in safe wrapper functions:

```rust
// In python_bridge/src/marshal.rs

/// Convert a Rust &str into a Python str object.
/// Returns a new reference (caller owns the refcount).
pub fn str_to_py(s: &str) -> *mut ffi::PyObject {
    unsafe {
        ffi::PyUnicode_FromStringAndSize(
            s.as_ptr() as *const c_char,
            s.len() as ffi::Py_ssize_t,
        )
    }
}
```

The caller never writes `unsafe`. The bridge function's name (`str_to_py`) is self-documenting. The unsafe block is small, auditable, and annotated.

### 4. Each Bridge is ~300-400 Lines, Not 15,000-50,000

A bridge crate has four modules:

| Module    | Purpose                                  | ~Lines |
|-----------|------------------------------------------|--------|
| `module`  | Module/class init, method registration   | 80     |
| `marshal` | Type conversion: Rust ↔ language types   | 120    |
| `data`    | Wrap/unwrap Rust structs in GC objects   | 60     |
| `error`   | Exception/error propagation              | 40     |
| **Total** |                                          | **~300** |

Compare with PyO3 (over 40,000 lines), Magnus (~15,000 lines), or napi-rs (~25,000 lines).

### 5. Type Marshaling Is Explicit

No `FromPyObject` trait. No `IntoRuby` trait. No automatic conversion. Every type conversion is a function call with a clear name:

```rust
let name: &str = str_from_py(py_arg);       // Python str → Rust &str
let count: i64 = int_from_py(py_arg);        // Python int → Rust i64
let items: Vec<String> = list_from_py(py_arg, str_from_py);  // Python list → Vec
```

This is more verbose than `extract::<String>()`. It is also impossible to misunderstand.

### 6. Entry Points Are Plain `#[no_mangle] pub extern "C" fn`

Every native extension's entry point is a standard C-callable function. No macro wraps it. No code generation produces it. The developer writes it by hand, and it looks like what the language runtime actually calls:

```rust
#[no_mangle]
pub extern "C" fn PyInit_my_module() -> *mut ffi::PyObject { ... }

#[no_mangle]
pub extern "C" fn Init_my_module() { ... }

#[no_mangle]
pub unsafe extern "C" fn napi_register_module_v1(
    env: napi_sys::napi_env,
    exports: napi_sys::napi_value,
) -> napi_sys::napi_value { ... }
```

---

## Python Bridge (`python-bridge` crate)

### Background: How CPython Extensions Work

CPython extensions are shared libraries (`.so` on Linux, `.pyd` on Windows, `.dylib` on macOS) that export a `PyInit_<name>` function. When Python executes `import my_module`, the interpreter:

1. Searches `sys.path` for `my_module.so` (or `.pyd`)
2. Calls `dlopen()` to load the shared library
3. Calls `dlsym("PyInit_my_module")` to find the init function
4. Calls `PyInit_my_module()`, which must return a `*mut PyObject` pointing to the new module

Everything after step 4 is your code, calling CPython's C API.

### Dependency

```toml
[dependencies]
pyo3-ffi = { version = "0.23", features = ["extension-module"] }
```

`pyo3-ffi` is the raw binding layer of PyO3, generated from Python's `Include/Python.h`. It provides `extern "C"` declarations for every CPython API function. It does not include any of PyO3's macro layer, trait system, or smart pointers.

### Module Initialization

The init function creates a module using CPython's multi-phase initialization protocol:

```rust
use pyo3_ffi as ffi;
use std::ffi::CString;

static mut MODULE_DEF: ffi::PyModuleDef = ffi::PyModuleDef {
    m_base: ffi::PyModuleDef_HEAD_INIT,
    m_name: c"directed_graph".as_ptr(),
    m_doc: c"Directed graph library".as_ptr(),
    m_size: -1,  // module has global state
    m_methods: std::ptr::null_mut(),  // set during init
    m_slots: std::ptr::null_mut(),
    m_traverse: None,
    m_clear: None,
    m_free: None,
};

#[no_mangle]
pub extern "C" fn PyInit_directed_graph() -> *mut ffi::PyObject {
    unsafe { ffi::PyModule_Create(&mut MODULE_DEF) }
}
```

The bridge wraps this in a helper:

```rust
// python_bridge::module

pub fn module_create(
    name: &'static str,
    doc: &'static str,
    methods: &[MethodDef],
) -> *mut ffi::PyObject {
    // Build PyMethodDef array from our MethodDef descriptors
    // Create PyModuleDef
    // Call PyModule_Create
    // Return the module pointer
}
```

### Class Definition via PyType_Spec

To expose a Rust struct as a Python class, CPython uses `PyType_Spec` — a descriptor that defines the class name, size, flags, and slot functions (init, dealloc, repr, etc.):

```rust
// The bridge provides a helper to build a type spec:

pub fn class_register(
    module: *mut ffi::PyObject,
    name: &'static str,
    new_fn: ffi::newfunc,       // __new__
    dealloc_fn: ffi::destructor, // __del__
    methods: &[MethodDef],
) -> *mut ffi::PyObject {
    // Build PyType_Spec with PyType_Slot entries
    // Call PyType_FromSpec
    // Add the type to the module via PyModule_AddObject
    // Return the type object
}
```

### Type Marshaling

Each marshaling function handles exactly one type conversion. There are no generic traits, no blanket implementations, no specialization tricks.

```rust
// python_bridge::marshal

/// Python str → Rust &str (borrows from the Python object's internal buffer).
/// The returned &str is valid as long as the PyObject is alive.
pub fn str_from_py(obj: *mut ffi::PyObject) -> &str { ... }

/// Rust &str → Python str (new reference, caller owns refcount).
pub fn str_to_py(s: &str) -> *mut ffi::PyObject { ... }

/// Python int → Rust i64.
pub fn int_from_py(obj: *mut ffi::PyObject) -> i64 { ... }

/// Rust i64 → Python int (new reference).
pub fn int_to_py(n: i64) -> *mut ffi::PyObject { ... }

/// Python bool → Rust bool.
pub fn bool_from_py(obj: *mut ffi::PyObject) -> bool { ... }

/// Rust bool → Python bool (returns Py_True or Py_False, new reference).
pub fn bool_to_py(b: bool) -> *mut ffi::PyObject { ... }

/// Python list → Rust Vec<T>, using a per-element conversion function.
pub fn list_from_py<T>(
    obj: *mut ffi::PyObject,
    convert: fn(*mut ffi::PyObject) -> T,
) -> Vec<T> { ... }

/// Rust slice → Python list (new reference).
pub fn list_to_py<T>(
    items: &[T],
    convert: fn(&T) -> *mut ffi::PyObject,
) -> *mut ffi::PyObject { ... }

/// Python tuple → Rust Vec<T>.
pub fn tuple_from_py<T>(
    obj: *mut ffi::PyObject,
    convert: fn(*mut ffi::PyObject) -> T,
) -> Vec<T> { ... }

/// Python set → Rust Vec<T> (order is not preserved).
pub fn set_from_py<T>(
    obj: *mut ffi::PyObject,
    convert: fn(*mut ffi::PyObject) -> T,
) -> Vec<T> { ... }
```

### Data Wrapping

To store a Rust struct inside a Python object, we allocate a Python object with enough space for a pointer to the Rust data, then store the pointer:

```rust
// python_bridge::data

/// Allocate a Python object and store a Rust value inside it.
/// The Rust value is boxed and leaked into a raw pointer, which is stored
/// in the PyObject's internal data area. The dealloc function must
/// reconstruct the Box and drop it.
pub fn wrap<T>(type_obj: *mut ffi::PyTypeObject, value: T) -> *mut ffi::PyObject { ... }

/// Retrieve a reference to the Rust value stored inside a Python object.
/// Returns None if the pointer is null.
pub fn unwrap<T>(obj: *mut ffi::PyObject) -> Option<&T> { ... }

/// Retrieve a mutable reference to the Rust value stored inside a Python object.
pub fn unwrap_mut<T>(obj: *mut ffi::PyObject) -> Option<&mut T> { ... }
```

### Reference Counting

CPython uses manual reference counting. Every `PyObject` has a reference count. When you create a new reference (return an object, store it in a list), you must increment. When you release a reference, you must decrement. When the count hits zero, the object is deallocated.

```rust
// python_bridge::refcount

/// Increment the reference count of a Python object.
/// Call this when you are creating a new reference to an existing object
/// (e.g., returning an object that someone else also holds a reference to).
pub fn incref(obj: *mut ffi::PyObject) {
    unsafe { ffi::Py_INCREF(obj) }
}

/// Decrement the reference count of a Python object.
/// If the count drops to zero, the object is immediately deallocated.
/// After calling this, the pointer is potentially dangling.
pub fn decref(obj: *mut ffi::PyObject) {
    unsafe { ffi::Py_DECREF(obj) }
}
```

The most common refcount bugs:
- **Leak**: forgetting to `decref` a returned object. The object lives forever.
- **Use-after-free**: calling `decref` too early, then using the dangling pointer.
- **Double-free**: calling `decref` twice on the same reference.

The bridge does not try to automate refcounting with smart pointers (that is what PyO3 does, and it adds 3,000 lines of code). Instead, each wrapper function documents whether it returns a "new reference" (caller owns) or a "borrowed reference" (caller must not decref).

### Exception Handling

When a C API call fails, CPython sets a thread-local error indicator. The extension must check for errors and either handle them or propagate them by returning `NULL`.

```rust
// python_bridge::error

/// Set a Python exception. After calling this, the extension function
/// must return NULL to propagate the exception to the Python interpreter.
pub fn raise(exc_type: *mut ffi::PyObject, message: &str) {
    let c_msg = CString::new(message).unwrap();
    unsafe { ffi::PyErr_SetString(exc_type, c_msg.as_ptr()) }
}

/// Set a TypeError exception.
pub fn raise_type_error(message: &str) {
    unsafe { raise(ffi::PyExc_TypeError, message) }
}

/// Set a ValueError exception.
pub fn raise_value_error(message: &str) {
    unsafe { raise(ffi::PyExc_ValueError, message) }
}

/// Set a RuntimeError exception.
pub fn raise_runtime_error(message: &str) {
    unsafe { raise(ffi::PyExc_RuntimeError, message) }
}

/// Check whether a Python exception is currently set.
pub fn occurred() -> bool {
    unsafe { !ffi::PyErr_Occurred().is_null() }
}
```

---

## Ruby Bridge (`ruby-bridge` crate)

### Background: How CRuby Extensions Work

CRuby extensions are shared libraries that export an `Init_<name>` function. When Ruby executes `require 'my_module'`, the interpreter:

1. Searches `$LOAD_PATH` for `my_module.so` (or `.bundle` on macOS)
2. Calls `dlopen()` to load the library
3. Calls `dlsym("Init_my_module")` to find the init function
4. Calls `Init_my_module()`, which registers classes and methods with the Ruby VM

### The VALUE Type

In CRuby, every Ruby object is represented as a `VALUE` — a pointer-sized integer. Small integers and special constants (`nil`, `true`, `false`) are encoded directly in the `VALUE` bits (called "immediate values" or "fixnums"). Everything else is a pointer to a heap-allocated `RObject` struct.

```
VALUE layout (64-bit):
  If least significant bit is 1 → Fixnum (integer, value = VALUE >> 1)
  If VALUE == 0x00               → false
  If VALUE == 0x02               → true
  If VALUE == 0x04               → nil
  If VALUE == 0x06               → undefined
  Otherwise                      → pointer to heap object
```

This means you never dereference a `VALUE` directly. You always call `rb_*` functions that know how to interpret the encoding.

### Dependency

```toml
[dependencies]
rb-sys = "0.9"
```

`rb-sys` provides raw bindings generated from Ruby's `ruby.h` and `intern.h`. It handles the complexity of finding the Ruby installation and linking against `libruby`.

### Module and Class Definition

```rust
// ruby_bridge::module

/// Create a Ruby module. Returns a VALUE representing the module.
pub fn define_module(name: &str) -> rb_sys::VALUE {
    let c_name = CString::new(name).unwrap();
    unsafe { rb_sys::rb_define_module(c_name.as_ptr()) }
}

/// Define a class under a module. Returns a VALUE representing the class.
pub fn define_class_under(
    parent: rb_sys::VALUE,
    name: &str,
    superclass: rb_sys::VALUE,
) -> rb_sys::VALUE {
    let c_name = CString::new(name).unwrap();
    unsafe { rb_sys::rb_define_class_under(parent, c_name.as_ptr(), superclass) }
}
```

### Method Binding

Ruby methods defined in C take a `VALUE self` (the receiver) plus arguments, and return a `VALUE`. The arity parameter tells Ruby how many arguments the method expects:

```rust
// ruby_bridge::module

/// Bind a Rust function as a Ruby method on a class/module.
///
/// arity:
///   >= 0 → fixed number of arguments
///   -1   → variable arguments (receives argc + *argv)
///   -2   → variable arguments (receives a Ruby array)
pub fn define_method(
    class: rb_sys::VALUE,
    name: &str,
    func: unsafe extern "C" fn(rb_sys::VALUE, ...) -> rb_sys::VALUE,
    arity: i32,
) {
    let c_name = CString::new(name).unwrap();
    unsafe {
        rb_sys::rb_define_method(
            class,
            c_name.as_ptr(),
            Some(std::mem::transmute(func)),
            arity as std::os::raw::c_int,
        )
    }
}
```

### Type Marshaling

```rust
// ruby_bridge::marshal

/// Ruby String → Rust &str.
/// The returned &str borrows from the Ruby string's internal buffer.
/// It is valid as long as the Ruby string is not modified or GC'd.
pub fn str_from_rb(val: rb_sys::VALUE) -> &str {
    unsafe {
        let ptr = rb_sys::rb_string_value_cstr(&val as *const _ as *mut _);
        let len = rb_sys::RSTRING_LEN(val) as usize;
        std::str::from_utf8_unchecked(std::slice::from_raw_parts(ptr as *const u8, len))
    }
}

/// Rust &str → Ruby String (new object, tracked by GC).
pub fn str_to_rb(s: &str) -> rb_sys::VALUE {
    unsafe {
        rb_sys::rb_utf8_str_new(s.as_ptr() as *const c_char, s.len() as c_long)
    }
}

/// Ruby integer → Rust i64.
pub fn int_from_rb(val: rb_sys::VALUE) -> i64 {
    unsafe { rb_sys::rb_num2long(val) as i64 }
}

/// Rust i64 → Ruby integer.
pub fn int_to_rb(n: i64) -> rb_sys::VALUE {
    unsafe { rb_sys::rb_ll2inum(n as c_longlong) }
}

/// Ruby true/false → Rust bool.
pub fn bool_from_rb(val: rb_sys::VALUE) -> bool {
    val != unsafe { rb_sys::Qfalse } && val != unsafe { rb_sys::Qnil }
}

/// Rust bool → Ruby true/false.
pub fn bool_to_rb(b: bool) -> rb_sys::VALUE {
    if b { unsafe { rb_sys::Qtrue } } else { unsafe { rb_sys::Qfalse } }
}

/// Ruby Array → Rust Vec<T>.
pub fn array_from_rb<T>(
    val: rb_sys::VALUE,
    convert: fn(rb_sys::VALUE) -> T,
) -> Vec<T> { ... }

/// Rust slice → Ruby Array.
pub fn array_to_rb<T>(
    items: &[T],
    convert: fn(&T) -> rb_sys::VALUE,
) -> rb_sys::VALUE { ... }
```

### Data Wrapping

CRuby provides `TypedData_Wrap_Struct` for storing arbitrary C/Rust data inside a Ruby object. The "typed data" mechanism tells the garbage collector the size of the data and provides callbacks for marking (if the data contains Ruby references) and freeing.

```rust
// ruby_bridge::data

/// Wrap a Rust value in a Ruby object. The Rust value is boxed and
/// owned by the Ruby GC — when the Ruby object is collected, the
/// free function drops the Box.
pub fn wrap<T>(class: rb_sys::VALUE, value: T) -> rb_sys::VALUE {
    let boxed = Box::into_raw(Box::new(value));
    unsafe {
        // Uses rb_data_typed_object_wrap with a static rb_data_type_t
        // that specifies the free function
        ...
    }
}

/// Retrieve a reference to the Rust value inside a Ruby typed-data object.
pub fn unwrap<T>(obj: rb_sys::VALUE) -> &T {
    unsafe {
        let ptr = rb_sys::rb_check_typeddata(obj, &DATA_TYPE as *const _);
        &*(ptr as *const T)
    }
}
```

### Exception Handling

```rust
// ruby_bridge::error

/// Raise a Ruby RuntimeError with the given message.
/// This function does not return — it performs a longjmp.
pub fn raise_runtime_error(message: &str) -> ! {
    let c_msg = CString::new(message).unwrap();
    unsafe {
        rb_sys::rb_raise(rb_sys::rb_eRuntimeError, c"%s\0".as_ptr(), c_msg.as_ptr());
    }
    unreachable!()
}

/// Raise a Ruby TypeError.
pub fn raise_type_error(message: &str) -> ! {
    let c_msg = CString::new(message).unwrap();
    unsafe {
        rb_sys::rb_raise(rb_sys::rb_eTypeError, c"%s\0".as_ptr(), c_msg.as_ptr());
    }
    unreachable!()
}

/// Raise a Ruby ArgumentError.
pub fn raise_argument_error(message: &str) -> ! {
    let c_msg = CString::new(message).unwrap();
    unsafe {
        rb_sys::rb_raise(rb_sys::rb_eArgError, c"%s\0".as_ptr(), c_msg.as_ptr());
    }
    unreachable!()
}
```

**Important:** Ruby's `rb_raise` performs a `longjmp`, which means it never returns. Rust destructors for values on the current stack frame will **not** run. This is a fundamental mismatch between Ruby's exception model and Rust's RAII model. The bridge must ensure no Rust resources need dropping when `rb_raise` is called.

### Entry Point

```rust
#[no_mangle]
pub extern "C" fn Init_directed_graph() {
    let module = ruby_bridge::define_module("DirectedGraph");
    let graph_class = ruby_bridge::define_class_under(
        module, "Graph", unsafe { rb_sys::rb_cObject }
    );
    ruby_bridge::define_method(graph_class, "initialize", graph_new, 0);
    ruby_bridge::define_method(graph_class, "add_node", graph_add_node, 1);
    ruby_bridge::define_method(graph_class, "add_edge", graph_add_edge, 2);
    ruby_bridge::define_method(graph_class, "topological_sort", graph_topo_sort, 0);
}
```

---

## Node.js Bridge (`node-bridge` crate)

### Background: How Node.js Native Addons Work

Node.js native addons use N-API (Node-API), a C API that is ABI-stable across Node.js versions. Unlike the older V8-based addon API, N-API functions never expose V8 internals. Every function takes an opaque `napi_env` handle, making the API stateless and forward-compatible.

When Node.js loads a native addon via `require('my_module')`, it:

1. Calls `dlopen()` on the `.node` file (a renamed `.so`/`.dylib`/`.dll`)
2. Calls `napi_register_module_v1(env, exports)`, which receives the module's exports object
3. The init function populates the exports object with functions, classes, and values

### The Stateless Design

N-API is different from CPython and CRuby APIs in one crucial way: **every function takes `napi_env` as its first argument**. There is no global state, no thread-local state, no GIL. The environment handle is the only way to interact with the JavaScript runtime.

```
CPython:  PyLong_FromLong(42)           // implicit global state, GIL must be held
CRuby:    rb_ll2inum(42)               // implicit global VM state
N-API:    napi_create_int64(env, 42, &result)  // explicit env, explicit output pointer
```

This makes N-API easier to reason about but more verbose. Every call requires passing `env`.

### Dependency

```toml
[dependencies]
napi-sys = "2"
```

`napi-sys` provides raw bindings generated from Node.js's `node_api.h`. It declares `napi_env`, `napi_value`, `napi_status`, and all the `napi_*` functions.

### Module Initialization

```rust
use napi_sys::*;

#[no_mangle]
pub unsafe extern "C" fn napi_register_module_v1(
    env: napi_env,
    exports: napi_value,
) -> napi_value {
    // Register functions on the exports object
    node_bridge::define_function(env, exports, "addNode", add_node_fn);
    node_bridge::define_function(env, exports, "addEdge", add_edge_fn);
    exports
}
```

The bridge helper:

```rust
// node_bridge::module

/// Register a function as a named property on the exports object.
pub fn define_function(
    env: napi_env,
    exports: napi_value,
    name: &str,
    func: napi_callback,
) {
    let c_name = CString::new(name).unwrap();
    let mut result: napi_value = std::ptr::null_mut();
    unsafe {
        napi_create_function(env, c_name.as_ptr(), name.len(), func, std::ptr::null_mut(), &mut result);
        napi_set_named_property(env, exports, c_name.as_ptr(), result);
    }
}
```

### Class Definition

```rust
// node_bridge::module

/// Define a JavaScript class with constructor and methods.
/// Returns a napi_value representing the class constructor.
pub fn define_class(
    env: napi_env,
    name: &str,
    constructor: napi_callback,
    properties: &[PropertyDescriptor],
) -> napi_value {
    let c_name = CString::new(name).unwrap();
    let mut result: napi_value = std::ptr::null_mut();
    // Build napi_property_descriptor array from PropertyDescriptor
    // Call napi_define_class
    unsafe {
        napi_define_class(
            env,
            c_name.as_ptr(),
            name.len(),
            constructor,
            std::ptr::null_mut(),
            properties.len(),
            props.as_ptr(),
            &mut result,
        );
    }
    result
}
```

### Type Marshaling

N-API uses an output-pointer pattern: you pass a `&mut napi_value` and the function fills it in. The bridge wraps this:

```rust
// node_bridge::marshal

/// JavaScript string → Rust String.
pub fn string_from_js(env: napi_env, val: napi_value) -> String {
    unsafe {
        let mut len: usize = 0;
        napi_get_value_string_utf8(env, val, std::ptr::null_mut(), 0, &mut len);
        let mut buf = vec![0u8; len + 1];
        napi_get_value_string_utf8(env, val, buf.as_mut_ptr() as *mut c_char, len + 1, &mut len);
        String::from_utf8_lossy(&buf[..len]).into_owned()
    }
}

/// Rust &str → JavaScript string.
pub fn string_to_js(env: napi_env, s: &str) -> napi_value {
    let mut result: napi_value = std::ptr::null_mut();
    unsafe {
        napi_create_string_utf8(env, s.as_ptr() as *const c_char, s.len(), &mut result);
    }
    result
}

/// JavaScript number → Rust i64.
pub fn int_from_js(env: napi_env, val: napi_value) -> i64 {
    let mut result: i64 = 0;
    unsafe { napi_get_value_int64(env, val, &mut result); }
    result
}

/// Rust i64 → JavaScript number.
pub fn int_to_js(env: napi_env, n: i64) -> napi_value {
    let mut result: napi_value = std::ptr::null_mut();
    unsafe { napi_create_int64(env, n, &mut result); }
    result
}

/// JavaScript boolean → Rust bool.
pub fn bool_from_js(env: napi_env, val: napi_value) -> bool {
    let mut result: bool = false;
    unsafe { napi_get_value_bool(env, val, &mut result); }
    result
}

/// Rust bool → JavaScript boolean.
pub fn bool_to_js(env: napi_env, b: bool) -> napi_value {
    let mut result: napi_value = std::ptr::null_mut();
    unsafe { napi_get_boolean(env, b, &mut result); }
    result
}

/// JavaScript Array → Rust Vec<T>.
pub fn array_from_js<T>(
    env: napi_env,
    val: napi_value,
    convert: fn(napi_env, napi_value) -> T,
) -> Vec<T> { ... }

/// Rust slice → JavaScript Array.
pub fn array_to_js<T>(
    env: napi_env,
    items: &[T],
    convert: fn(napi_env, &T) -> napi_value,
) -> napi_value { ... }
```

### Data Wrapping

N-API provides `napi_wrap` and `napi_unwrap` for associating a native pointer with a JavaScript object. A release callback is called when the JavaScript object is garbage collected.

```rust
// node_bridge::data

/// Associate a Rust value with a JavaScript object.
/// The Rust value is boxed and owned by the GC — when the JS object
/// is collected, the release callback drops the Box.
pub fn wrap<T>(env: napi_env, js_object: napi_value, value: T) {
    let boxed = Box::into_raw(Box::new(value));
    unsafe {
        napi_wrap(
            env,
            js_object,
            boxed as *mut c_void,
            Some(release_callback::<T>),  // drop the Box when GC'd
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
    }
}

/// Retrieve a reference to the Rust value associated with a JavaScript object.
pub fn unwrap<T>(env: napi_env, js_object: napi_value) -> &T {
    let mut raw: *mut c_void = std::ptr::null_mut();
    unsafe {
        napi_unwrap(env, js_object, &mut raw);
        &*(raw as *const T)
    }
}

/// GC release callback — reconstructs and drops the Box.
unsafe extern "C" fn release_callback<T>(
    _env: napi_env,
    data: *mut c_void,
    _hint: *mut c_void,
) {
    drop(Box::from_raw(data as *mut T));
}
```

### Error Handling

N-API error handling uses return codes (`napi_status`) rather than exceptions or longjmps. Every N-API function returns a status code, and errors can be thrown explicitly:

```rust
// node_bridge::error

/// Throw a JavaScript Error with the given message.
pub fn throw_error(env: napi_env, message: &str) {
    let c_msg = CString::new(message).unwrap();
    unsafe {
        napi_throw_error(env, std::ptr::null(), c_msg.as_ptr());
    }
}

/// Throw a TypeError.
pub fn throw_type_error(env: napi_env, message: &str) {
    let c_msg = CString::new(message).unwrap();
    unsafe {
        napi_throw_type_error(env, std::ptr::null(), c_msg.as_ptr());
    }
}

/// Throw a RangeError.
pub fn throw_range_error(env: napi_env, message: &str) {
    let c_msg = CString::new(message).unwrap();
    unsafe {
        napi_throw_range_error(env, std::ptr::null(), c_msg.as_ptr());
    }
}

/// Check the status of a N-API call and throw if it failed.
pub fn check_status(env: napi_env, status: napi_status) -> bool {
    if status != napi_status::napi_ok {
        throw_error(env, &format!("N-API call failed with status {:?}", status));
        return false;
    }
    true
}
```

---

## WASM Bridge

The WASM bridge is an exception: `wasm-bindgen` stays as-is. Unlike PyO3/Magnus/napi-rs, `wasm-bindgen` is already thin — it generates the minimal glue needed to cross the JavaScript/WASM boundary. There is no alternative raw binding to use because WASM's FFI layer is defined by the WASM specification itself, not by a C header file.

```toml
[dependencies]
wasm-bindgen = "0.2"
```

The WASM bridge follows the same pattern of explicit type marshaling and `#[wasm_bindgen]` annotations, which generate only the serialization stubs required by the WASM calling convention.

---

## How Native Extensions Use the Bridges

A native extension crate is the meeting point of a bridge crate and a core crate. It has exactly two responsibilities:

1. Convert language-specific types to Rust types (using the bridge)
2. Call the core Rust library with those Rust types

Here is the complete pattern for a Python native extension:

```rust
// crate: directed-graph-python-native
// Cargo.toml dependencies: python-bridge, directed-graph

use python_bridge::{module_create, class_register, MethodDef};
use python_bridge::marshal::{str_from_py, str_to_py, list_to_py, int_to_py};
use python_bridge::data::{wrap, unwrap};
use python_bridge::error::raise_value_error;
use directed_graph::Graph;
use pyo3_ffi as ffi;

// ── Entry point ────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn PyInit_directed_graph_native() -> *mut ffi::PyObject {
    module_create("directed_graph_native", "Directed graph library", &[
        MethodDef::new("create_graph", create_graph, 0),
    ])
}

// ── Functions ──────────────────────────────────────────────────────

extern "C" fn create_graph(
    _self: *mut ffi::PyObject,
    _args: *mut ffi::PyObject,
) -> *mut ffi::PyObject {
    let graph = Graph::new();
    wrap(GRAPH_TYPE, graph)
}

extern "C" fn add_node(
    self_obj: *mut ffi::PyObject,
    args: *mut ffi::PyObject,
) -> *mut ffi::PyObject {
    let graph: &mut Graph = unwrap_mut(self_obj);
    let name = str_from_py(args);   // explicit conversion
    graph.add_node(name);           // pure Rust call
    python_bridge::none()           // return None
}

extern "C" fn topological_sort(
    self_obj: *mut ffi::PyObject,
    _args: *mut ffi::PyObject,
) -> *mut ffi::PyObject {
    let graph: &Graph = unwrap(self_obj);
    match graph.topological_sort() {
        Ok(order) => list_to_py(&order, str_to_py),  // explicit conversion
        Err(e) => {
            raise_value_error(&format!("cycle detected: {}", e));
            std::ptr::null_mut()  // return NULL to propagate exception
        }
    }
}
```

The equivalent Ruby extension:

```rust
// crate: directed-graph-ruby-native

use ruby_bridge::{define_module, define_class_under, define_method};
use ruby_bridge::marshal::{str_from_rb, str_to_rb, array_to_rb};
use ruby_bridge::data::{wrap, unwrap};
use directed_graph::Graph;

#[no_mangle]
pub extern "C" fn Init_directed_graph() {
    let module = define_module("DirectedGraph");
    let klass = define_class_under(module, "Graph", rb_sys::rb_cObject);
    define_method(klass, "initialize", graph_new, 0);
    define_method(klass, "add_node", graph_add_node, 1);
    define_method(klass, "topological_sort", graph_topo_sort, 0);
}

extern "C" fn graph_new(self_val: rb_sys::VALUE) -> rb_sys::VALUE {
    let graph = Graph::new();
    wrap(self_val, graph);
    self_val
}

extern "C" fn graph_add_node(self_val: rb_sys::VALUE, name: rb_sys::VALUE) -> rb_sys::VALUE {
    let graph: &mut Graph = unwrap(self_val);
    let name = str_from_rb(name);
    graph.add_node(name);
    ruby_bridge::nil()
}
```

The equivalent Node.js extension:

```rust
// crate: directed-graph-node-native

use node_bridge::module::define_function;
use node_bridge::marshal::{string_from_js, string_to_js, array_to_js};
use node_bridge::data::{wrap, unwrap};
use directed_graph::Graph;
use napi_sys::*;

#[no_mangle]
pub unsafe extern "C" fn napi_register_module_v1(
    env: napi_env,
    exports: napi_value,
) -> napi_value {
    define_function(env, exports, "createGraph", create_graph);
    define_function(env, exports, "addNode", add_node);
    define_function(env, exports, "topologicalSort", topo_sort);
    exports
}

unsafe extern "C" fn create_graph(
    env: napi_env,
    info: napi_callback_info,
) -> napi_value {
    let graph = Graph::new();
    let js_obj = node_bridge::create_object(env);
    wrap(env, js_obj, graph);
    js_obj
}
```

Notice the pattern across all three: the native extension is pure glue. It calls bridge functions for type conversion and core library functions for logic. No framework magic. No generated code. Every function call is visible in the source.

---

## Comparison with Third-Party Frameworks

| Metric                        | PyO3         | python-bridge | Magnus       | ruby-bridge | napi-rs      | node-bridge |
|-------------------------------|--------------|---------------|--------------|-------------|--------------|-------------|
| **Lines of code**             | ~45,000      | ~350          | ~15,000      | ~300        | ~25,000      | ~300        |
| **Direct dependencies**       | 8+           | 1 (pyo3-ffi)  | 5+           | 1 (rb-sys)  | 6+           | 1 (napi-sys)|
| **Transitive dependencies**   | 20-40        | 1             | 15-25        | 1           | 15-30        | 1           |
| **Uses proc macros**          | Yes          | No            | Yes          | No          | Yes          | No          |
| **Requires syn/quote**        | Yes          | No            | Yes          | No          | Yes          | No          |
| **Debuggable stack traces**   | Deep         | Shallow       | Deep         | Shallow     | Deep         | Shallow     |
| **Time to understand fully**  | Days-weeks   | Hours         | Days-weeks   | Hours       | Days-weeks   | Hours       |
| **Automatic type conversion** | Yes (traits) | No (explicit) | Yes (traits) | No (explicit)| Yes (traits)| No (explicit)|
| **Smart pointer refcounting** | Yes (Py<T>)  | No (manual)   | N/A (GC)     | N/A (GC)    | N/A (GC)     | N/A (GC)    |
| **Error handling**            | PyResult<T>  | NULL + set err| Magnus::Error| rb_raise    | napi::Error  | throw_error |
| **Compile time impact**       | High (macros)| Minimal       | High (macros)| Minimal     | High (macros)| Minimal     |

### When to Use What

**Use PyO3/Magnus/napi-rs when:**
- You are building a production library and want maximum ergonomics
- You don't need to understand the underlying FFI mechanism
- Compile times are not a concern
- The framework's abstraction matches your mental model

**Use the raw bridges when:**
- You are learning how language FFI works
- You need to debug native extensions at the C API level
- You want minimal compile-time dependencies
- You prefer explicit code over implicit trait dispatch
- Your extension is simple enough that framework overhead is not justified

## Test Strategy

Each bridge crate is tested by building a minimal native extension and loading it from the target language:

1. **Python**: build `.so`, `import` it from Python, call functions, check results
2. **Ruby**: build `.so`, `require` it from Ruby, call methods, check results
3. **Node.js**: build `.node`, `require()` it from JavaScript, call functions, check results

Integration tests cover:
- Module initialization succeeds
- Function calls with each supported type
- Round-trip marshaling (Rust → language → Rust produces the same value)
- Error/exception propagation
- GC interaction (objects are not prematurely freed)
- Multi-threaded access (where applicable)

Unit tests within each bridge test individual marshaling functions using mock `PyObject`/`VALUE`/`napi_value` values where possible.

## Future Extensions

- **Float marshaling**: `f64` support for all three bridges
- **Dict/Hash/Object marshaling**: key-value containers
- **Callback support**: passing language-side closures into Rust
- **Async bridge**: Python `asyncio` / Node.js `Promise` integration
- **GIL release**: allowing long-running Rust code to release the Python GIL
- **Memory-mapped buffers**: zero-copy data sharing via `PyBuffer` / `TypedArray`
