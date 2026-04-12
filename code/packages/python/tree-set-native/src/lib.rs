use std::ffi::{c_char, c_int, c_long, c_void, CString};
use std::ptr;

use python_bridge::*;
use tree_set::TreeSet as CoreTreeSet;

const PY_TP_DEALLOC: c_int = 52;
const PY_TP_REPR: c_int = 66;
const PY_TP_METHODS: c_int = 64;
const PY_TP_NEW: c_int = 65;
const PY_TP_ITER: c_int = 62;
const PY_SQ_LENGTH: c_int = 45;
const PY_SQ_CONTAINS: c_int = 41;

#[allow(non_snake_case)]
extern "C" {
    fn PyType_GenericAlloc(type_obj: PyObjectPtr, nitems: isize) -> PyObjectPtr;
    fn PyObject_Free(ptr: *mut c_void);
    fn PyLong_AsLong(obj: PyObjectPtr) -> c_long;
    fn PyTuple_Size(tuple: PyObjectPtr) -> isize;
}

#[repr(C)]
struct TreeSetObject {
    ob_refcnt: isize,
    ob_type: PyObjectPtr,
    tree: *mut CoreTreeSet<i64>,
}

unsafe fn get_tree(slf: PyObjectPtr) -> &'static CoreTreeSet<i64> {
    &*((slf as *mut TreeSetObject).read().tree)
}

unsafe fn get_tree_mut(slf: PyObjectPtr) -> &'static mut CoreTreeSet<i64> {
    &mut *(*(slf as *mut TreeSetObject)).tree
}

fn to_i64(obj: PyObjectPtr) -> i64 {
    unsafe { PyLong_AsLong(obj) as i64 }
}

fn from_i64(value: i64) -> PyObjectPtr {
    unsafe { PyLong_FromLong(value as c_long) }
}

unsafe fn vec_i64_to_py(values: &[i64]) -> PyObjectPtr {
    let list = PyList_New(values.len() as isize);
    for (index, value) in values.iter().enumerate() {
        PyList_SetItem(list, index as isize, from_i64(*value));
    }
    list
}

unsafe fn wrap_tree_like(slf: PyObjectPtr, values: Vec<i64>) -> PyObjectPtr {
    let type_obj = (*(slf as *mut TreeSetObject)).ob_type;
    let obj = PyType_GenericAlloc(type_obj, 0);
    if obj.is_null() {
        return ptr::null_mut();
    }
    (*(obj as *mut TreeSetObject)).tree = Box::into_raw(Box::new(CoreTreeSet::from_list(values)));
    obj
}

unsafe fn values_from_py_iterable(values: PyObjectPtr) -> Option<Vec<i64>> {
    let iter = PyObject_GetIter(values);
    if iter.is_null() {
        PyErr_Clear();
        set_error(type_error_class(), "TreeSet() requires an iterable of numbers");
        return None;
    }

    let mut result = Vec::new();
    loop {
        let item = PyIter_Next(iter);
        if item.is_null() {
            break;
        }
        result.push(to_i64(item));
        Py_DecRef(item);
    }

    Py_DecRef(iter);
    Some(result)
}

unsafe extern "C" fn tree_set_new(
    type_obj: PyObjectPtr,
    _args: PyObjectPtr,
    _kwargs: PyObjectPtr,
) -> PyObjectPtr {
    let argc = PyTuple_Size(_args);
    if argc < 0 {
        PyErr_Clear();
        set_error(type_error_class(), "TreeSet() could not read constructor arguments");
        return ptr::null_mut();
    }
    if argc > 1 {
        PyErr_Clear();
        set_error(type_error_class(), "TreeSet() accepts at most one iterable argument");
        return ptr::null_mut();
    }

    let initial_values = if argc == 1 {
        let values = PyTuple_GetItem(_args, 0);
        if values.is_null() {
            PyErr_Clear();
            set_error(type_error_class(), "TreeSet() accepts one iterable argument");
            return ptr::null_mut();
        }
        match values_from_py_iterable(values) {
            Some(values) => values,
            None => return ptr::null_mut(),
        }
    } else {
        Vec::new()
    };

    let obj = PyType_GenericAlloc(type_obj, 0);
    if obj.is_null() {
        return ptr::null_mut();
    }
    (*(obj as *mut TreeSetObject)).tree = Box::into_raw(Box::new(CoreTreeSet::from_list(initial_values)));
    obj
}

