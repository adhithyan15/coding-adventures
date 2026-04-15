use std::ffi::{c_char, c_int, c_void, CString};
use std::ptr;

use python_bridge::*;
use radix_tree::RadixTree as CoreRadixTree;

const PY_TP_DEALLOC: c_int = 52;
const PY_TP_ITER: c_int = 62;
const PY_TP_METHODS: c_int = 64;
const PY_TP_NEW: c_int = 65;
const PY_TP_REPR: c_int = 66;
const PY_SQ_CONTAINS: c_int = 41;
const PY_SQ_LENGTH: c_int = 45;

extern "C" {
    fn PyType_GenericAlloc(type_obj: PyObjectPtr, nitems: isize) -> PyObjectPtr;
    fn PyObject_Free(ptr: *mut c_void);
    fn PyDict_New() -> PyObjectPtr;
    fn PyDict_SetItem(dict: PyObjectPtr, key: PyObjectPtr, value: PyObjectPtr) -> c_int;
}

struct PyValue(PyObjectPtr);

impl PyValue {
    unsafe fn from_borrowed(obj: PyObjectPtr) -> Self {
        Py_IncRef(obj);
        Self(obj)
    }

    unsafe fn to_object(&self) -> PyObjectPtr {
        Py_IncRef(self.0);
        self.0
    }
}

impl Clone for PyValue {
    fn clone(&self) -> Self {
        unsafe {
            Py_IncRef(self.0);
        }
        Self(self.0)
    }
}

impl Drop for PyValue {
    fn drop(&mut self) {
        unsafe {
            Py_DecRef(self.0);
        }
    }
}

#[repr(C)]
struct RadixTreeObject {
    ob_refcnt: isize,
    ob_type: PyObjectPtr,
    tree: *mut CoreRadixTree<PyValue>,
}

unsafe fn get_tree(slf: PyObjectPtr) -> &'static CoreRadixTree<PyValue> {
    &*((slf as *mut RadixTreeObject).read().tree)
}

unsafe fn get_tree_mut(slf: PyObjectPtr) -> &'static mut CoreRadixTree<PyValue> {
    &mut *(*(slf as *mut RadixTreeObject)).tree
}

unsafe fn list_of_strings(values: &[String]) -> PyObjectPtr {
    vec_str_to_py(values)
}

unsafe fn dict_from_entries(entries: Vec<(String, PyValue)>) -> PyObjectPtr {
    let dict = PyDict_New();
    for (key, value) in entries {
        let py_key = str_to_py(&key);
        let py_value = value.to_object();
        PyDict_SetItem(dict, py_key, py_value);
        Py_DecRef(py_key);
        Py_DecRef(py_value);
    }
    dict
}

unsafe extern "C" fn radix_tree_new(
    type_obj: PyObjectPtr,
    _args: PyObjectPtr,
    _kwargs: PyObjectPtr,
) -> PyObjectPtr {
    let obj = PyType_GenericAlloc(type_obj, 0);
    if obj.is_null() {
        return ptr::null_mut();
    }
    (*(obj as *mut RadixTreeObject)).tree = Box::into_raw(Box::new(CoreRadixTree::new()));
    obj
}

unsafe extern "C" fn radix_tree_dealloc(obj: PyObjectPtr) {
    let tree_obj = obj as *mut RadixTreeObject;
    if !(*tree_obj).tree.is_null() {
        let _ = Box::from_raw((*tree_obj).tree);
        (*tree_obj).tree = ptr::null_mut();
    }
    PyObject_Free(obj as *mut c_void);
}

unsafe extern "C" fn radix_tree_sq_length(slf: PyObjectPtr) -> isize {
    get_tree(slf).len() as isize
}

unsafe extern "C" fn radix_tree_sq_contains(slf: PyObjectPtr, key: PyObjectPtr) -> c_int {
    match str_from_py(key) {
        Some(key) => {
            if get_tree(slf).contains_key(&key) {
                1
            } else {
                0
            }
        }
        None => 0,
    }
}

unsafe extern "C" fn radix_tree_insert(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let Some(key) = parse_arg_str(args, 0) else {
        set_error(type_error_class(), "insert() requires a string key");
        return ptr::null_mut();
    };
    let value = PyTuple_GetItem(args, 1);
    if value.is_null() {
        set_error(type_error_class(), "insert() requires a value");
        return ptr::null_mut();
    }
    get_tree_mut(slf).insert(&key, PyValue::from_borrowed(value));
    py_none()
}

unsafe extern "C" fn radix_tree_search(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let Some(key) = parse_arg_str(args, 0) else {
        set_error(type_error_class(), "search() requires a string key");
        return ptr::null_mut();
    };
    match get_tree(slf).search(&key) {
        Some(value) => value.to_object(),
        None => py_none(),
    }
}

unsafe extern "C" fn radix_tree_delete(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let Some(key) = parse_arg_str(args, 0) else {
        set_error(type_error_class(), "delete() requires a string key");
        return ptr::null_mut();
    };
    bool_to_py(get_tree_mut(slf).delete(&key))
}

unsafe extern "C" fn radix_tree_starts_with(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let Some(prefix) = parse_arg_str(args, 0) else {
        set_error(type_error_class(), "starts_with() requires a string prefix");
        return ptr::null_mut();
    };
    bool_to_py(get_tree(slf).starts_with(&prefix))
}

