// lib.rs -- ImmutableList Python extension using python-bridge
// =============================================================
//
// Native extension wrapping the Rust `immutable-list` crate for Python via
// our zero-dependency python-bridge. This follows the same architecture as
// the bitset-native extension:
//
// 1. PyInit_immutable_list_native() creates the module
// 2. A PyTypeObject "ImmutableList" is created via PyType_FromSpec
// 3. Each instance holds a pointer to a Rust ImmutableList in its object body
// 4. Method calls extract the ImmutableList pointer, call Rust, marshal result
// 5. A custom ImmutableListError exception for error conditions
//
// # API Surface
//
// The ImmutableList is a persistent (immutable) data structure. Every
// "mutation" returns a NEW list, leaving the original unchanged. Structural
// sharing happens inside Rust via Arc, so creating new versions is cheap.
//
//   - Constructor: ImmutableList()  -- creates an empty list
//   - Class method: from_list(items) -- builds from a Python list of strings
//   - Mutations (return new ImmutableList):
//       push(value) -> new list with value appended
//       set(index, value) -> new list with element at index replaced
//       pop() -> (new list, removed value) tuple
//   - Queries:
//       get(index) -> str or None
//       len() -> int  (also via __len__)
//       is_empty() -> bool
//       to_list() -> Python list of str
//   - Protocols:
//       __getitem__ (supports negative indices)
//       __len__
//       __iter__
//       __eq__
//       __repr__

use std::ffi::{c_char, c_int, c_long, c_void, CString};
use std::ptr;

use immutable_list::ImmutableList;
use python_bridge::*;

// ---------------------------------------------------------------------------
// Slot numbers from CPython's typeslots.h (stable ABI)
// ---------------------------------------------------------------------------
//
// These numeric constants identify which "slot" a function pointer fills
// in the type object. They come from CPython's Include/typeslots.h.
// CRITICAL: these must match exactly -- wrong numbers cause silent crashes.

const PY_MP_SUBSCRIPT: c_int = 5;     // Py_mp_subscript (__getitem__)
const PY_SQ_LENGTH: c_int = 45;       // Py_sq_length (__len__)
const PY_TP_DEALLOC: c_int = 52;      // Py_tp_dealloc
const PY_TP_ITER: c_int = 62;         // Py_tp_iter (__iter__)
const PY_TP_METHODS: c_int = 64;      // Py_tp_methods
const PY_TP_NEW: c_int = 65;          // Py_tp_new
const PY_TP_REPR: c_int = 66;         // Py_tp_repr
const PY_TP_RICHCOMPARE: c_int = 67;  // Py_tp_richcompare (__eq__)

// Rich comparison opcodes (from CPython object.h)
const PY_EQ: c_int = 2;
const PY_NE: c_int = 3;

// METH_CLASS flag -- CPython uses 0x0010 for class methods
const METH_CLASS: c_int = 0x0010;

// ---------------------------------------------------------------------------
// Additional CPython extern declarations not in python-bridge
// ---------------------------------------------------------------------------

#[allow(non_snake_case)]
extern "C" {
    fn PyType_GenericAlloc(type_obj: PyObjectPtr, nitems: isize) -> PyObjectPtr;
    fn PyObject_Free(ptr: *mut c_void);
    fn PyLong_AsLong(obj: PyObjectPtr) -> c_long;
    fn PyObject_IsInstance(obj: PyObjectPtr, cls: PyObjectPtr) -> c_int;
    fn PyErr_Occurred() -> PyObjectPtr;
}

// ---------------------------------------------------------------------------
// Py_NotImplemented helper
// ---------------------------------------------------------------------------
//
// On Windows, accessing _Py_NotImplementedStruct as an extern static has
// dllimport issues. We fetch it via builtins.NotImplemented instead.

unsafe fn py_not_implemented() -> PyObjectPtr {
    let builtins_name = CString::new("builtins").unwrap();
    let builtins = PyImport_ImportModule(builtins_name.as_ptr());
    let attr_name = CString::new("NotImplemented").unwrap();
    let not_impl = PyObject_GetAttrString(builtins, attr_name.as_ptr());
    Py_DecRef(builtins);
    not_impl // already a new reference from GetAttrString
}

