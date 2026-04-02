//! # font-parser-python
//!
//! Python C extension that wraps the Rust `font-parser` core library.
//! Exposes five functions to Python:
//!
//! ```python
//! import font_parser_native as fp
//!
//! data = open("Inter-Regular.ttf", "rb").read()
//! font = fp.load(data)
//!
//! m = fp.font_metrics(font)
//! # m["units_per_em"]  → 2048
//! # m["family_name"]   → "Inter"
//!
//! gid = fp.glyph_id(font, 0x0041)   # 'A'
//! gm  = fp.glyph_metrics(font, gid)
//! # gm["advance_width"]       → 1401
//! # gm["left_side_bearing"]   → 7
//!
//! k = fp.kerning(font, gid, fp.glyph_id(font, 0x0056))
//! # k → 0 (Inter v4 uses GPOS)
//! ```
//!
//! ## How font handles work
//!
//! `fp.load()` returns a Python `PyCapsule` — an opaque object that holds a
//! raw pointer to a heap-allocated `font_parser::FontFile`. The capsule has
//! a destructor (`free_font_file`) that drops the `Box<FontFile>` when
//! Python garbage-collects it.
//!
//! Every other function receives this capsule as its first argument and
//! extracts the pointer with `PyCapsule_GetPointer`.
//!
//! ## Integer and dict returns
//!
//! - Optional integers (glyph_id) return Python `None` when absent.
//! - Struct returns (font_metrics, glyph_metrics) become Python `dict`s.
//! - Error conditions raise `ValueError`.

#![allow(non_snake_case)] // C API names are PascalCase / SCREAMING_SNAKE_CASE

use std::ffi::{c_char, c_int, c_long, c_void, CString};
use std::ptr;

use python_bridge::{
    PyMethodDef, PyModuleDef, PyModuleDef_Base, PyObjectPtr,
    METH_VARARGS, PYTHON_API_VERSION,
    Py_DecRef, PyModule_Create2,
    PyLong_FromLong,
    PyErr_SetString, PyErr_Clear,
    PyTuple_GetItem,
    py_none, str_to_py,
    value_error_class, type_error_class,
};
use font_parser::{self as fp, FontError};

// ─────────────────────────────────────────────────────────────────────────────
// Additional Python C API functions not already in python-bridge
// ─────────────────────────────────────────────────────────────────────────────
//
// We declare them inline here rather than modifying the bridge crate.
// These are all part of Python's stable Limited API (PEP 384).

#[allow(non_snake_case)]
extern "C" {
    // PyCapsule: opaque wrapper for a C pointer + destructor.
    // Python docs: https://docs.python.org/3/c-api/capsule.html
    fn PyCapsule_New(
        pointer: *mut c_void,
        name: *const c_char,
        destructor: Option<unsafe extern "C" fn(PyObjectPtr)>,
    ) -> PyObjectPtr;
    fn PyCapsule_GetPointer(capsule: PyObjectPtr, name: *const c_char) -> *mut c_void;

    // PyBytes: `bytes` type — used to receive raw font data from Python.
    // PyBytes_AsStringAndSize fills `buf` with a pointer into the bytes
    // object's internal buffer and sets `size` to its length.
    // Returns 0 on success, -1 on type error.
    fn PyBytes_AsStringAndSize(
        o: PyObjectPtr,
        buf: *mut *const u8,
        size: *mut isize,
    ) -> c_int;

    // PyDict: mutable mapping — we use it to return metric structs.
    fn PyDict_New() -> PyObjectPtr;
    fn PyDict_SetItemString(d: PyObjectPtr, key: *const c_char, val: PyObjectPtr) -> c_int;

    // PyLong_AsLong: extract a C long from a Python int.
    // Returns -1 and sets OverflowError on out-of-range values.
    // IMPORTANT: -1 is a valid return value for negative ints, so always
    // check PyErr_Occurred() after a -1 return.
    fn PyLong_AsLong(o: PyObjectPtr) -> c_long;

    // PyErr_Occurred: returns non-null if any Python exception is set.
    // Used to distinguish -1-as-error from -1-as-valid-integer after
    // PyLong_AsLong returns -1.
    fn PyErr_Occurred() -> PyObjectPtr;
}

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

/// Capsule name — Python checks this on `PyCapsule_GetPointer` to prevent
/// type confusion between capsules from different modules.
const CAPSULE_NAME: &[u8] = b"font_parser_native.FontFile\0";

