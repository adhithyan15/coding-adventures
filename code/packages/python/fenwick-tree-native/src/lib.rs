use std::ffi::{c_char, c_double, c_int, c_long, c_void, CString};
use std::ptr;

use fenwick_tree::{FenwickError as CoreFenwickError, FenwickTree as CoreFenwickTree};
use python_bridge::*;

const PY_TP_DEALLOC: c_int = 52;
const PY_TP_METHODS: c_int = 64;
const PY_TP_NEW: c_int = 65;
const PY_SQ_LENGTH: c_int = 45;
const METH_CLASS: c_int = 0x0010;

extern "C" {
    fn PyType_GenericAlloc(type_obj: PyObjectPtr, nitems: isize) -> PyObjectPtr;
    fn PyObject_Free(ptr: *mut c_void);
    fn PyTuple_Size(tuple: PyObjectPtr) -> isize;
    fn PyLong_AsLong(obj: PyObjectPtr) -> c_long;
    fn PyFloat_AsDouble(obj: PyObjectPtr) -> c_double;
    fn PyFloat_FromDouble(value: c_double) -> PyObjectPtr;
}

#[repr(C)]
struct FenwickTreeObject {
    ob_refcnt: isize,
    ob_type: PyObjectPtr,
    tree: *mut CoreFenwickTree,
}

static mut FENWICK_ERROR: PyObjectPtr = ptr::null_mut();
static mut INDEX_OUT_OF_RANGE_ERROR: PyObjectPtr = ptr::null_mut();
static mut EMPTY_TREE_ERROR: PyObjectPtr = ptr::null_mut();

unsafe fn get_tree(slf: PyObjectPtr) -> &'static CoreFenwickTree {
    &*((slf as *mut FenwickTreeObject).read().tree)
}

unsafe fn get_tree_mut(slf: PyObjectPtr) -> &'static mut CoreFenwickTree {
    &mut *(*(slf as *mut FenwickTreeObject)).tree
}

unsafe fn float_to_py(value: f64) -> PyObjectPtr {
    PyFloat_FromDouble(value)
}

unsafe fn parse_usize_arg(args: PyObjectPtr, index: isize, what: &str) -> Option<usize> {
    let obj = PyTuple_GetItem(args, index);
    if obj.is_null() {
        set_error(type_error_class(), &format!("{what} is required"));
        return None;
    }
    let value = PyLong_AsLong(obj);
    if value < 0 {
        set_error(type_error_class(), &format!("{what} must be non-negative"));
        return None;
    }
    Some(value as usize)
}

unsafe fn parse_f64_arg(args: PyObjectPtr, index: isize, what: &str) -> Option<f64> {
    let obj = PyTuple_GetItem(args, index);
    if obj.is_null() {
        set_error(type_error_class(), &format!("{what} is required"));
        return None;
    }
    Some(PyFloat_AsDouble(obj))
}

unsafe fn iterable_to_f64_vec(obj: PyObjectPtr) -> Option<Vec<f64>> {
    let iter = PyObject_GetIter(obj);
    if iter.is_null() {
        set_error(type_error_class(), "expected an iterable of numbers");
        return None;
    }

    let mut values = Vec::new();
    loop {
        let item = PyIter_Next(iter);
        if item.is_null() {
            break;
        }
        values.push(PyFloat_AsDouble(item));
        Py_DecRef(item);
    }
    Py_DecRef(iter);
    Some(values)
}

unsafe fn set_fenwick_error(error: CoreFenwickError) -> PyObjectPtr {
    match error {
        CoreFenwickError::IndexOutOfRange { index, min, max } => {
            set_error(
                INDEX_OUT_OF_RANGE_ERROR,
                &format!("index {index} out of range [{min}, {max}]"),
            );
        }
        CoreFenwickError::InvalidRange { left, right } => {
            set_error(FENWICK_ERROR, &format!("left ({left}) must be <= right ({right})"));
        }
        CoreFenwickError::EmptyTree => {
            set_error(EMPTY_TREE_ERROR, "find_kth called on empty tree");
        }
        CoreFenwickError::NonPositiveTarget { target } => {
            set_error(FENWICK_ERROR, &format!("k must be positive, got {target}"));
        }
        CoreFenwickError::TargetExceedsTotal { target, total } => {
            set_error(
                FENWICK_ERROR,
                &format!("k ({target}) exceeds total sum of the tree ({total})"),
            );
        }
    }
    ptr::null_mut()
}

