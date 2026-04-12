use std::cmp::Ordering;
use std::ffi::{c_char, c_int, c_long, c_void, CString};
use std::ptr;

use heap::{
    heap_sort as core_heap_sort, heapify as core_heapify, nlargest as core_nlargest,
    nsmallest as core_nsmallest, MaxHeap as CoreMaxHeap, MinHeap as CoreMinHeap,
};
use python_bridge::*;

const PY_TP_DEALLOC: c_int = 52;
const PY_TP_REPR: c_int = 66;
const PY_TP_METHODS: c_int = 64;
const PY_TP_NEW: c_int = 65;
const PY_SQ_LENGTH: c_int = 45;
const METH_CLASS: c_int = 0x0010;
const PY_LT: c_int = 0;
const PY_EQ: c_int = 2;
const PY_GT: c_int = 4;

extern "C" {
    fn PyType_GenericAlloc(type_obj: PyObjectPtr, nitems: isize) -> PyObjectPtr;
    fn PyObject_Free(ptr: *mut c_void);
    fn PyTuple_Size(tuple: PyObjectPtr) -> isize;
    fn PyObject_RichCompareBool(a: PyObjectPtr, b: PyObjectPtr, op: c_int) -> c_int;
    fn PyObject_Repr(obj: PyObjectPtr) -> PyObjectPtr;
    fn PyLong_AsLong(obj: PyObjectPtr) -> c_long;
}

struct PyComparable(PyObjectPtr);

impl PyComparable {
    unsafe fn from_borrowed(obj: PyObjectPtr) -> Self {
        Py_IncRef(obj);
        Self(obj)
    }

    unsafe fn to_object(&self) -> PyObjectPtr {
        Py_IncRef(self.0);
        self.0
    }

    unsafe fn compare(&self, other: &Self, op: c_int) -> bool {
        let result = PyObject_RichCompareBool(self.0, other.0, op);
        if result < 0 {
            PyErr_Clear();
            false
        } else {
            result == 1
        }
    }
}

impl Clone for PyComparable {
    fn clone(&self) -> Self {
        unsafe {
            Py_IncRef(self.0);
        }
        Self(self.0)
    }
}

impl Drop for PyComparable {
    fn drop(&mut self) {
        unsafe {
            Py_DecRef(self.0);
        }
    }
}

impl PartialEq for PyComparable {
    fn eq(&self, other: &Self) -> bool {
        unsafe { self.compare(other, PY_EQ) }
    }
}

impl Eq for PyComparable {}

impl PartialOrd for PyComparable {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PyComparable {
    fn cmp(&self, other: &Self) -> Ordering {
        unsafe {
            if self.compare(other, PY_LT) {
                Ordering::Less
            } else if self.compare(other, PY_GT) {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        }
    }
}

#[repr(C)]
struct MinHeapObject {
    ob_refcnt: isize,
    ob_type: PyObjectPtr,
    heap: *mut CoreMinHeap<PyComparable>,
}

#[repr(C)]
struct MaxHeapObject {
    ob_refcnt: isize,
    ob_type: PyObjectPtr,
    heap: *mut CoreMaxHeap<PyComparable>,
}

unsafe fn get_min_heap(slf: PyObjectPtr) -> &'static CoreMinHeap<PyComparable> {
    &*((slf as *mut MinHeapObject).read().heap)
}

unsafe fn get_min_heap_mut(slf: PyObjectPtr) -> &'static mut CoreMinHeap<PyComparable> {
    &mut *(*(slf as *mut MinHeapObject)).heap
}

unsafe fn get_max_heap(slf: PyObjectPtr) -> &'static CoreMaxHeap<PyComparable> {
    &*((slf as *mut MaxHeapObject).read().heap)
}

unsafe fn get_max_heap_mut(slf: PyObjectPtr) -> &'static mut CoreMaxHeap<PyComparable> {
    &mut *(*(slf as *mut MaxHeapObject)).heap
}

unsafe fn iterable_to_values(obj: PyObjectPtr) -> Option<Vec<PyComparable>> {
    let iter = PyObject_GetIter(obj);
    if iter.is_null() {
        set_error(type_error_class(), "expected an iterable");
        return None;
    }

    let mut values = Vec::new();
    loop {
        let item = PyIter_Next(iter);
        if item.is_null() {
            break;
        }
        values.push(PyComparable::from_borrowed(item));
        Py_DecRef(item);
    }
    Py_DecRef(iter);
    Some(values)
}

