use std::ffi::{c_char, c_int, c_void, CString};
use std::ptr;

use python_bridge::*;
use trie::Trie as CoreTrie;

const PY_TP_DEALLOC: c_int = 52;
const PY_TP_REPR: c_int = 66;
const PY_TP_ITER: c_int = 62;
const PY_TP_METHODS: c_int = 64;
const PY_TP_NEW: c_int = 65;
const PY_SQ_CONTAINS: c_int = 41;
const PY_SQ_LENGTH: c_int = 45;
const PY_MP_ASS_SUBSCRIPT: c_int = 3;
const PY_MP_SUBSCRIPT: c_int = 5;

extern "C" {
    fn PyType_GenericAlloc(type_obj: PyObjectPtr, nitems: isize) -> PyObjectPtr;
    fn PyObject_Free(ptr: *mut c_void);
    fn PyTuple_Size(tuple: PyObjectPtr) -> isize;
}

struct PyValue(PyObjectPtr);

impl PyValue {
    unsafe fn from_borrowed(obj: PyObjectPtr) -> Self {
        Py_IncRef(obj);
        Self(obj)
    }

    unsafe fn from_owned(obj: PyObjectPtr) -> Self {
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
struct TrieObject {
    ob_refcnt: isize,
    ob_type: PyObjectPtr,
    trie: *mut CoreTrie<PyValue>,
}

static mut TRIE_ERROR: PyObjectPtr = ptr::null_mut();
static mut KEY_NOT_FOUND_ERROR: PyObjectPtr = ptr::null_mut();

unsafe fn get_trie(slf: PyObjectPtr) -> &'static CoreTrie<PyValue> {
    &*((slf as *mut TrieObject).read().trie)
}

unsafe fn get_trie_mut(slf: PyObjectPtr) -> &'static mut CoreTrie<PyValue> {
    &mut *(*(slf as *mut TrieObject)).trie
}

unsafe fn entries_to_py(entries: &[(String, PyValue)]) -> PyObjectPtr {
    let list = PyList_New(entries.len() as isize);
    for (index, (key, value)) in entries.iter().enumerate() {
        let tuple = PyTuple_New(2);
        PyTuple_SetItem(tuple, 0, str_to_py(key));
        PyTuple_SetItem(tuple, 1, value.to_object());
        PyList_SetItem(list, index as isize, tuple);
    }
    list
}

unsafe fn key_match_to_py(result: Option<(String, PyValue)>) -> PyObjectPtr {
    match result {
        Some((key, value)) => {
            let tuple = PyTuple_New(2);
            PyTuple_SetItem(tuple, 0, str_to_py(&key));
            PyTuple_SetItem(tuple, 1, value.to_object());
            tuple
        }
        None => py_none(),
    }
}

unsafe fn parse_insert_args(args: PyObjectPtr) -> Option<(String, PyValue)> {
    let nargs = PyTuple_Size(args);
    if !(1..=2).contains(&nargs) {
        set_error(
            type_error_class(),
            "insert() takes one required key and an optional value",
        );
        return None;
    }

    let key = match parse_arg_str(args, 0) {
        Some(key) => key,
        None => {
            set_error(type_error_class(), "key must be a string");
            return None;
        }
    };

    let value = if nargs == 2 {
        let obj = PyTuple_GetItem(args, 1);
        if obj.is_null() {
            set_error(type_error_class(), "value argument missing");
            return None;
        }
        PyValue::from_borrowed(obj)
    } else {
        PyValue::from_owned(py_true())
    };

    Some((key, value))
}

unsafe extern "C" fn trie_new(
    type_obj: PyObjectPtr,
    _args: PyObjectPtr,
    _kwargs: PyObjectPtr,
) -> PyObjectPtr {
    let obj = PyType_GenericAlloc(type_obj, 0);
    if obj.is_null() {
        return ptr::null_mut();
    }
    (*(obj as *mut TrieObject)).trie = Box::into_raw(Box::new(CoreTrie::new()));
    obj
}

unsafe extern "C" fn trie_dealloc(obj: PyObjectPtr) {
    let trie_obj = obj as *mut TrieObject;
    if !(*trie_obj).trie.is_null() {
        let _ = Box::from_raw((*trie_obj).trie);
        (*trie_obj).trie = ptr::null_mut();
    }
    PyObject_Free(obj as *mut c_void);
}