unsafe extern "C" fn tree_set_dealloc(obj: PyObjectPtr) {
    let tree_obj = obj as *mut TreeSetObject;
    if !(*tree_obj).tree.is_null() {
        let _ = Box::from_raw((*tree_obj).tree);
        (*tree_obj).tree = ptr::null_mut();
    }
    PyObject_Free(obj as *mut c_void);
}

unsafe extern "C" fn tree_set_sq_length(slf: PyObjectPtr) -> isize {
    get_tree(slf).len() as isize
}

unsafe extern "C" fn tree_set_sq_contains(slf: PyObjectPtr, key: PyObjectPtr) -> c_int {
    let value = to_i64(key);
    if get_tree(slf).contains(&value) {
        1
    } else {
        0
    }
}

unsafe extern "C" fn tree_set_add(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let value = match PyTuple_GetItem(args, 0) {
        v if v.is_null() => {
            PyErr_Clear();
            set_error(type_error_class(), "add() requires one numeric argument");
            return ptr::null_mut();
        }
        v => to_i64(v),
    };
    let inner = get_tree_mut(slf);
    let current = std::mem::take(inner);
    *inner = current.insert(value);
    py_none()
}

unsafe extern "C" fn tree_set_delete(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let value = match PyTuple_GetItem(args, 0) {
        v if v.is_null() => {
            PyErr_Clear();
            set_error(type_error_class(), "delete() requires one numeric argument");
            return ptr::null_mut();
        }
        v => to_i64(v),
    };
    let inner = get_tree_mut(slf);
    let current = std::mem::take(inner);
    let existed = current.contains(&value);
    *inner = current.delete(&value);
    bool_to_py(existed)
}

unsafe extern "C" fn tree_set_contains(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let value = match PyTuple_GetItem(args, 0) {
        v if v.is_null() => {
            PyErr_Clear();
            set_error(type_error_class(), "contains() requires one numeric argument");
            return ptr::null_mut();
        }
        v => to_i64(v),
    };
    bool_to_py(get_tree(slf).contains(&value))
}

unsafe extern "C" fn tree_set_size(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    usize_to_py(get_tree(slf).size())
}

unsafe extern "C" fn tree_set_is_empty(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    bool_to_py(get_tree(slf).is_empty())
}

unsafe extern "C" fn tree_set_min_value(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    match get_tree(slf).min_value() {
        Some(value) => from_i64(*value),
        None => py_none(),
    }
}

unsafe extern "C" fn tree_set_max_value(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    match get_tree(slf).max_value() {
        Some(value) => from_i64(*value),
        None => py_none(),
    }
}

unsafe extern "C" fn tree_set_predecessor(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let value = match PyTuple_GetItem(args, 0) {
        v if v.is_null() => {
            PyErr_Clear();
            set_error(type_error_class(), "predecessor() requires one numeric argument");
            return ptr::null_mut();
        }
        v => to_i64(v),
    };
    match get_tree(slf).predecessor(&value) {
        Some(found) => from_i64(*found),
        None => py_none(),
    }
}

unsafe extern "C" fn tree_set_successor(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let value = match PyTuple_GetItem(args, 0) {
        v if v.is_null() => {
            PyErr_Clear();
            set_error(type_error_class(), "successor() requires one numeric argument");
            return ptr::null_mut();
        }
        v => to_i64(v),
    };
    match get_tree(slf).successor(&value) {
        Some(found) => from_i64(*found),
        None => py_none(),
    }
}