unsafe fn values_to_list(values: &[PyComparable]) -> PyObjectPtr {
    let list = PyList_New(values.len() as isize);
    for (index, value) in values.iter().enumerate() {
        PyList_SetItem(list, index as isize, value.to_object());
    }
    list
}

unsafe fn parse_iterable_arg(args: PyObjectPtr, index: isize) -> Option<Vec<PyComparable>> {
    let obj = PyTuple_GetItem(args, index);
    if obj.is_null() {
        set_error(type_error_class(), "iterable argument missing");
        return None;
    }
    iterable_to_values(obj)
}

unsafe fn parse_n_arg(args: PyObjectPtr, index: isize) -> Option<usize> {
    let obj = PyTuple_GetItem(args, index);
    if obj.is_null() {
        set_error(type_error_class(), "n argument missing");
        return None;
    }
    let value = PyLong_AsLong(obj);
    if value < 0 {
        return Some(0);
    }
    Some(value as usize)
}

unsafe fn heap_root_repr(value: Option<&PyComparable>) -> String {
    match value {
        Some(value) => {
            let repr = PyObject_Repr(value.0);
            if repr.is_null() {
                PyErr_Clear();
                "<??>".to_string()
            } else {
                let rendered = str_from_py(repr).unwrap_or_else(|| "<??>".to_string());
                Py_DecRef(repr);
                rendered
            }
        }
        None => "empty".to_string(),
    }
}

unsafe fn wrap_min_heap(type_obj: PyObjectPtr, heap: CoreMinHeap<PyComparable>) -> PyObjectPtr {
    let obj = PyType_GenericAlloc(type_obj, 0);
    if obj.is_null() {
        return ptr::null_mut();
    }
    (*(obj as *mut MinHeapObject)).heap = Box::into_raw(Box::new(heap));
    obj
}

unsafe fn wrap_max_heap(type_obj: PyObjectPtr, heap: CoreMaxHeap<PyComparable>) -> PyObjectPtr {
    let obj = PyType_GenericAlloc(type_obj, 0);
    if obj.is_null() {
        return ptr::null_mut();
    }
    (*(obj as *mut MaxHeapObject)).heap = Box::into_raw(Box::new(heap));
    obj
}

unsafe extern "C" fn min_heap_new(
    type_obj: PyObjectPtr,
    _args: PyObjectPtr,
    _kwargs: PyObjectPtr,
) -> PyObjectPtr {
    wrap_min_heap(type_obj, CoreMinHeap::new())
}

unsafe extern "C" fn max_heap_new(
    type_obj: PyObjectPtr,
    _args: PyObjectPtr,
    _kwargs: PyObjectPtr,
) -> PyObjectPtr {
    wrap_max_heap(type_obj, CoreMaxHeap::new())
}

unsafe extern "C" fn min_heap_dealloc(obj: PyObjectPtr) {
    let heap_obj = obj as *mut MinHeapObject;
    if !(*heap_obj).heap.is_null() {
        let _ = Box::from_raw((*heap_obj).heap);
        (*heap_obj).heap = ptr::null_mut();
    }
    PyObject_Free(obj as *mut c_void);
}

unsafe extern "C" fn max_heap_dealloc(obj: PyObjectPtr) {
    let heap_obj = obj as *mut MaxHeapObject;
    if !(*heap_obj).heap.is_null() {
        let _ = Box::from_raw((*heap_obj).heap);
        (*heap_obj).heap = ptr::null_mut();
    }
    PyObject_Free(obj as *mut c_void);
}

unsafe extern "C" fn min_heap_sq_length(slf: PyObjectPtr) -> isize {
    get_min_heap(slf).len() as isize
}

unsafe extern "C" fn max_heap_sq_length(slf: PyObjectPtr) -> isize {
    get_max_heap(slf).len() as isize
}

unsafe extern "C" fn min_heap_from_iterable(cls: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let Some(values) = parse_iterable_arg(args, 0) else {
        return ptr::null_mut();
    };
    wrap_min_heap(cls, CoreMinHeap::from_iterable(values))
}