unsafe extern "C" fn trie_sq_length(slf: PyObjectPtr) -> isize {
    get_trie(slf).len() as isize
}

unsafe extern "C" fn trie_sq_contains(slf: PyObjectPtr, key: PyObjectPtr) -> c_int {
    match str_from_py(key) {
        Some(key) => {
            if get_trie(slf).contains_key(&key) {
                1
            } else {
                0
            }
        }
        None => 0,
    }
}

unsafe extern "C" fn trie_mp_subscript(slf: PyObjectPtr, key: PyObjectPtr) -> PyObjectPtr {
    let Some(key) = str_from_py(key) else {
        set_error(type_error_class(), "key must be a string");
        return ptr::null_mut();
    };

    match get_trie(slf).search(&key) {
        Some(value) => value.to_object(),
        None => {
            set_error(KEY_NOT_FOUND_ERROR, &format!("Key not found: {key:?}"));
            ptr::null_mut()
        }
    }
}

unsafe extern "C" fn trie_mp_ass_subscript(
    slf: PyObjectPtr,
    key: PyObjectPtr,
    value: PyObjectPtr,
) -> c_int {
    let Some(key) = str_from_py(key) else {
        set_error(type_error_class(), "key must be a string");
        return -1;
    };

    if value.is_null() {
        if get_trie_mut(slf).delete(&key) {
            0
        } else {
            set_error(KEY_NOT_FOUND_ERROR, &format!("Key not found: {key:?}"));
            -1
        }
    } else {
        get_trie_mut(slf).insert(&key, PyValue::from_borrowed(value));
        0
    }
}

unsafe extern "C" fn trie_insert(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let Some((key, value)) = parse_insert_args(args) else {
        return ptr::null_mut();
    };
    get_trie_mut(slf).insert(&key, value);
    py_none()
}

unsafe extern "C" fn trie_search(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let Some(key) = parse_arg_str(args, 0) else {
        set_error(type_error_class(), "search() requires a string key");
        return ptr::null_mut();
    };

    match get_trie(slf).search(&key) {
        Some(value) => value.to_object(),
        None => py_none(),
    }
}

unsafe extern "C" fn trie_delete(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let Some(key) = parse_arg_str(args, 0) else {
        set_error(type_error_class(), "delete() requires a string key");
        return ptr::null_mut();
    };
    bool_to_py(get_trie_mut(slf).delete(&key))
}

unsafe extern "C" fn trie_starts_with(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let Some(prefix) = parse_arg_str(args, 0) else {
        set_error(type_error_class(), "starts_with() requires a string prefix");
        return ptr::null_mut();
    };
    bool_to_py(get_trie(slf).starts_with(&prefix))
}

unsafe extern "C" fn trie_words_with_prefix(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let Some(prefix) = parse_arg_str(args, 0) else {
        set_error(type_error_class(), "words_with_prefix() requires a string prefix");
        return ptr::null_mut();
    };
    let words = get_trie(slf).words_with_prefix(&prefix);
    entries_to_py(&words)
}

unsafe extern "C" fn trie_longest_prefix_match(
    slf: PyObjectPtr,
    args: PyObjectPtr,
) -> PyObjectPtr {
    let Some(string) = parse_arg_str(args, 0) else {
        set_error(
            type_error_class(),
            "longest_prefix_match() requires a string input",
        );
        return ptr::null_mut();
    };
    key_match_to_py(get_trie(slf).longest_prefix_match(&string))
}

unsafe extern "C" fn trie_all_words(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    let words = get_trie(slf).all_words();
    entries_to_py(&words)
}

unsafe extern "C" fn trie_items(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    trie_all_words(slf, ptr::null_mut())
}

unsafe extern "C" fn trie_is_valid(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    bool_to_py(get_trie(slf).is_valid())
}

unsafe extern "C" fn trie_repr(slf: PyObjectPtr) -> PyObjectPtr {
    str_to_py(&format!("Trie({} keys)", get_trie(slf).len()))
}

unsafe extern "C" fn trie_iter(slf: PyObjectPtr) -> PyObjectPtr {
    let keys = get_trie(slf).keys();
    let list = vec_str_to_py(&keys);
    let iter = PyObject_GetIter(list);
    Py_DecRef(list);
    iter
}

fn cstr(value: &str) -> *const c_char {
    CString::new(value).expect("no NUL").into_raw()
}