unsafe extern "C" fn tree_set_rank(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let value = match PyTuple_GetItem(args, 0) {
        v if v.is_null() => {
            PyErr_Clear();
            set_error(type_error_class(), "rank() requires one numeric argument");
            return ptr::null_mut();
        }
        v => to_i64(v),
    };
    usize_to_py(get_tree(slf).rank(&value))
}

unsafe extern "C" fn tree_set_by_rank(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let rank = match PyTuple_GetItem(args, 0) {
        v if v.is_null() => {
            PyErr_Clear();
            set_error(type_error_class(), "by_rank() requires one numeric argument");
            return ptr::null_mut();
        }
        v => to_i64(v) as usize,
    };
    match get_tree(slf).to_sorted_array().get(rank) {
        Some(found) => from_i64(*found),
        None => py_none(),
    }
}

unsafe extern "C" fn tree_set_kth_smallest(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let k = match PyTuple_GetItem(args, 0) {
        v if v.is_null() => {
            PyErr_Clear();
            set_error(type_error_class(), "kth_smallest() requires one numeric argument");
            return ptr::null_mut();
        }
        v => to_i64(v) as usize,
    };
    match get_tree(slf).kth_smallest(k) {
        Some(found) => from_i64(*found),
        None => py_none(),
    }
}

unsafe extern "C" fn tree_set_to_sorted_array(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    vec_i64_to_py(&get_tree(slf).to_sorted_array())
}

unsafe extern "C" fn tree_set_range(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let min = match PyTuple_GetItem(args, 0) {
        v if v.is_null() => {
            PyErr_Clear();
            set_error(type_error_class(), "range() requires a minimum and maximum argument");
            return ptr::null_mut();
        }
        v => to_i64(v),
    };
    let max = match PyTuple_GetItem(args, 1) {
        v if v.is_null() => {
            PyErr_Clear();
            set_error(type_error_class(), "range() requires a maximum argument");
            return ptr::null_mut();
        }
        v => to_i64(v),
    };
    let inclusive = match PyTuple_GetItem(args, 2) {
        v if v.is_null() => {
            PyErr_Clear();
            true
        }
        v => to_i64(v) != 0,
    };
    vec_i64_to_py(&get_tree(slf).range(&min, &max, inclusive))
}

unsafe extern "C" fn tree_set_union(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let other = PyTuple_GetItem(args, 0);
    if other.is_null() {
        PyErr_Clear();
        set_error(type_error_class(), "union() requires a TreeSet argument");
        return ptr::null_mut();
    }
    wrap_tree_like(
        slf,
        get_tree(slf)
            .union(unsafe { get_tree(other) })
            .to_sorted_array(),
    )
}

unsafe extern "C" fn tree_set_intersection(
    slf: PyObjectPtr,
    args: PyObjectPtr,
) -> PyObjectPtr {
    let other = PyTuple_GetItem(args, 0);
    if other.is_null() {
        PyErr_Clear();
        set_error(type_error_class(), "intersection() requires a TreeSet argument");
        return ptr::null_mut();
    }
    wrap_tree_like(
        slf,
        get_tree(slf)
            .intersection(unsafe { get_tree(other) })
            .to_sorted_array(),
    )
}

unsafe extern "C" fn tree_set_difference(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let other = PyTuple_GetItem(args, 0);
    if other.is_null() {
        PyErr_Clear();
        set_error(type_error_class(), "difference() requires a TreeSet argument");
        return ptr::null_mut();
    }
    wrap_tree_like(
        slf,
        get_tree(slf)
            .difference(unsafe { get_tree(other) })
            .to_sorted_array(),
    )
}

unsafe extern "C" fn tree_set_symmetric_difference(
    slf: PyObjectPtr,
    args: PyObjectPtr,
) -> PyObjectPtr {
    let other = PyTuple_GetItem(args, 0);
    if other.is_null() {
        PyErr_Clear();
        set_error(
            type_error_class(),
            "symmetric_difference() requires a TreeSet argument",
        );
        return ptr::null_mut();
    }
    wrap_tree_like(
        slf,
        get_tree(slf)
            .symmetric_difference(unsafe { get_tree(other) })
            .to_sorted_array(),
    )
}