// ---------------------------------------------------------------------------
// Instance layout: ListObject = PyObject_HEAD + immutable list pointer
// ---------------------------------------------------------------------------
//
// Every Python object starts with ob_refcnt and ob_type (the "PyObject head").
// After that comes our custom field: a pointer to a heap-allocated Rust
// ImmutableList. Because ImmutableList uses Arc internally for structural
// sharing, cloning is cheap -- but each Python wrapper owns its own
// Box<ImmutableList> on the heap.
//
//     Memory layout:
//
//     ┌─────────────┐
//     │ ob_refcnt    │  Python reference count
//     │ ob_type      │  Pointer to our type object
//     │ inner ───────┼──► Box<ImmutableList>
//     └─────────────┘        │
//                            ▼
//                    ┌───────────────┐
//                    │ root: Arc     │  Shared trie structure
//                    │ tail: Vec     │
//                    │ len, shift    │
//                    └───────────────┘

#[repr(C)]
struct ListObject {
    ob_refcnt: isize,
    ob_type: PyObjectPtr,
    inner: *mut ImmutableList,
}

// ---------------------------------------------------------------------------
// Exception class global
// ---------------------------------------------------------------------------

static mut LIST_ERROR: PyObjectPtr = ptr::null_mut();

// ---------------------------------------------------------------------------
// Type object global (needed for creating new instances from Rust methods)
// ---------------------------------------------------------------------------

static mut LIST_TYPE: PyObjectPtr = ptr::null_mut();

// ---------------------------------------------------------------------------
// ImmutableList access helpers
// ---------------------------------------------------------------------------
//
// These extract the inner Rust ImmutableList from a Python object pointer.
// The caller must ensure `slf` is a valid ListObject pointer.

unsafe fn get_list(slf: PyObjectPtr) -> &'static ImmutableList {
    &*((slf as *mut ListObject).read().inner)
}

// ---------------------------------------------------------------------------
// Helper: create a new Python ImmutableList object wrapping a Rust one
// ---------------------------------------------------------------------------
//
// This is the CRITICAL function for immutability semantics. Every mutation
// method (push, set, pop) calls the Rust ImmutableList method which returns
// a NEW Rust ImmutableList (sharing structure with the old one via Arc).
// We then wrap that new Rust list in a new Python object.
//
//     Python: new_list = old_list.push("hello")
//
//     Under the hood:
//     1. Extract old_list's inner ImmutableList
//     2. Call rust_list.push("hello") -> new_rust_list (Arc-shared)
//     3. wrap_list(new_rust_list) -> new Python ListObject
//     4. old_list is unchanged (still points to old Rust list)

unsafe fn wrap_list(list: ImmutableList) -> PyObjectPtr {
    let obj = PyType_GenericAlloc(LIST_TYPE, 0);
    if obj.is_null() {
        return ptr::null_mut();
    }
    (*(obj as *mut ListObject)).inner = Box::into_raw(Box::new(list));
    obj
}

// ---------------------------------------------------------------------------
// Helper: check if a PyObject is our ImmutableList type
// ---------------------------------------------------------------------------

unsafe fn is_list(obj: PyObjectPtr) -> bool {
    if obj.is_null() || LIST_TYPE.is_null() {
        return false;
    }
    PyObject_IsInstance(obj, LIST_TYPE) == 1
}

// ---------------------------------------------------------------------------
// Helper: parse a single integer argument from Python args tuple
// ---------------------------------------------------------------------------

unsafe fn parse_arg_long(args: PyObjectPtr, index: isize) -> Option<c_long> {
    let arg = PyTuple_GetItem(args, index);
    if arg.is_null() {
        return None;
    }
    let val = PyLong_AsLong(arg);
    if val == -1 && !PyErr_Occurred().is_null() {
        PyErr_Clear();
        return None;
    }
    Some(val)
}

// ---------------------------------------------------------------------------
// Helper: resolve a possibly-negative index to a positive one
// ---------------------------------------------------------------------------
//
// Python convention: negative indices count from the end.
//   list[-1] is the last element
//   list[-len] is the first element
//   list[-len-1] is out of bounds
//
// Returns None if the resolved index is out of bounds.