unsafe extern "C" fn fenwick_tree_new(
    type_obj: PyObjectPtr,
    args: PyObjectPtr,
    _kwargs: PyObjectPtr,
) -> PyObjectPtr {
    let nargs = PyTuple_Size(args);
    if nargs != 1 {
        set_error(type_error_class(), "FenwickTree() requires a size argument");
        return ptr::null_mut();
    }
    let Some(size) = parse_usize_arg(args, 0, "size") else {
        return ptr::null_mut();
    };

    let obj = PyType_GenericAlloc(type_obj, 0);
    if obj.is_null() {
        return ptr::null_mut();
    }
    (*(obj as *mut FenwickTreeObject)).tree = Box::into_raw(Box::new(CoreFenwickTree::new(size)));
    obj
}

unsafe extern "C" fn fenwick_tree_dealloc(obj: PyObjectPtr) {
    let tree_obj = obj as *mut FenwickTreeObject;
    if !(*tree_obj).tree.is_null() {
        let _ = Box::from_raw((*tree_obj).tree);
        (*tree_obj).tree = ptr::null_mut();
    }
    PyObject_Free(obj as *mut c_void);
}

unsafe extern "C" fn fenwick_tree_sq_length(slf: PyObjectPtr) -> isize {
    get_tree(slf).len() as isize
}

unsafe extern "C" fn fenwick_tree_from_list(
    _cls: PyObjectPtr,
    args: PyObjectPtr,
) -> PyObjectPtr {
    let obj = PyTuple_GetItem(args, 0);
    if obj.is_null() {
        set_error(type_error_class(), "from_list() requires an iterable");
        return ptr::null_mut();
    }
    let Some(values) = iterable_to_f64_vec(obj) else {
        return ptr::null_mut();
    };

    let tree = CoreFenwickTree::from_iterable(values);
    let type_obj = PyObject_GetAttrString(_cls, CString::new("__class__").unwrap().as_ptr());
    if !type_obj.is_null() {
        Py_DecRef(type_obj);
    }
    let result = PyType_GenericAlloc(_cls, 0);
    if result.is_null() {
        return ptr::null_mut();
    }
    (*(result as *mut FenwickTreeObject)).tree = Box::into_raw(Box::new(tree));
    result
}

unsafe extern "C" fn fenwick_tree_update(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let Some(index) = parse_usize_arg(args, 0, "index") else {
        return ptr::null_mut();
    };
    let Some(delta) = parse_f64_arg(args, 1, "delta") else {
        return ptr::null_mut();
    };
    match get_tree_mut(slf).update(index, delta) {
        Ok(()) => py_none(),
        Err(error) => set_fenwick_error(error),
    }
}

unsafe extern "C" fn fenwick_tree_prefix_sum(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let Some(index) = parse_usize_arg(args, 0, "index") else {
        return ptr::null_mut();
    };
    match get_tree(slf).prefix_sum(index) {
        Ok(value) => float_to_py(value),
        Err(error) => set_fenwick_error(error),
    }
}

unsafe extern "C" fn fenwick_tree_range_sum(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let Some(left) = parse_usize_arg(args, 0, "left") else {
        return ptr::null_mut();
    };
    let Some(right) = parse_usize_arg(args, 1, "right") else {
        return ptr::null_mut();
    };
    match get_tree(slf).range_sum(left, right) {
        Ok(value) => float_to_py(value),
        Err(error) => set_fenwick_error(error),
    }
}

unsafe extern "C" fn fenwick_tree_point_query(
    slf: PyObjectPtr,
    args: PyObjectPtr,
) -> PyObjectPtr {
    let Some(index) = parse_usize_arg(args, 0, "index") else {
        return ptr::null_mut();
    };
    match get_tree(slf).point_query(index) {
        Ok(value) => float_to_py(value),
        Err(error) => set_fenwick_error(error),
    }
}