unsafe extern "C" fn tree_set_is_subset(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let other = PyTuple_GetItem(args, 0);
    if other.is_null() {
        PyErr_Clear();
        set_error(type_error_class(), "is_subset() requires a TreeSet argument");
        return ptr::null_mut();
    }
    bool_to_py(get_tree(slf).is_subset(unsafe { get_tree(other) }))
}

unsafe extern "C" fn tree_set_is_superset(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let other = PyTuple_GetItem(args, 0);
    if other.is_null() {
        PyErr_Clear();
        set_error(type_error_class(), "is_superset() requires a TreeSet argument");
        return ptr::null_mut();
    }
    bool_to_py(get_tree(slf).is_superset(unsafe { get_tree(other) }))
}

unsafe extern "C" fn tree_set_is_disjoint(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let other = PyTuple_GetItem(args, 0);
    if other.is_null() {
        PyErr_Clear();
        set_error(type_error_class(), "is_disjoint() requires a TreeSet argument");
        return ptr::null_mut();
    }
    bool_to_py(get_tree(slf).is_disjoint(unsafe { get_tree(other) }))
}

unsafe extern "C" fn tree_set_equals(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let other = PyTuple_GetItem(args, 0);
    if other.is_null() {
        PyErr_Clear();
        set_error(type_error_class(), "equals() requires a TreeSet argument");
        return ptr::null_mut();
    }
    bool_to_py(get_tree(slf).equals(unsafe { get_tree(other) }))
}

unsafe fn tree_set_repr_text(slf: PyObjectPtr) -> PyObjectPtr {
    str_to_py(&format!("TreeSet({:?})", get_tree(slf).to_sorted_array()))
}

unsafe extern "C" fn tree_set_repr_slot(slf: PyObjectPtr) -> PyObjectPtr {
    tree_set_repr_text(slf)
}

unsafe extern "C" fn tree_set_to_string(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    tree_set_repr_text(slf)
}

unsafe extern "C" fn tree_set_iter(slf: PyObjectPtr) -> PyObjectPtr {
    let list = vec_i64_to_py(&get_tree(slf).to_sorted_array());
    let iter = PyObject_GetIter(list);
    Py_DecRef(list);
    iter
}

fn cstr(value: &str) -> *const c_char {
    CString::new(value).expect("no NUL").into_raw()
}

