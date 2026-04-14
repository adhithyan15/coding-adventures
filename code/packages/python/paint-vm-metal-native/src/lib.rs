use std::ffi::{c_char, CString};
use std::ptr;
use std::sync::OnceLock;

use paint_instructions::{PaintInstruction, PaintRect, PaintScene};
use python_bridge::*;

#[allow(non_snake_case)]
extern "C" {
    fn PyBytes_FromStringAndSize(s: *const c_char, size: isize) -> PyObjectPtr;
}

fn cstr(value: &str) -> *const c_char {
    CString::new(value).expect("no NUL bytes").into_raw()
}

unsafe fn bytes_to_py(bytes: &[u8]) -> PyObjectPtr {
    PyBytes_FromStringAndSize(bytes.as_ptr() as *const c_char, bytes.len() as isize)
}

unsafe fn parse_rect(item: PyObjectPtr) -> Option<PaintInstruction> {
    let x = f64_from_py(PyTuple_GetItem(item, 0))?;
    let y = f64_from_py(PyTuple_GetItem(item, 1))?;
    let width = f64_from_py(PyTuple_GetItem(item, 2))?;
    let height = f64_from_py(PyTuple_GetItem(item, 3))?;
    let fill = str_from_py(PyTuple_GetItem(item, 4))?;

    if width < 0.0 || height < 0.0 {
        return None;
    }

    Some(PaintInstruction::Rect(PaintRect::filled(
        x, y, width, height, &fill,
    )))
}

unsafe fn parse_rects(obj: PyObjectPtr) -> Option<Vec<PaintInstruction>> {
    let len = PyList_Size(obj);
    if len < 0 {
        PyErr_Clear();
        return None;
    }

    let mut rects = Vec::with_capacity(len as usize);
    for index in 0..len {
        let item = PyList_GetItem(obj, index);
        if item.is_null() {
            PyErr_Clear();
            return None;
        }
        rects.push(parse_rect(item)?);
    }

    Some(rects)
}

unsafe extern "C" fn render_rect_scene_native(
    _module: PyObjectPtr,
    args: PyObjectPtr,
) -> PyObjectPtr {
    let Some(width) = parse_arg_f64(args, 0) else {
        set_error(type_error_class(), "render_rect_scene_native() requires width");
        return ptr::null_mut();
    };
    let Some(height) = parse_arg_f64(args, 1) else {
        set_error(type_error_class(), "render_rect_scene_native() requires height");
        return ptr::null_mut();
    };
    let Some(background) = parse_arg_str(args, 2) else {
        set_error(
            type_error_class(),
            "render_rect_scene_native() requires a background string",
        );
        return ptr::null_mut();
    };

    let rects_obj = PyTuple_GetItem(args, 3);
    if rects_obj.is_null() {
        PyErr_Clear();
        set_error(
            type_error_class(),
            "render_rect_scene_native() requires a rectangle list",
        );
        return ptr::null_mut();
    }

    let Some(rects) = parse_rects(rects_obj) else {
        set_error(
            value_error_class(),
            "render_rect_scene_native() expected a list of (x, y, width, height, fill) tuples",
        );
        return ptr::null_mut();
    };

    let mut scene = PaintScene::new(width, height);
    scene.background = background;
    scene.instructions = rects;

    let pixels = match std::panic::catch_unwind(|| paint_metal::render(&scene)) {
        Ok(value) => value,
        Err(_) => {
            set_error(runtime_error_class(), "Metal rendering failed");
            return ptr::null_mut();
        }
    };

    let payload = PyTuple_New(3);
    PyTuple_SetItem(payload, 0, usize_to_py(pixels.width as usize));
    PyTuple_SetItem(payload, 1, usize_to_py(pixels.height as usize));
    PyTuple_SetItem(payload, 2, bytes_to_py(&pixels.data));
    payload
}

struct SendSync<T>(T);
unsafe impl<T> Send for SendSync<T> {}
unsafe impl<T> Sync for SendSync<T> {}

fn get_methods() -> &'static [PyMethodDef] {
    static METHODS: OnceLock<SendSync<Vec<PyMethodDef>>> = OnceLock::new();
    &METHODS
        .get_or_init(|| {
            SendSync(vec![
                PyMethodDef {
                    ml_name: cstr("render_rect_scene_native"),
                    ml_meth: Some(render_rect_scene_native),
                    ml_flags: METH_VARARGS,
                    ml_doc: cstr(
                        "render_rect_scene_native(width, height, background, rects) -> (width, height, bytes)\n\n\
                         Execute a rect-only PaintScene through Metal and return RGBA8 bytes.",
                    ),
                },
                PyMethodDef {
                    ml_name: ptr::null(),
                    ml_meth: None,
                    ml_flags: 0,
                    ml_doc: ptr::null(),
                },
            ])
        })
        .0
}

fn get_module_def() -> &'static PyModuleDef {
    static MODULE_DEF: OnceLock<SendSync<PyModuleDef>> = OnceLock::new();
    &MODULE_DEF
        .get_or_init(|| {
            SendSync(PyModuleDef {
                m_base: PyModuleDef_Base {
                    ob_base: [0; std::mem::size_of::<usize>() * 2],
                    m_init: None,
                    m_index: 0,
                    m_copy: ptr::null_mut(),
                },
                m_name: cstr("paint_vm_metal_native"),
                m_doc: cstr(
                    "paint_vm_metal_native -- Rust-backed Metal Paint VM bridge for Python.",
                ),
                m_size: -1,
                m_methods: get_methods().as_ptr() as *mut PyMethodDef,
                m_slots: ptr::null_mut(),
                m_traverse: ptr::null_mut(),
                m_clear: ptr::null_mut(),
                m_free: ptr::null_mut(),
            })
        })
        .0
}

#[no_mangle]
pub unsafe extern "C" fn PyInit_paint_vm_metal_native() -> PyObjectPtr {
    PyModule_Create2(
        get_module_def() as *const PyModuleDef as *mut PyModuleDef,
        PYTHON_API_VERSION,
    )
}