#[no_mangle]
pub unsafe extern "C" fn PyInit_trie_native() -> PyObjectPtr {
    static mut METHODS: [PyMethodDef; 10] = [method_def_sentinel(); 10];
    METHODS[0] = PyMethodDef {
        ml_name: cstr("insert"),
        ml_meth: Some(trie_insert),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[1] = PyMethodDef {
        ml_name: cstr("search"),
        ml_meth: Some(trie_search),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[2] = PyMethodDef {
        ml_name: cstr("delete"),
        ml_meth: Some(trie_delete),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[3] = PyMethodDef {
        ml_name: cstr("starts_with"),
        ml_meth: Some(trie_starts_with),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[4] = PyMethodDef {
        ml_name: cstr("words_with_prefix"),
        ml_meth: Some(trie_words_with_prefix),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[5] = PyMethodDef {
        ml_name: cstr("longest_prefix_match"),
        ml_meth: Some(trie_longest_prefix_match),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[6] = PyMethodDef {
        ml_name: cstr("all_words"),
        ml_meth: Some(trie_all_words),
        ml_flags: METH_NOARGS,
        ml_doc: ptr::null(),
    };
    METHODS[7] = PyMethodDef {
        ml_name: cstr("items"),
        ml_meth: Some(trie_items),
        ml_flags: METH_NOARGS,
        ml_doc: ptr::null(),
    };
    METHODS[8] = PyMethodDef {
        ml_name: cstr("is_valid"),
        ml_meth: Some(trie_is_valid),
        ml_flags: METH_NOARGS,
        ml_doc: ptr::null(),
    };
    METHODS[9] = method_def_sentinel();

    static mut SLOTS: [PyType_Slot; 10] = [type_slot_sentinel(); 10];
    SLOTS[0] = PyType_Slot {
        slot: PY_TP_NEW,
        pfunc: trie_new as *mut c_void,
    };
    SLOTS[1] = PyType_Slot {
        slot: PY_TP_DEALLOC,
        pfunc: trie_dealloc as *mut c_void,
    };
    SLOTS[2] = PyType_Slot {
        slot: PY_TP_METHODS,
        pfunc: (&raw mut METHODS) as *mut c_void,
    };
    SLOTS[3] = PyType_Slot {
        slot: PY_TP_REPR,
        pfunc: trie_repr as *mut c_void,
    };
    SLOTS[4] = PyType_Slot {
        slot: PY_TP_ITER,
        pfunc: trie_iter as *mut c_void,
    };
    SLOTS[5] = PyType_Slot {
        slot: PY_SQ_LENGTH,
        pfunc: trie_sq_length as *mut c_void,
    };
    SLOTS[6] = PyType_Slot {
        slot: PY_SQ_CONTAINS,
        pfunc: trie_sq_contains as *mut c_void,
    };
    SLOTS[7] = PyType_Slot {
        slot: PY_MP_SUBSCRIPT,
        pfunc: trie_mp_subscript as *mut c_void,
    };
    SLOTS[8] = PyType_Slot {
        slot: PY_MP_ASS_SUBSCRIPT,
        pfunc: trie_mp_ass_subscript as *mut c_void,
    };
    SLOTS[9] = type_slot_sentinel();

    static mut SPEC: PyType_Spec = PyType_Spec {
        name: ptr::null(),
        basicsize: 0,
        itemsize: 0,
        flags: 0,
        slots: ptr::null_mut(),
    };
    SPEC.name = cstr("trie_native.Trie");
    SPEC.basicsize = std::mem::size_of::<TrieObject>() as c_int;
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
    MODULE_DEF.m_name = cstr("trie_native");

    let module = PyModule_Create2(&raw mut MODULE_DEF, PYTHON_API_VERSION);
    if module.is_null() {
        return ptr::null_mut();
    }

    Py_IncRef(type_obj);
    module_add_object(module, "Trie", type_obj);

    TRIE_ERROR = new_exception("trie_native", "TrieError", exception_class());
    KEY_NOT_FOUND_ERROR = new_exception("trie_native", "KeyNotFoundError", TRIE_ERROR);
    Py_IncRef(TRIE_ERROR);
    Py_IncRef(KEY_NOT_FOUND_ERROR);
    module_add_object(module, "TrieError", TRIE_ERROR);
    module_add_object(module, "KeyNotFoundError", KEY_NOT_FOUND_ERROR);

    module
}