unsafe extern "C" fn max_heap_from_iterable(cls: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let Some(values) = parse_iterable_arg(args, 0) else {
        return ptr::null_mut();
    };
    wrap_max_heap(cls, CoreMaxHeap::from_iterable(values))
}

unsafe extern "C" fn min_heap_push(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let value = PyTuple_GetItem(args, 0);
    if value.is_null() {
        set_error(type_error_class(), "push() requires a value");
        return ptr::null_mut();
    }
    get_min_heap_mut(slf).push(PyComparable::from_borrowed(value));
    py_none()
}

unsafe extern "C" fn max_heap_push(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let value = PyTuple_GetItem(args, 0);
    if value.is_null() {
        set_error(type_error_class(), "push() requires a value");
        return ptr::null_mut();
    }
    get_max_heap_mut(slf).push(PyComparable::from_borrowed(value));
    py_none()
}

unsafe extern "C" fn min_heap_pop(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    match get_min_heap_mut(slf).pop() {
        Some(value) => value.to_object(),
        None => {
            set_error(index_error_class(), "pop from an empty heap");
            ptr::null_mut()
        }
    }
}

unsafe extern "C" fn max_heap_pop(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    match get_max_heap_mut(slf).pop() {
        Some(value) => value.to_object(),
        None => {
            set_error(index_error_class(), "pop from an empty heap");
            ptr::null_mut()
        }
    }
}

unsafe extern "C" fn min_heap_peek(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    match get_min_heap(slf).peek() {
        Some(value) => value.to_object(),
        None => {
            set_error(index_error_class(), "peek at an empty heap");
            ptr::null_mut()
        }
    }
}

unsafe extern "C" fn max_heap_peek(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    match get_max_heap(slf).peek() {
        Some(value) => value.to_object(),
        None => {
            set_error(index_error_class(), "peek at an empty heap");
            ptr::null_mut()
        }
    }
}

unsafe extern "C" fn min_heap_is_empty(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    bool_to_py(get_min_heap(slf).is_empty())
}

unsafe extern "C" fn max_heap_is_empty(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    bool_to_py(get_max_heap(slf).is_empty())
}

unsafe extern "C" fn min_heap_to_array(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    let values = get_min_heap(slf).to_vec();
    values_to_list(&values)
}

unsafe extern "C" fn max_heap_to_array(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    let values = get_max_heap(slf).to_vec();
    values_to_list(&values)
}

unsafe extern "C" fn min_heap_repr(slf: PyObjectPtr) -> PyObjectPtr {
    let heap = get_min_heap(slf);
    str_to_py(&format!(
        "MinHeap(size={}, root={})",
        heap.len(),
        heap_root_repr(heap.peek())
    ))
}

unsafe extern "C" fn max_heap_repr(slf: PyObjectPtr) -> PyObjectPtr {
    let heap = get_max_heap(slf);
    str_to_py(&format!(
        "MaxHeap(size={}, root={})",
        heap.len(),
        heap_root_repr(heap.peek())
    ))
}

unsafe extern "C" fn py_heapify(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let Some(values) = parse_iterable_arg(args, 0) else {
        return ptr::null_mut();
    };
    let result = core_heapify(values);
    values_to_list(&result)
}

unsafe extern "C" fn py_heap_sort(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let Some(values) = parse_iterable_arg(args, 0) else {
        return ptr::null_mut();
    };
    let result = core_heap_sort(values);
    values_to_list(&result)
}

unsafe extern "C" fn py_nlargest(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let Some(values) = parse_iterable_arg(args, 0) else {
        return ptr::null_mut();
    };
    let Some(n) = parse_n_arg(args, 1) else {
        return ptr::null_mut();
    };
    let result = core_nlargest(values, n);
    values_to_list(&result)
}

unsafe extern "C" fn py_nsmallest(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let Some(values) = parse_iterable_arg(args, 0) else {
        return ptr::null_mut();
    };
    let Some(n) = parse_n_arg(args, 1) else {
        return ptr::null_mut();
    };
    let result = core_nsmallest(values, n);
    values_to_list(&result)
}

fn cstr(value: &str) -> *const c_char {
    CString::new(value).expect("no NUL").into_raw()
}