unsafe extern "C" fn radix_tree_words_with_prefix(
    slf: PyObjectPtr,
    args: PyObjectPtr,
) -> PyObjectPtr {
    let Some(prefix) = parse_arg_str(args, 0) else {
        set_error(type_error_class(), "words_with_prefix() requires a string prefix");
        return ptr::null_mut();
    };
    let words = get_tree(slf).words_with_prefix(&prefix);
    list_of_strings(&words)
}

unsafe extern "C" fn radix_tree_longest_prefix_match(
    slf: PyObjectPtr,
    args: PyObjectPtr,
) -> PyObjectPtr {
    let Some(key) = parse_arg_str(args, 0) else {
        set_error(
            type_error_class(),
            "longest_prefix_match() requires a string input",
        );
        return ptr::null_mut();
    };
    match get_tree(slf).longest_prefix_match(&key) {
        Some(value) => str_to_py(&value),
        None => py_none(),
    }
}

unsafe extern "C" fn radix_tree_to_dict(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    let entries: Vec<_> = get_tree(slf).to_map().into_iter().collect();
    dict_from_entries(entries)
}

unsafe extern "C" fn radix_tree_node_count(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    usize_to_py(get_tree(slf).node_count())
}

unsafe extern "C" fn radix_tree_repr(slf: PyObjectPtr) -> PyObjectPtr {
    str_to_py(&format!("RadixTree({} keys)", get_tree(slf).len()))
}

unsafe extern "C" fn radix_tree_iter(slf: PyObjectPtr) -> PyObjectPtr {
    let keys = get_tree(slf).keys();
    let list = vec_str_to_py(&keys);
    let iter = PyObject_GetIter(list);
    Py_DecRef(list);
    iter
}

fn cstr(value: &str) -> *const c_char {
    CString::new(value).expect("no NUL").into_raw()
}

#[no_mangle]
pub unsafe extern "C" fn PyInit_radix_tree_native() -> PyObjectPtr {
    static mut METHODS: [PyMethodDef; 9] = [method_def_sentinel(); 9];
    METHODS[0] = PyMethodDef {
        ml_name: cstr("insert"),
        ml_meth: Some(radix_tree_insert),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[1] = PyMethodDef {
        ml_name: cstr("search"),
        ml_meth: Some(radix_tree_search),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[2] = PyMethodDef {
        ml_name: cstr("delete"),
        ml_meth: Some(radix_tree_delete),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[3] = PyMethodDef {
        ml_name: cstr("starts_with"),
        ml_meth: Some(radix_tree_starts_with),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[4] = PyMethodDef {
        ml_name: cstr("words_with_prefix"),
        ml_meth: Some(radix_tree_words_with_prefix),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[5] = PyMethodDef {
        ml_name: cstr("longest_prefix_match"),
        ml_meth: Some(radix_tree_longest_prefix_match),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[6] = PyMethodDef {
        ml_name: cstr("to_dict"),
        ml_meth: Some(radix_tree_to_dict),
        ml_flags: METH_NOARGS,
        ml_doc: ptr::null(),
    };
    METHODS[7] = PyMethodDef {
        ml_name: cstr("node_count"),
        ml_meth: Some(radix_tree_node_count),
        ml_flags: METH_NOARGS,
        ml_doc: ptr::null(),
    };
    METHODS[8] = method_def_sentinel();

    static mut SLOTS: [PyType_Slot; 8] = [type_slot_sentinel(); 8];
    SLOTS[0] = PyType_Slot {
        slot: PY_TP_NEW,
        pfunc: radix_tree_new as *mut c_void,
    };
    SLOTS[1] = PyType_Slot {
        slot: PY_TP_DEALLOC,
        pfunc: radix_tree_dealloc as *mut c_void,
    };
    SLOTS[2] = PyType_Slot {
        slot: PY_TP_METHODS,
        pfunc: (&raw mut METHODS) as *mut c_void,
    };
    SLOTS[3] = PyType_Slot {
        slot: PY_TP_REPR,
        pfunc: radix_tree_repr as *mut c_void,
    };
    SLOTS[4] = PyType_Slot {
        slot: PY_TP_ITER,
        pfunc: radix_tree_iter as *mut c_void,
    };
    SLOTS[5] = PyType_Slot {
        slot: PY_SQ_LENGTH,
        pfunc: radix_tree_sq_length as *mut c_void,
    };
    SLOTS[6] = PyType_Slot {
        slot: PY_SQ_CONTAINS,
        pfunc: radix_tree_sq_contains as *mut c_void,
    };
    SLOTS[7] = type_slot_sentinel();

    static mut SPEC: PyType_Spec = PyType_Spec {
        name: ptr::null(),
        basicsize: 0,
        itemsize: 0,
        flags: 0,
        slots: ptr::null_mut(),
    };
    SPEC.name = cstr("radix_tree_native.RadixTree");
    SPEC.basicsize = std::mem::size_of::<RadixTreeObject>() as c_int;
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
    MODULE_DEF.m_name = cstr("radix_tree_native");

    let module = PyModule_Create2(&raw mut MODULE_DEF, PYTHON_API_VERSION);
    if module.is_null() {
        return ptr::null_mut();
    }

    Py_IncRef(type_obj);
    module_add_object(module, "RadixTree", type_obj);

    module
}