// ─────────────────────────────────────────────────────────────────────────────
// Capsule destructor — called by Python's GC
// ─────────────────────────────────────────────────────────────────────────────

/// Drop the `Box<FontFile>` stored in a capsule when Python GC collects it.
///
/// Python calls this automatically; it is NOT safe to call directly.
///
/// The function signature is fixed by the C API:
///   `void destructor(PyObject *capsule)`
///
/// We extract the stored pointer and reconstruct the `Box` so that Rust's
/// drop machinery runs the `FontFile` destructor and frees the memory.
unsafe extern "C" fn free_font_file(capsule: PyObjectPtr) {
    let raw = PyCapsule_GetPointer(capsule, CAPSULE_NAME.as_ptr() as *const c_char);
    if !raw.is_null() {
        // Reconstruct the Box and immediately drop it → releases the FontFile
        let _ = Box::from_raw(raw as *mut fp::FontFile);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper: build a Python dict from FontMetrics
// ─────────────────────────────────────────────────────────────────────────────

/// Convert `FontMetrics` to a Python dict.
///
/// Keys match the Rust field names (snake_case). Optional fields (`x_height`,
/// `cap_height`) are `None` when the OS/2 version < 2.
unsafe fn metrics_to_dict(m: &fp::FontMetrics) -> PyObjectPtr {
    let d = PyDict_New();

    macro_rules! set_i64 {
        ($key:expr, $val:expr) => {
            let k = CString::new($key).unwrap();
            let v = PyLong_FromLong($val as c_long);
            PyDict_SetItemString(d, k.as_ptr(), v);
            Py_DecRef(v);
        };
    }
    macro_rules! set_str {
        ($key:expr, $val:expr) => {
            let k = CString::new($key).unwrap();
            let v = str_to_py($val);
            PyDict_SetItemString(d, k.as_ptr(), v);
            Py_DecRef(v);
        };
    }
    macro_rules! set_opt_i16 {
        ($key:expr, $val:expr) => {
            let k = CString::new($key).unwrap();
            let v = match $val {
                Some(n) => PyLong_FromLong(n as c_long),
                None => py_none(),
            };
            PyDict_SetItemString(d, k.as_ptr(), v);
            Py_DecRef(v);
        };
    }

    set_i64!("units_per_em",  m.units_per_em);
    set_i64!("ascender",      m.ascender);
    set_i64!("descender",     m.descender);
    set_i64!("line_gap",      m.line_gap);
    set_opt_i16!("x_height",  m.x_height);
    set_opt_i16!("cap_height", m.cap_height);
    set_i64!("num_glyphs",    m.num_glyphs);
    set_str!("family_name",   &m.family_name);
    set_str!("subfamily_name",&m.subfamily_name);

    d
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper: build a Python dict from GlyphMetrics
// ─────────────────────────────────────────────────────────────────────────────

unsafe fn glyph_metrics_to_dict(gm: &fp::GlyphMetrics) -> PyObjectPtr {
    let d = PyDict_New();

    let k1 = CString::new("advance_width").unwrap();
    let v1 = PyLong_FromLong(gm.advance_width as c_long);
    PyDict_SetItemString(d, k1.as_ptr(), v1);
    Py_DecRef(v1);

    let k2 = CString::new("left_side_bearing").unwrap();
    let v2 = PyLong_FromLong(gm.left_side_bearing as c_long);
    PyDict_SetItemString(d, k2.as_ptr(), v2);
    Py_DecRef(v2);

    d
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper: map FontError → Python ValueError
// ─────────────────────────────────────────────────────────────────────────────

/// Raise a Python `ValueError` describing the `FontError` and return null.
///
/// Returning `ptr::null_mut()` from a C extension function signals to Python
/// that an exception has been set and propagation should begin.
unsafe fn set_font_error(err: FontError) -> PyObjectPtr {
    let msg = match err {
        FontError::InvalidMagic          => "invalid magic: not a TrueType/OpenType font".to_string(),
        FontError::InvalidHeadMagic      => "invalid head magic number".to_string(),
        FontError::TableNotFound(t)      => format!("required table not found: {}", t),
        FontError::BufferTooShort        => "buffer too short".to_string(),
        FontError::UnsupportedCmapFormat => "unsupported cmap format".to_string(),
    };
    let exc = value_error_class();
    set_error(exc, &msg);
    ptr::null_mut()
}

/// Thin wrapper around `PyErr_SetString`.
unsafe fn set_error(exc: PyObjectPtr, msg: &str) {
    let c_msg = CString::new(msg).unwrap_or_else(|_| CString::new("(error)").unwrap());
    PyErr_SetString(exc, c_msg.as_ptr());
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper: extract the FontFile pointer from a capsule argument
// ─────────────────────────────────────────────────────────────────────────────

/// Get the `&FontFile` from a capsule in the args tuple at `index`.
///
/// Returns `None` (with an active Python exception) if the argument is not a
/// valid capsule or if the pointer is null.
unsafe fn font_from_args(args: PyObjectPtr, index: isize) -> Option<&'static fp::FontFile> {
    let capsule = PyTuple_GetItem(args, index);
    if capsule.is_null() {
        PyErr_Clear();
        let exc = type_error_class();
        set_error(exc, "argument must be a FontFile capsule");
        return None;
    }
    let raw = PyCapsule_GetPointer(capsule, CAPSULE_NAME.as_ptr() as *const c_char);
    if raw.is_null() {
        // PyCapsule_GetPointer already set an exception (capsule type mismatch)
        return None;
    }
    Some(&*(raw as *const fp::FontFile))
}

// ─────────────────────────────────────────────────────────────────────────────
// Exported functions: load, font_metrics, glyph_id, glyph_metrics, kerning
// ─────────────────────────────────────────────────────────────────────────────

/// `load(data: bytes) -> capsule`
///
/// Parse a font from a `bytes` object. Returns an opaque capsule handle.
/// Raises `ValueError` on parse failure, `TypeError` if `data` is not bytes.
///
/// # C signature
///
/// Python calls every `METH_VARARGS` function as:
///   `PyObject* fn(PyObject* _self, PyObject* args)`
/// where `args` is a tuple of the positional arguments.
unsafe extern "C" fn py_load(_self: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    // Extract args[0] — the bytes object
    let bytes_obj = PyTuple_GetItem(args, 0);
    if bytes_obj.is_null() {
        PyErr_Clear();
        let exc = type_error_class();
        set_error(exc, "load() requires one argument: bytes");
        return ptr::null_mut();
    }

    // Get a pointer into the bytes buffer and its length.
    // PyBytes_AsStringAndSize does NOT copy — it returns a pointer into the
    // Python bytes object's internal buffer. We must finish using the data
    // before the bytes object is garbage-collected.
    let mut buf: *const u8 = ptr::null();
    let mut size: isize = 0;
    let rc = PyBytes_AsStringAndSize(bytes_obj, &mut buf, &mut size);
    if rc != 0 {
        // PyBytes_AsStringAndSize set a TypeError for us
        return ptr::null_mut();
    }

    // SECURITY: PyBytes_AsStringAndSize writes a signed isize. A negative
    // return would wrap to a huge usize in from_raw_parts — validate first.
    if size < 0 {
        let exc = type_error_class();
        set_error(exc, "load(): internal error — negative buffer size returned");
        return ptr::null_mut();
    }

    // Build a slice over the buffer — this is only valid while `bytes_obj`
    // is alive.  `fp::load` parses synchronously and copies what it needs,
    // so the slice does not outlive this function.
    let slice = std::slice::from_raw_parts(buf, size as usize);

    // Parse the font
    let font_file = match fp::load(slice) {
        Ok(f) => f,
        Err(e) => return set_font_error(e),
    };

    // Heap-allocate the FontFile and convert it to a raw pointer.
    // The capsule destructor (`free_font_file`) will `Box::from_raw` this
    // back to drop it when Python GC collects the capsule.
    let boxed = Box::into_raw(Box::new(font_file));

    PyCapsule_New(
        boxed as *mut c_void,
        CAPSULE_NAME.as_ptr() as *const c_char,
        Some(free_font_file),
    )
}

/// `font_metrics(font) -> dict`
///
/// Returns a dict with keys: units_per_em, ascender, descender, line_gap,
/// x_height (int | None), cap_height (int | None), num_glyphs,
/// family_name (str), subfamily_name (str).
unsafe extern "C" fn py_font_metrics(_self: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let font = match font_from_args(args, 0) {
        Some(f) => f,
        None => return ptr::null_mut(),
    };
    let m = fp::font_metrics(font);
    metrics_to_dict(&m)
}

/// `glyph_id(font, codepoint: int) -> int | None`
///
/// Map a Unicode codepoint (BMP only, 0x0000–0xFFFF) to a glyph ID.
/// Returns Python `None` if the codepoint is not in the font.
unsafe extern "C" fn py_glyph_id(_self: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let font = match font_from_args(args, 0) {
        Some(f) => f,
        None => return ptr::null_mut(),
    };

    let cp_obj = PyTuple_GetItem(args, 1);
    if cp_obj.is_null() {
        PyErr_Clear();
        let exc = type_error_class();
        set_error(exc, "glyph_id() requires two arguments: font, codepoint");
        return ptr::null_mut();
    }
    // SECURITY: PyLong_AsLong returns -1 on OverflowError/TypeError, but -1
    // is also a valid result for negative integers. Check PyErr_Occurred()
    // to distinguish the two cases.
    let cp_raw = PyLong_AsLong(cp_obj);
    if cp_raw == -1 && !PyErr_Occurred().is_null() {
        // Exception already set by PyLong_AsLong — propagate it
        return ptr::null_mut();
    }
    let cp = cp_raw as u32;

    match fp::glyph_id(font, cp) {
        Some(gid) => PyLong_FromLong(gid as c_long),
        None => py_none(),
    }
}

/// `glyph_metrics(font, glyph_id: int) -> dict | None`
///
/// Returns a dict with keys: advance_width (int), left_side_bearing (int).
/// Returns Python `None` if the glyph ID is out of range.
unsafe extern "C" fn py_glyph_metrics(_self: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let font = match font_from_args(args, 0) {
        Some(f) => f,
        None => return ptr::null_mut(),
    };

    let gid_obj = PyTuple_GetItem(args, 1);
    if gid_obj.is_null() {
        PyErr_Clear();
        let exc = type_error_class();
        set_error(exc, "glyph_metrics() requires two arguments: font, glyph_id");
        return ptr::null_mut();
    }
    // SECURITY: check PyLong_AsLong sentinel; also validate range for u16.
    let gid_raw = PyLong_AsLong(gid_obj);
    if gid_raw == -1 && !PyErr_Occurred().is_null() {
        return ptr::null_mut();
    }
    if gid_raw < 0 || gid_raw > u16::MAX as c_long {
        let exc = value_error_class();
        set_error(exc, "glyph_metrics(): glyph_id must be in range 0..65535");
        return ptr::null_mut();
    }
    let gid = gid_raw as u16;

    match fp::glyph_metrics(font, gid) {
        Some(gm) => glyph_metrics_to_dict(&gm),
        None => py_none(),
    }
}

/// `kerning(font, left: int, right: int) -> int`
///
/// Returns the kern value for the pair (left_glyph_id, right_glyph_id).
/// Returns 0 when no pair is found (or when the font uses GPOS).
unsafe extern "C" fn py_kerning(_self: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let font = match font_from_args(args, 0) {
        Some(f) => f,
        None => return ptr::null_mut(),
    };

    let left_obj = PyTuple_GetItem(args, 1);
    let right_obj = PyTuple_GetItem(args, 2);
    if left_obj.is_null() || right_obj.is_null() {
        PyErr_Clear();
        let exc = type_error_class();
        set_error(exc, "kerning() requires three arguments: font, left, right");
        return ptr::null_mut();
    }
    // SECURITY: check PyLong_AsLong sentinel and validate range for u16.
    let left_raw = PyLong_AsLong(left_obj);
    if left_raw == -1 && !PyErr_Occurred().is_null() {
        return ptr::null_mut();
    }
    let right_raw = PyLong_AsLong(right_obj);
    if right_raw == -1 && !PyErr_Occurred().is_null() {
        return ptr::null_mut();
    }
    if left_raw < 0 || left_raw > u16::MAX as c_long
        || right_raw < 0 || right_raw > u16::MAX as c_long
    {
        let exc = value_error_class();
        set_error(exc, "kerning(): glyph IDs must be in range 0..65535");
        return ptr::null_mut();
    }
    let left = left_raw as u16;
    let right = right_raw as u16;

    PyLong_FromLong(fp::kerning(font, left, right) as c_long)
}

// ─────────────────────────────────────────────────────────────────────────────
// Module methods table
// ─────────────────────────────────────────────────────────────────────────────
//
// `PyMethodDef` entries tell Python about each function. The table must be
// terminated with a sentinel (all-null entry).

static mut MODULE_METHODS: [PyMethodDef; 6] = [
    PyMethodDef {
        ml_name:  b"load\0".as_ptr() as *const c_char,
        ml_meth:  Some(py_load),
        ml_flags: METH_VARARGS,
        ml_doc:   b"load(data: bytes) -> capsule\n\nParse a font from bytes.\n\0"
                      .as_ptr() as *const c_char,
    },
    PyMethodDef {
        ml_name:  b"font_metrics\0".as_ptr() as *const c_char,
        ml_meth:  Some(py_font_metrics),
        ml_flags: METH_VARARGS,
        ml_doc:   b"font_metrics(font) -> dict\n\nReturn font-level metrics.\n\0"
                      .as_ptr() as *const c_char,
    },
    PyMethodDef {
        ml_name:  b"glyph_id\0".as_ptr() as *const c_char,
        ml_meth:  Some(py_glyph_id),
        ml_flags: METH_VARARGS,
        ml_doc:   b"glyph_id(font, codepoint: int) -> int | None\n\n\
                    Map a Unicode codepoint to a glyph ID.\n\0"
                      .as_ptr() as *const c_char,
    },
    PyMethodDef {
        ml_name:  b"glyph_metrics\0".as_ptr() as *const c_char,
        ml_meth:  Some(py_glyph_metrics),
        ml_flags: METH_VARARGS,
        ml_doc:   b"glyph_metrics(font, glyph_id: int) -> dict | None\n\n\
                    Return per-glyph advance width and LSB.\n\0"
                      .as_ptr() as *const c_char,
    },
    PyMethodDef {
        ml_name:  b"kerning\0".as_ptr() as *const c_char,
        ml_meth:  Some(py_kerning),
        ml_flags: METH_VARARGS,
        ml_doc:   b"kerning(font, left: int, right: int) -> int\n\n\
                    Return kern value for a glyph pair (0 if not found).\n\0"
                      .as_ptr() as *const c_char,
    },
    // Sentinel — terminates the methods array
    PyMethodDef {
        ml_name:  ptr::null(),
        ml_meth:  None,
        ml_flags: 0,
        ml_doc:   ptr::null(),
    },
];

// ─────────────────────────────────────────────────────────────────────────────
// Module definition
// ─────────────────────────────────────────────────────────────────────────────
//
// `PyModuleDef` is the singleton descriptor for our module. It must be a
// `static` because Python holds a pointer to it for the lifetime of the
// interpreter.

static mut MODULE_DEF: PyModuleDef = PyModuleDef {
    m_base: PyModuleDef_Base {
        // PyModuleDef_HEAD_INIT in C initialises ob_refcnt=1, ob_type=NULL,
        // m_init=NULL, m_index=0, m_copy=NULL.
        ob_base: [0u8; std::mem::size_of::<usize>() * 2],
        m_init: None,
        m_index: 0,
        m_copy: ptr::null_mut(),
    },
    m_name: b"font_parser_native\0".as_ptr() as *const c_char,
    m_doc:  b"Rust-backed font parser - zero-dependency OpenType/TrueType metrics.\0"
                .as_ptr() as *const c_char,
    m_size: -1, // -1 = module does not support sub-interpreter reinitialisation
    m_methods: &raw mut MODULE_METHODS as *mut PyMethodDef,
    m_slots: ptr::null_mut(),
    m_traverse: ptr::null_mut(),
    m_clear: ptr::null_mut(),
    m_free: ptr::null_mut(),
};

// ─────────────────────────────────────────────────────────────────────────────
// Module init — the entry point Python calls when it imports the module
// ─────────────────────────────────────────────────────────────────────────────
//
// The name MUST be `PyInit_<module_name>` where `<module_name>` is the name
// of the .so/.pyd file (without the file extension and ABI tag).
//
// `#[no_mangle]` prevents Rust from mangling the symbol — Python's import
// machinery looks for this exact symbol name via `dlsym` / `GetProcAddress`.

#[no_mangle]
pub unsafe extern "C" fn PyInit_font_parser_native() -> PyObjectPtr {
    PyModule_Create2(&raw mut MODULE_DEF, PYTHON_API_VERSION)
}