fn resolve_index(index: c_long, length: usize) -> Option<usize> {
    let len = length as c_long;
    if index >= 0 {
        let i = index as usize;
        if i < length { Some(i) } else { None }
    } else {
        let resolved = len + index;
        if resolved >= 0 { Some(resolved as usize) } else { None }
    }
}

// ---------------------------------------------------------------------------
// tp_new and tp_dealloc
// ---------------------------------------------------------------------------
//
// tp_new: ImmutableList() creates a new empty list.
// tp_dealloc: drops the Box<ImmutableList>, freeing the Rust memory.

unsafe extern "C" fn list_new(
    type_obj: PyObjectPtr,
    _args: PyObjectPtr,
    _kwargs: PyObjectPtr,
) -> PyObjectPtr {
    // ImmutableList() -- no arguments, always creates an empty list.
    // Use from_list() class method to build from existing data.
    let obj = PyType_GenericAlloc(type_obj, 0);
    if obj.is_null() {
        return ptr::null_mut();
    }
    (*(obj as *mut ListObject)).inner = Box::into_raw(Box::new(ImmutableList::new()));
    obj
}

unsafe extern "C" fn list_dealloc(obj: PyObjectPtr) {
    let list_obj = obj as *mut ListObject;
    if !(*list_obj).inner.is_null() {
        // Drop the Box, which decrements Arc refcounts inside the trie.
        // If this was the last reference to certain trie nodes, they get freed.
        let _ = Box::from_raw((*list_obj).inner);
        (*list_obj).inner = ptr::null_mut();
    }
    PyObject_Free(obj as *mut c_void);
}

// ---------------------------------------------------------------------------
// sq_length (__len__)
// ---------------------------------------------------------------------------
//
// Returns the number of elements. O(1) -- the length is stored as a field.

unsafe extern "C" fn list_sq_length(slf: PyObjectPtr) -> isize {
    get_list(slf).len() as isize
}

// ---------------------------------------------------------------------------
// tp_repr (__repr__)
// ---------------------------------------------------------------------------
//
// Produces a string like: ImmutableList(['a', 'b', 'c'])
// For long lists (>10 elements), we truncate with "..." to avoid
// enormous repr strings.

unsafe extern "C" fn list_repr(slf: PyObjectPtr) -> PyObjectPtr {
    let list = get_list(slf);
    let len = list.len();
    let mut parts = Vec::new();

    // Show at most 10 elements in the repr
    let show_count = if len > 10 { 10 } else { len };
    for i in 0..show_count {
        if let Some(s) = list.get(i) {
            parts.push(format!("'{}'", s));
        }
    }

    let contents = if len > 10 {
        format!("{}, ...", parts.join(", "))
    } else {
        parts.join(", ")
    };

    str_to_py(&format!("ImmutableList([{}])", contents))
}

// ---------------------------------------------------------------------------
// tp_richcompare (__eq__, __ne__)
// ---------------------------------------------------------------------------
//
// Two ImmutableLists are equal if they contain the same elements in the
// same order. We delegate to the Rust PartialEq implementation which
// first checks lengths, then does element-by-element comparison.

unsafe extern "C" fn list_richcompare(
    slf: PyObjectPtr,
    other: PyObjectPtr,
    op: c_int,
) -> PyObjectPtr {
    // Only support == and !=. For everything else, return NotImplemented.
    if op != PY_EQ && op != PY_NE {
        return py_not_implemented();
    }

    // If other is not an ImmutableList, return NotImplemented.
    if !is_list(other) {
        return py_not_implemented();
    }

    let a = get_list(slf);
    let b = get_list(other);
    let equal = a == b;

    match op {
        PY_EQ => bool_to_py(equal),
        PY_NE => bool_to_py(!equal),
        _ => py_not_implemented(),
    }
}

// ---------------------------------------------------------------------------
// tp_iter (__iter__)
// ---------------------------------------------------------------------------
//
// Returns a Python iterator over the list's string elements.
// Strategy: build a Python list of str, then return its iterator.
// This is simple and correct. For very large lists, a lazy iterator
// would be more memory-efficient, but this matches the bitset-native pattern.