unsafe extern "C" fn fenwick_tree_find_kth(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let Some(target) = parse_f64_arg(args, 0, "k") else {
        return ptr::null_mut();
    };
    match get_tree(slf).find_kth(target) {
        Ok(index) => usize_to_py(index),
        Err(error) => set_fenwick_error(error),
    }
}

fn cstr(value: &str) -> *const c_char {
    CString::new(value).expect("no NUL").into_raw()
}

#[no_mangle]
pub unsafe extern "C" fn PyInit_fenwick_tree_native() -> PyObjectPtr {
    static mut METHODS: [PyMethodDef; 7] = [method_def_sentinel(); 7];
    METHODS[0] = PyMethodDef {
        ml_name: cstr("from_list"),
        ml_meth: Some(fenwick_tree_from_list),
        ml_flags: METH_VARARGS | METH_CLASS,
        ml_doc: ptr::null(),
    };
    METHODS[1] = PyMethodDef {
        ml_name: cstr("update"),
        ml_meth: Some(fenwick_tree_update),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[2] = PyMethodDef {
        ml_name: cstr("prefix_sum"),
        ml_meth: Some(fenwick_tree_prefix_sum),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[3] = PyMethodDef {
        ml_name: cstr("range_sum"),
        ml_meth: Some(fenwick_tree_range_sum),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[4] = PyMethodDef {
        ml_name: cstr("point_query"),
        ml_meth: Some(fenwick_tree_point_query),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[5] = PyMethodDef {
        ml_name: cstr("find_kth"),
        ml_meth: Some(fenwick_tree_find_kth),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[6] = method_def_sentinel();

    static mut SLOTS: [PyType_Slot; 5] = [type_slot_sentinel(); 5];
    SLOTS[0] = PyType_Slot {
        slot: PY_TP_NEW,
        pfunc: fenwick_tree_new as *mut c_void,
    };
    SLOTS[1] = PyType_Slot {
        slot: PY_TP_DEALLOC,
        pfunc: fenwick_tree_dealloc as *mut c_void,
    };
    SLOTS[2] = PyType_Slot {
        slot: PY_TP_METHODS,
        pfunc: (&raw mut METHODS) as *mut c_void,
    };
    SLOTS[3] = PyType_Slot {
        slot: PY_SQ_LENGTH,
        pfunc: fenwick_tree_sq_length as *mut c_void,
    };
    SLOTS[4] = type_slot_sentinel();

    static mut SPEC: PyType_Spec = PyType_Spec {
        name: ptr::null(),
        basicsize: 0,
        itemsize: 0,
        flags: 0,
        slots: ptr::null_mut(),
    };
    SPEC.name = cstr("fenwick_tree_native.FenwickTree");
    SPEC.basicsize = std::mem::size_of::<FenwickTreeObject>() as c_int;
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
    MODULE_DEF.m_name = cstr("fenwick_tree_native");

    let module = PyModule_Create2(&raw mut MODULE_DEF, PYTHON_API_VERSION);
    if module.is_null() {
        return ptr::null_mut();
    }

    Py_IncRef(type_obj);
    module_add_object(module, "FenwickTree", type_obj);

    FENWICK_ERROR = new_exception("fenwick_tree_native", "FenwickError", exception_class());
    INDEX_OUT_OF_RANGE_ERROR =
        new_exception("fenwick_tree_native", "IndexOutOfRangeError", FENWICK_ERROR);
    EMPTY_TREE_ERROR = new_exception("fenwick_tree_native", "EmptyTreeError", FENWICK_ERROR);

    Py_IncRef(FENWICK_ERROR);
    Py_IncRef(INDEX_OUT_OF_RANGE_ERROR);
    Py_IncRef(EMPTY_TREE_ERROR);
    module_add_object(module, "FenwickError", FENWICK_ERROR);
    module_add_object(module, "IndexOutOfRangeError", INDEX_OUT_OF_RANGE_ERROR);
    module_add_object(module, "EmptyTreeError", EMPTY_TREE_ERROR);

    module
}
