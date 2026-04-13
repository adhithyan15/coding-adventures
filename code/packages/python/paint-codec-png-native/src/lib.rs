use std::ffi::{c_char, c_int, CString};
use std::ptr;
use std::slice;
use std::sync::OnceLock;

use paint_instructions::PixelContainer;
use python_bridge::*;

#[allow(non_snake_case)]
extern "C" {
    fn PyBytes_FromStringAndSize(s: *const c_char, size: isize) -> PyObjectPtr;
    fn PyBytes_AsStringAndSize(
        obj: PyObjectPtr,
        buffer: *mut *mut c_char,
        size: *mut isize,
    ) -> c_int;
}

fn cstr(value: &str) -> *const c_char {
    CString::new(value).expect("no NUL bytes").into_raw()
}

unsafe fn bytes_to_py(bytes: &[u8]) -> PyObjectPtr {
    PyBytes_FromStringAndSize(bytes.as_ptr() as *const c_char, bytes.len() as isize)
}

unsafe fn bytes_from_py(obj: PyObjectPtr) -> Option<Vec<u8>> {
    let mut buffer: *mut c_char = ptr::null_mut();
    let mut size: isize = 0;
    if PyBytes_AsStringAndSize(obj, &mut buffer, &mut size) != 0 || buffer.is_null() {
        PyErr_Clear();
        return None;
    }

    Some(slice::from_raw_parts(buffer as *const u8, size as usize).to_vec())
}

unsafe extern "C" fn encode_rgba8_native(
    _module: PyObjectPtr,
    args: PyObjectPtr,
) -> PyObjectPtr {
    let Some(width) = parse_arg_f64(args, 0) else {
        set_error(type_error_class(), "encode_rgba8_native() requires width");
        return ptr::null_mut();
    };
    let Some(height) = parse_arg_f64(args, 1) else {
        set_error(type_error_class(), "encode_rgba8_native() requires height");
        return ptr::null_mut();
    };

    let data_obj = PyTuple_GetItem(args, 2);
    if data_obj.is_null() {
        PyErr_Clear();
        set_error(type_error_class(), "encode_rgba8_native() requires RGBA bytes");
        return ptr::null_mut();
    }

    let Some(bytes) = bytes_from_py(data_obj) else {
        set_error(type_error_class(), "encode_rgba8_native() expected bytes");
        return ptr::null_mut();
    };

    let width = width as u32;
    let height = height as u32;
    let expected_len = width as usize * height as usize * 4;
    if bytes.len() != expected_len {
        set_error(value_error_class(), "RGBA buffer length does not match width * height * 4");
        return ptr::null_mut();
    }

    let pixels = PixelContainer::from_data(width, height, bytes);
    bytes_to_py(&paint_codec_png::encode_png(&pixels))
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
                    ml_name: cstr("encode_rgba8_native"),
                    ml_meth: Some(encode_rgba8_native),
                    ml_flags: METH_VARARGS,
                    ml_doc: cstr(
                        "encode_rgba8_native(width, height, rgba_bytes) -> bytes\n\n\
                         Encode RGBA8 pixels as a PNG image.",
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
                m_name: cstr("paint_codec_png_native"),
                m_doc: cstr("paint_codec_png_native -- Rust-backed PNG codec for Python."),
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
pub unsafe extern "C" fn PyInit_paint_codec_png_native() -> PyObjectPtr {
    PyModule_Create2(
        get_module_def() as *const PyModuleDef as *mut PyModuleDef,
        PYTHON_API_VERSION,
    )
}