unsafe extern "C" fn list_iter(slf: PyObjectPtr) -> PyObjectPtr {
    let list = get_list(slf);
    let py_list = PyList_New(list.len() as isize);
    for i in 0..list.len() {
        if let Some(s) = list.get(i) {
            PyList_SetItem(py_list, i as isize, str_to_py(s));
        }
    }
    let iter = PyObject_GetIter(py_list);
    Py_DecRef(py_list);
    iter
}

// ---------------------------------------------------------------------------
// mp_subscript (__getitem__) -- supports negative indices
// ---------------------------------------------------------------------------
//
// Python's __getitem__ protocol goes through the mapping protocol when
// using PyType_FromSpec (slot Py_mp_subscript = 5). This allows both
// positive and negative integer indexing:
//
//   list[0]   -> first element
//   list[-1]  -> last element
//   list[99]  -> IndexError if out of bounds

unsafe extern "C" fn list_mp_subscript(
    slf: PyObjectPtr,
    key: PyObjectPtr,
) -> PyObjectPtr {
    let index = PyLong_AsLong(key);
    if index == -1 && !PyErr_Occurred().is_null() {
        // key was not an integer
        PyErr_Clear();
        set_error(value_error_class(), "list indices must be integers");
        return ptr::null_mut();
    }

    let list = get_list(slf);
    match resolve_index(index, list.len()) {
        Some(i) => {
            match list.get(i) {
                Some(s) => str_to_py(s),
                None => py_none(),
            }
        }
        None => {
            // Get IndexError from builtins
            let builtins_name = CString::new("builtins").unwrap();
            let builtins = PyImport_ImportModule(builtins_name.as_ptr());
            let exc_name = CString::new("IndexError").unwrap();
            let index_error = PyObject_GetAttrString(builtins, exc_name.as_ptr());
            Py_DecRef(builtins);
            set_error(index_error, &format!("index out of range: {}", index));
            Py_DecRef(index_error);
            ptr::null_mut()
        }
    }
}

// ---------------------------------------------------------------------------
// Method implementations
// ---------------------------------------------------------------------------

// -- push(value) -> new ImmutableList --
//
// Appends value to the end and returns a NEW list. The original is unchanged.
// This is the most common operation -- ~97% of calls just append to the
// internal tail buffer (O(1)), only every 32nd push promotes into the trie.

unsafe extern "C" fn list_push(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let value = match parse_arg_str(args, 0) {
        Some(s) => s,
        None => {
            set_error(value_error_class(), "push requires one string argument");
            return ptr::null_mut();
        }
    };
    let old = get_list(slf);
    let new_list = old.push(value);
    wrap_list(new_list)
}

// -- get(index) -> str or None --
//
// Returns the element at the given index, or None if out of bounds.
// Supports negative indices (Python convention).

unsafe extern "C" fn list_get(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let index = match parse_arg_long(args, 0) {
        Some(v) => v,
        None => {
            set_error(value_error_class(), "get requires one integer argument");
            return ptr::null_mut();
        }
    };

    let list = get_list(slf);
    match resolve_index(index, list.len()) {
        Some(i) => {
            match list.get(i) {
                Some(s) => str_to_py(s),
                None => py_none(),
            }
        }
        None => py_none(),
    }
}

// -- set(index, value) -> new ImmutableList --
//
// Replaces the element at index with value and returns a NEW list.
// The original is unchanged. Uses path-copying in the trie for O(log32 n).

unsafe extern "C" fn list_set(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let index = match parse_arg_long(args, 0) {
        Some(v) => v,
        None => {
            set_error(value_error_class(), "set requires (index, value) arguments");
            return ptr::null_mut();
        }
    };
    let value = match parse_arg_str(args, 1) {
        Some(s) => s,
        None => {
            set_error(value_error_class(), "set requires (index, value) arguments");
            return ptr::null_mut();
        }
    };

    let list = get_list(slf);
    match resolve_index(index, list.len()) {
        Some(i) => {
            let new_list = list.set(i, value);
            wrap_list(new_list)
        }
        None => {
            set_error(
                unsafe { LIST_ERROR },
                &format!("index out of range: {}", index),
            );
            ptr::null_mut()
        }
    }
}

// -- pop() -> (new ImmutableList, str) --
//
// Removes the last element and returns a tuple of (new_list, removed_value).
// Raises ImmutableListError if the list is empty.