#[no_mangle]
pub unsafe extern "C" fn PyInit_tree_set_native() -> PyObjectPtr {
    static mut METHODS: [PyMethodDef; 24] = [method_def_sentinel(); 24];
    METHODS[0] = PyMethodDef {
        ml_name: cstr("add"),
        ml_meth: Some(tree_set_add),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[1] = PyMethodDef {
        ml_name: cstr("delete"),
        ml_meth: Some(tree_set_delete),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[2] = PyMethodDef {
        ml_name: cstr("contains"),
        ml_meth: Some(tree_set_contains),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[3] = PyMethodDef {
        ml_name: cstr("size"),
        ml_meth: Some(tree_set_size),
        ml_flags: METH_NOARGS,
        ml_doc: ptr::null(),
    };
    METHODS[4] = PyMethodDef {
        ml_name: cstr("is_empty"),
        ml_meth: Some(tree_set_is_empty),
        ml_flags: METH_NOARGS,
        ml_doc: ptr::null(),
    };
    METHODS[5] = PyMethodDef {
        ml_name: cstr("min_value"),
        ml_meth: Some(tree_set_min_value),
        ml_flags: METH_NOARGS,
        ml_doc: ptr::null(),
    };
    METHODS[6] = PyMethodDef {
        ml_name: cstr("max_value"),
        ml_meth: Some(tree_set_max_value),
        ml_flags: METH_NOARGS,
        ml_doc: ptr::null(),
    };
    METHODS[7] = PyMethodDef {
        ml_name: cstr("predecessor"),
        ml_meth: Some(tree_set_predecessor),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[8] = PyMethodDef {
        ml_name: cstr("successor"),
        ml_meth: Some(tree_set_successor),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[9] = PyMethodDef {
        ml_name: cstr("rank"),
        ml_meth: Some(tree_set_rank),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[10] = PyMethodDef {
        ml_name: cstr("by_rank"),
        ml_meth: Some(tree_set_by_rank),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[11] = PyMethodDef {
        ml_name: cstr("kth_smallest"),
        ml_meth: Some(tree_set_kth_smallest),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[12] = PyMethodDef {
        ml_name: cstr("to_sorted_array"),
        ml_meth: Some(tree_set_to_sorted_array),
        ml_flags: METH_NOARGS,
        ml_doc: ptr::null(),
    };
    METHODS[13] = PyMethodDef {
        ml_name: cstr("range"),
        ml_meth: Some(tree_set_range),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[14] = PyMethodDef {
        ml_name: cstr("union"),
        ml_meth: Some(tree_set_union),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[15] = PyMethodDef {
        ml_name: cstr("intersection"),
        ml_meth: Some(tree_set_intersection),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[16] = PyMethodDef {
        ml_name: cstr("difference"),
        ml_meth: Some(tree_set_difference),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[17] = PyMethodDef {
        ml_name: cstr("symmetric_difference"),
        ml_meth: Some(tree_set_symmetric_difference),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[18] = PyMethodDef {
        ml_name: cstr("is_subset"),
        ml_meth: Some(tree_set_is_subset),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[19] = PyMethodDef {
        ml_name: cstr("is_superset"),
        ml_meth: Some(tree_set_is_superset),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[20] = PyMethodDef {
        ml_name: cstr("is_disjoint"),
        ml_meth: Some(tree_set_is_disjoint),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[21] = PyMethodDef {
        ml_name: cstr("equals"),
        ml_meth: Some(tree_set_equals),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[22] = PyMethodDef {
        ml_name: cstr("to_string"),
        ml_meth: Some(tree_set_to_string),
        ml_flags: METH_NOARGS,
        ml_doc: ptr::null(),
    };
    METHODS[23] = method_def_sentinel();

    static mut SLOTS: [PyType_Slot; 8] = [type_slot_sentinel(); 8];
    SLOTS[0] = PyType_Slot {
        slot: PY_TP_NEW,
        pfunc: tree_set_new as *mut c_void,
    };
    SLOTS[1] = PyType_Slot {
        slot: PY_TP_DEALLOC,
        pfunc: tree_set_dealloc as *mut c_void,
    };
    SLOTS[2] = PyType_Slot {
        slot: PY_TP_METHODS,
        pfunc: (&raw mut METHODS) as *mut c_void,
    };
    SLOTS[3] = PyType_Slot {
        slot: PY_TP_REPR,
        pfunc: tree_set_repr_slot as *mut c_void,
    };
    SLOTS[4] = PyType_Slot {
        slot: PY_TP_ITER,
        pfunc: tree_set_iter as *mut c_void,
    };
    SLOTS[5] = PyType_Slot {
        slot: PY_SQ_LENGTH,
        pfunc: tree_set_sq_length as *mut c_void,
    };
    SLOTS[6] = PyType_Slot {
        slot: PY_SQ_CONTAINS,
        pfunc: tree_set_sq_contains as *mut c_void,
    };
    SLOTS[7] = type_slot_sentinel();

    static mut SPEC: PyType_Spec = PyType_Spec {
        name: ptr::null(),
        basicsize: 0,
        itemsize: 0,
        flags: 0,
        slots: ptr::null_mut(),
    };
    SPEC.name = cstr("tree_set_native.TreeSet");
    SPEC.basicsize = std::mem::size_of::<TreeSetObject>() as c_int;
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
    MODULE_DEF.m_name = cstr("tree_set_native");

    let module = PyModule_Create2(&raw mut MODULE_DEF, PYTHON_API_VERSION);
    if module.is_null() {
        return ptr::null_mut();
    }

    Py_IncRef(type_obj);
    module_add_object(module, "TreeSet", type_obj);

    module
}