#[no_mangle]
pub unsafe extern "C" fn PyInit_heap_native() -> PyObjectPtr {
    static mut MODULE_METHODS: [PyMethodDef; 5] = [method_def_sentinel(); 5];
    MODULE_METHODS[0] = PyMethodDef {
        ml_name: cstr("heapify"),
        ml_meth: Some(py_heapify),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    MODULE_METHODS[1] = PyMethodDef {
        ml_name: cstr("heap_sort"),
        ml_meth: Some(py_heap_sort),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    MODULE_METHODS[2] = PyMethodDef {
        ml_name: cstr("nlargest"),
        ml_meth: Some(py_nlargest),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    MODULE_METHODS[3] = PyMethodDef {
        ml_name: cstr("nsmallest"),
        ml_meth: Some(py_nsmallest),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    MODULE_METHODS[4] = method_def_sentinel();

    static mut MIN_HEAP_METHODS: [PyMethodDef; 7] = [method_def_sentinel(); 7];
    MIN_HEAP_METHODS[0] = PyMethodDef {
        ml_name: cstr("from_iterable"),
        ml_meth: Some(min_heap_from_iterable),
        ml_flags: METH_VARARGS | METH_CLASS,
        ml_doc: ptr::null(),
    };
    MIN_HEAP_METHODS[1] = PyMethodDef {
        ml_name: cstr("push"),
        ml_meth: Some(min_heap_push),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    MIN_HEAP_METHODS[2] = PyMethodDef {
        ml_name: cstr("pop"),
        ml_meth: Some(min_heap_pop),
        ml_flags: METH_NOARGS,
        ml_doc: ptr::null(),
    };
    MIN_HEAP_METHODS[3] = PyMethodDef {
        ml_name: cstr("peek"),
        ml_meth: Some(min_heap_peek),
        ml_flags: METH_NOARGS,
        ml_doc: ptr::null(),
    };
    MIN_HEAP_METHODS[4] = PyMethodDef {
        ml_name: cstr("is_empty"),
        ml_meth: Some(min_heap_is_empty),
        ml_flags: METH_NOARGS,
        ml_doc: ptr::null(),
    };
    MIN_HEAP_METHODS[5] = PyMethodDef {
        ml_name: cstr("to_array"),
        ml_meth: Some(min_heap_to_array),
        ml_flags: METH_NOARGS,
        ml_doc: ptr::null(),
    };
    MIN_HEAP_METHODS[6] = method_def_sentinel();

    static mut MAX_HEAP_METHODS: [PyMethodDef; 7] = [method_def_sentinel(); 7];
    MAX_HEAP_METHODS[0] = PyMethodDef {
        ml_name: cstr("from_iterable"),
        ml_meth: Some(max_heap_from_iterable),
        ml_flags: METH_VARARGS | METH_CLASS,
        ml_doc: ptr::null(),
    };
    MAX_HEAP_METHODS[1] = PyMethodDef {
        ml_name: cstr("push"),
        ml_meth: Some(max_heap_push),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    MAX_HEAP_METHODS[2] = PyMethodDef {
        ml_name: cstr("pop"),
        ml_meth: Some(max_heap_pop),
        ml_flags: METH_NOARGS,
        ml_doc: ptr::null(),
    };
    MAX_HEAP_METHODS[3] = PyMethodDef {
        ml_name: cstr("peek"),
        ml_meth: Some(max_heap_peek),
        ml_flags: METH_NOARGS,
        ml_doc: ptr::null(),
    };
    MAX_HEAP_METHODS[4] = PyMethodDef {
        ml_name: cstr("is_empty"),
        ml_meth: Some(max_heap_is_empty),
        ml_flags: METH_NOARGS,
        ml_doc: ptr::null(),
    };
    MAX_HEAP_METHODS[5] = PyMethodDef {
        ml_name: cstr("to_array"),
        ml_meth: Some(max_heap_to_array),
        ml_flags: METH_NOARGS,
        ml_doc: ptr::null(),
    };
    MAX_HEAP_METHODS[6] = method_def_sentinel();

    static mut MIN_HEAP_SLOTS: [PyType_Slot; 6] = [type_slot_sentinel(); 6];
    MIN_HEAP_SLOTS[0] = PyType_Slot {
        slot: PY_TP_NEW,
        pfunc: min_heap_new as *mut c_void,
    };
    MIN_HEAP_SLOTS[1] = PyType_Slot {
        slot: PY_TP_DEALLOC,
        pfunc: min_heap_dealloc as *mut c_void,
    };
    MIN_HEAP_SLOTS[2] = PyType_Slot {
        slot: PY_TP_METHODS,
        pfunc: (&raw mut MIN_HEAP_METHODS) as *mut c_void,
    };
    MIN_HEAP_SLOTS[3] = PyType_Slot {
        slot: PY_TP_REPR,
        pfunc: min_heap_repr as *mut c_void,
    };
    MIN_HEAP_SLOTS[4] = PyType_Slot {
        slot: PY_SQ_LENGTH,
        pfunc: min_heap_sq_length as *mut c_void,
    };
    MIN_HEAP_SLOTS[5] = type_slot_sentinel();

    static mut MAX_HEAP_SLOTS: [PyType_Slot; 6] = [type_slot_sentinel(); 6];
    MAX_HEAP_SLOTS[0] = PyType_Slot {
        slot: PY_TP_NEW,
        pfunc: max_heap_new as *mut c_void,
    };
    MAX_HEAP_SLOTS[1] = PyType_Slot {
        slot: PY_TP_DEALLOC,
        pfunc: max_heap_dealloc as *mut c_void,
    };
    MAX_HEAP_SLOTS[2] = PyType_Slot {
        slot: PY_TP_METHODS,
        pfunc: (&raw mut MAX_HEAP_METHODS) as *mut c_void,
    };
    MAX_HEAP_SLOTS[3] = PyType_Slot {
        slot: PY_TP_REPR,
        pfunc: max_heap_repr as *mut c_void,
    };
    MAX_HEAP_SLOTS[4] = PyType_Slot {
        slot: PY_SQ_LENGTH,
        pfunc: max_heap_sq_length as *mut c_void,
    };
    MAX_HEAP_SLOTS[5] = type_slot_sentinel();

    static mut MIN_HEAP_SPEC: PyType_Spec = PyType_Spec {
        name: ptr::null(),
        basicsize: 0,
        itemsize: 0,
        flags: 0,
        slots: ptr::null_mut(),
    };
    MIN_HEAP_SPEC.name = cstr("heap_native.MinHeap");
    MIN_HEAP_SPEC.basicsize = std::mem::size_of::<MinHeapObject>() as c_int;
    MIN_HEAP_SPEC.flags = PY_TPFLAGS_DEFAULT;
    MIN_HEAP_SPEC.slots = (&raw mut MIN_HEAP_SLOTS) as *mut PyType_Slot;

    static mut MAX_HEAP_SPEC: PyType_Spec = PyType_Spec {
        name: ptr::null(),
        basicsize: 0,
        itemsize: 0,
        flags: 0,
        slots: ptr::null_mut(),
    };
    MAX_HEAP_SPEC.name = cstr("heap_native.MaxHeap");
    MAX_HEAP_SPEC.basicsize = std::mem::size_of::<MaxHeapObject>() as c_int;
    MAX_HEAP_SPEC.flags = PY_TPFLAGS_DEFAULT;
    MAX_HEAP_SPEC.slots = (&raw mut MAX_HEAP_SLOTS) as *mut PyType_Slot;

    let min_heap_type = PyType_FromSpec(&raw mut MIN_HEAP_SPEC);
    if min_heap_type.is_null() {
        return ptr::null_mut();
    }
    let max_heap_type = PyType_FromSpec(&raw mut MAX_HEAP_SPEC);
    if max_heap_type.is_null() {
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
    MODULE_DEF.m_name = cstr("heap_native");
    MODULE_DEF.m_methods = (&raw mut MODULE_METHODS) as *mut PyMethodDef;

    let module = PyModule_Create2(&raw mut MODULE_DEF, PYTHON_API_VERSION);
    if module.is_null() {
        return ptr::null_mut();
    }

    Py_IncRef(min_heap_type);
    module_add_object(module, "MinHeap", min_heap_type);
    Py_IncRef(max_heap_type);
    module_add_object(module, "MaxHeap", max_heap_type);

    module
}