unsafe extern "C" fn list_pop(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    let list = get_list(slf);
    if list.is_empty() {
        set_error(LIST_ERROR, "cannot pop from an empty list");
        return ptr::null_mut();
    }
    let (new_list, val) = list.pop();
    let py_new = wrap_list(new_list);
    let py_val = str_to_py(&val);

    // Return a tuple (new_list, value)
    let tuple = PyTuple_New(2);
    PyTuple_SetItem(tuple, 0, py_new);
    PyTuple_SetItem(tuple, 1, py_val);
    tuple
}

// -- len() -> int --
//
// Returns the number of elements. Also accessible via __len__ / len().

unsafe extern "C" fn list_len_method(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    usize_to_py(get_list(slf).len())
}

// -- is_empty() -> bool --

unsafe extern "C" fn list_is_empty(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    bool_to_py(get_list(slf).is_empty())
}

// -- to_list() -> Python list of str --
//
// Collects all elements into a plain Python list. Useful for interop
// with code that expects a regular list.

unsafe extern "C" fn list_to_list(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    let list = get_list(slf);
    let items: Vec<String> = list.to_vec();
    vec_str_to_py(&items)
}

// -- from_list(items) -> ImmutableList (class method) --
//
// Builds an ImmutableList from a Python list of strings. This is the
// preferred way to create a list with initial data.
//
//     lst = ImmutableList.from_list(["a", "b", "c"])

unsafe extern "C" fn list_from_list(_cls: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let arg = PyTuple_GetItem(args, 0);
    if arg.is_null() {
        set_error(
            value_error_class(),
            "from_list requires one list argument",
        );
        return ptr::null_mut();
    }

    match vec_str_from_py(arg) {
        Some(items) => {
            let list = ImmutableList::from_slice(&items);
            wrap_list(list)
        }
        None => {
            set_error(
                value_error_class(),
                "from_list requires a list of strings",
            );
            ptr::null_mut()
        }
    }
}

// ---------------------------------------------------------------------------
// Leaked CString helper (method tables need static lifetime)
// ---------------------------------------------------------------------------

fn cstr(s: &str) -> *const c_char {
    CString::new(s).expect("no NUL").into_raw()
}

// ---------------------------------------------------------------------------
// Module init: PyInit_immutable_list_native
// ---------------------------------------------------------------------------
//
// This is the entry point called by Python's import machinery when
// `import immutable_list_native` is executed. It:
//
// 1. Defines the method table (push, get, set, pop, len, is_empty, etc.)
// 2. Defines the type slots (dealloc, repr, richcompare, iter, etc.)
// 3. Creates the type object via PyType_FromSpec
// 4. Creates the module and adds the type + exception to it
//
// The naming convention PyInit_<module_name> is mandatory for CPython
// to find and call this function when loading the shared library.

#[no_mangle]
pub unsafe extern "C" fn PyInit_immutable_list_native() -> PyObjectPtr {
    // -- Method table -------------------------------------------------------
    //
    // 8 methods + 1 sentinel = 9 entries
    //
    //  0: push       (VARARGS)     -- append value, return new list
    //  1: get        (VARARGS)     -- get element by index
    //  2: set        (VARARGS)     -- replace element, return new list
    //  3: pop        (NOARGS)      -- remove last, return (new_list, value)
    //  4: len        (NOARGS)      -- element count
    //  5: is_empty   (NOARGS)      -- True if empty
    //  6: to_list    (NOARGS)      -- convert to Python list
    //  7: from_list  (VARARGS|CLASS) -- build from Python list
    //  8: sentinel

    static mut METHODS: [PyMethodDef; 9] = [
        PyMethodDef {
            ml_name: ptr::null(),
            ml_meth: None,
            ml_flags: 0,
            ml_doc: ptr::null(),
        }; 9
    ];

    METHODS[0] = PyMethodDef {
        ml_name: cstr("push"),
        ml_meth: Some(list_push),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[1] = PyMethodDef {
        ml_name: cstr("get"),
        ml_meth: Some(list_get),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[2] = PyMethodDef {
        ml_name: cstr("set"),
        ml_meth: Some(list_set),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[3] = PyMethodDef {
        ml_name: cstr("pop"),
        ml_meth: Some(list_pop),
        ml_flags: METH_NOARGS,
        ml_doc: ptr::null(),
    };
    METHODS[4] = PyMethodDef {
        ml_name: cstr("len"),
        ml_meth: Some(list_len_method),
        ml_flags: METH_NOARGS,
        ml_doc: ptr::null(),
    };
    METHODS[5] = PyMethodDef {
        ml_name: cstr("is_empty"),
        ml_meth: Some(list_is_empty),
        ml_flags: METH_NOARGS,
        ml_doc: ptr::null(),
    };
    METHODS[6] = PyMethodDef {
        ml_name: cstr("to_list"),
        ml_meth: Some(list_to_list),
        ml_flags: METH_NOARGS,
        ml_doc: ptr::null(),
    };
    METHODS[7] = PyMethodDef {
        ml_name: cstr("from_list"),
        ml_meth: Some(list_from_list),
        ml_flags: METH_VARARGS | METH_CLASS,
        ml_doc: ptr::null(),
    };
    METHODS[8] = method_def_sentinel();

    // -- Type slots ----------------------------------------------------------
    //
    // These define the special protocol methods (dunder methods) that Python
    // uses for operators, len(), iteration, etc.
    //
    // Slot numbers MUST match CPython's typeslots.h exactly. See the
    // lessons.md entry about slot number crashes.

    static mut SLOTS: [PyType_Slot; 9] = [
        PyType_Slot {
            slot: 0,
            pfunc: ptr::null_mut(),
        }; 9
    ];

    SLOTS[0] = PyType_Slot { slot: PY_TP_NEW, pfunc: list_new as *mut c_void };
    SLOTS[1] = PyType_Slot { slot: PY_TP_DEALLOC, pfunc: list_dealloc as *mut c_void };
    SLOTS[2] = PyType_Slot { slot: PY_TP_METHODS, pfunc: (&raw mut METHODS) as *mut c_void };
    SLOTS[3] = PyType_Slot { slot: PY_TP_REPR, pfunc: list_repr as *mut c_void };
    SLOTS[4] = PyType_Slot { slot: PY_TP_RICHCOMPARE, pfunc: list_richcompare as *mut c_void };
    SLOTS[5] = PyType_Slot { slot: PY_TP_ITER, pfunc: list_iter as *mut c_void };
    SLOTS[6] = PyType_Slot { slot: PY_SQ_LENGTH, pfunc: list_sq_length as *mut c_void };
    SLOTS[7] = PyType_Slot { slot: PY_MP_SUBSCRIPT, pfunc: list_mp_subscript as *mut c_void };
    SLOTS[8] = type_slot_sentinel();

    // -- Type spec -----------------------------------------------------------
    static mut SPEC: PyType_Spec = PyType_Spec {
        name: ptr::null(),
        basicsize: 0,
        itemsize: 0,
        flags: 0,
        slots: ptr::null_mut(),
    };

    SPEC.name = cstr("immutable_list_native.ImmutableList");
    SPEC.basicsize = std::mem::size_of::<ListObject>() as c_int;
    SPEC.flags = PY_TPFLAGS_DEFAULT;
    SPEC.slots = (&raw mut SLOTS) as *mut PyType_Slot;

    let type_obj = PyType_FromSpec(&raw mut SPEC);
    if type_obj.is_null() {
        return ptr::null_mut();
    }

    // Store the type object globally so wrap_list() and is_list() can use it
    LIST_TYPE = type_obj;

    // -- Module definition ---------------------------------------------------
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
    MODULE_DEF.m_name = cstr("immutable_list_native");

    let module = PyModule_Create2(&raw mut MODULE_DEF, PYTHON_API_VERSION);
    if module.is_null() {
        return ptr::null_mut();
    }

    // -- Add class to module -------------------------------------------------
    Py_IncRef(type_obj);
    module_add_object(module, "ImmutableList", type_obj);

    // -- Create exception class ----------------------------------------------
    LIST_ERROR = new_exception(
        "immutable_list_native",
        "ImmutableListError",
        exception_class(),
    );
    Py_IncRef(LIST_ERROR);
    module_add_object(module, "ImmutableListError", LIST_ERROR);

    module
}
