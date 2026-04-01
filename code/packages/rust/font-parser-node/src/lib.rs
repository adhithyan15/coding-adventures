//! # font-parser-node
//!
//! Node.js N-API addon that wraps the Rust `font-parser` core library.
//! Exposes five functions on the addon's `exports` object:
//!
//! ```javascript
//! const fp = require("./font_parser_native.node");
//!
//! const data = require("fs").readFileSync("Inter-Regular.ttf");
//! const font = fp.load(data);
//!
//! const m = fp.fontMetrics(font);
//! // m.unitsPerEm  → 2048
//! // m.familyName  → "Inter"
//!
//! const gidA = fp.glyphId(font, 0x0041);
//! const gm   = fp.glyphMetrics(font, gidA);
//! // gm.advanceWidth     → 1401
//! // gm.leftSideBearing  → 7
//!
//! const k = fp.kerning(font, gidA, fp.glyphId(font, 0x0056));
//! // k → 0  (Inter v4 uses GPOS)
//! ```
//!
//! ## How font handles work
//!
//! `fp.load()` creates a JS object of a "FontFile" class defined via
//! `napi_define_class`. The constructor wraps a `Box<FontFile>` using
//! `napi_wrap` with a finalizer that calls `Box::from_raw` when the GC
//! collects the object.
//!
//! All other functions unwrap the object with `napi_unwrap` to get the
//! `&FontFile` reference, then call into the pure-Rust core.
//!
//! ## JavaScript naming
//!
//! We use camelCase for the exported functions to match Node.js/JavaScript
//! conventions: `fontMetrics`, `glyphId`, `glyphMetrics`, `kerning`.
//!
//! ## N-API version
//!
//! This addon targets N-API version 4 (Node.js 10.16+ / 12+), which is
//! the minimum version that supports all used APIs.
//!
//! ## Security notes
//!
//! - `FONT_FILE_CTOR` is stored as an `AtomicUsize` to prevent data races
//!   when Node.js Worker threads load the addon concurrently (Finding 3.1).
//!
//! - The Buffer bytes are copied into a Rust-owned `Vec<u8>` before parsing
//!   to prevent use-after-free if the ArrayBuffer is transferred/detached
//!   during `fp::load` (Finding 3.2).
//!
//! - `napi_get_value_int64` return status is checked at every call site
//!   (Finding 3.4). Invalid argument types throw JS errors rather than
//!   silently proceeding with zero.
//!
//! - `napi_new_instance` status is checked; on failure the Box is not leaked
//!   (Finding 3.5).

#![allow(non_camel_case_types, non_snake_case)]

use std::ffi::{c_void, CString};
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};

use node_bridge::{
    napi_env, napi_value, napi_callback_info,
    napi_status, NAPI_OK,
    str_to_js,
    get_cb_info,
    throw_error,
    set_named_property,
    create_function,
    napi_create_int64, napi_get_undefined,
    napi_wrap, napi_unwrap,
    napi_define_class,
    napi_set_named_property,
};
use font_parser::{self as fp, FontError};

// ─────────────────────────────────────────────────────────────────────────────
// Additional N-API functions not in node-bridge
// ─────────────────────────────────────────────────────────────────────────────

extern "C" {
    fn napi_create_object(env: napi_env, result: *mut napi_value) -> napi_status;
    fn napi_get_value_int64(env: napi_env, value: napi_value, result: *mut i64) -> napi_status;
    fn napi_get_null(env: napi_env, result: *mut napi_value) -> napi_status;
    fn napi_get_buffer_info(
        env: napi_env,
        value: napi_value,
        data: *mut *mut c_void,
        byte_length: *mut usize,
    ) -> napi_status;
    fn napi_new_instance(
        env: napi_env,
        constructor: napi_value,
        argc: usize,
        argv: *const napi_value,
        result: *mut napi_value,
    ) -> napi_status;
}

// ─────────────────────────────────────────────────────────────────────────────
// FONT_FILE_CTOR — thread-safe storage for the FontFile constructor VALUE
// ─────────────────────────────────────────────────────────────────────────────
//
// SECURITY (Finding 3.1): `napi_value` is a pointer-sized opaque handle.
// Storing it in a `static mut` is a data race when Node.js Worker threads
// load the addon concurrently. We use an `AtomicUsize` (same size as a
// pointer on 64-bit and 32-bit) so reads/writes are race-free.
//
// Release ordering on write, Acquire on read — this is the standard
// "publish once, read many" pattern.

static FONT_FILE_CTOR: AtomicUsize = AtomicUsize::new(0);

fn store_ctor(val: napi_value) {
    FONT_FILE_CTOR.store(val as usize, Ordering::Release);
}

fn load_ctor() -> napi_value {
    FONT_FILE_CTOR.load(Ordering::Acquire) as napi_value
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn object_new(env: napi_env) -> napi_value {
    let mut result: napi_value = ptr::null_mut();
    unsafe { napi_create_object(env, &mut result) };
    result
}

unsafe fn set_int_prop(env: napi_env, obj: napi_value, key: &str, val: i64) {
    let mut v: napi_value = ptr::null_mut();
    napi_create_int64(env, val, &mut v);
    let k = CString::new(key).unwrap();
    napi_set_named_property(env, obj, k.as_ptr(), v);
}

unsafe fn set_str_prop(env: napi_env, obj: napi_value, key: &str, val: &str) {
    let v = str_to_js(env, val);
    let k = CString::new(key).unwrap();
    napi_set_named_property(env, obj, k.as_ptr(), v);
}

unsafe fn set_opt_int_prop(env: napi_env, obj: napi_value, key: &str, val: Option<i16>) {
    let v = match val {
        Some(n) => { let mut r = ptr::null_mut(); napi_create_int64(env, n as i64, &mut r); r }
        None    => { let mut r = ptr::null_mut(); napi_get_null(env, &mut r); r }
    };
    let k = CString::new(key).unwrap();
    napi_set_named_property(env, obj, k.as_ptr(), v);
}

unsafe fn metrics_to_js_obj(env: napi_env, m: &fp::FontMetrics) -> napi_value {
    let obj = object_new(env);
    set_int_prop(env, obj, "unitsPerEm",    m.units_per_em as i64);
    set_int_prop(env, obj, "ascender",      m.ascender as i64);
    set_int_prop(env, obj, "descender",     m.descender as i64);
    set_int_prop(env, obj, "lineGap",       m.line_gap as i64);
    set_opt_int_prop(env, obj, "xHeight",   m.x_height);
    set_opt_int_prop(env, obj, "capHeight",  m.cap_height);
    set_int_prop(env, obj, "numGlyphs",     m.num_glyphs as i64);
    set_str_prop(env, obj, "familyName",    &m.family_name);
    set_str_prop(env, obj, "subfamilyName", &m.subfamily_name);
    obj
}

unsafe fn glyph_metrics_to_js_obj(env: napi_env, gm: &fp::GlyphMetrics) -> napi_value {
    let obj = object_new(env);
    set_int_prop(env, obj, "advanceWidth",    gm.advance_width as i64);
    set_int_prop(env, obj, "leftSideBearing", gm.left_side_bearing as i64);
    obj
}

/// Map `FontError` to a JS Error and return `undefined`.
unsafe fn throw_font_error(env: napi_env, err: FontError) -> napi_value {
    let msg = match err {
        FontError::InvalidMagic          => "invalid magic: not a TrueType/OpenType font".to_string(),
        FontError::InvalidHeadMagic      => "invalid head magic number".to_string(),
        FontError::TableNotFound(t)      => format!("required table not found: {}", t),
        FontError::BufferTooShort        => "buffer too short".to_string(),
        FontError::UnsupportedCmapFormat => "unsupported cmap format".to_string(),
    };
    throw_error(env, &msg);
    let mut undef: napi_value = ptr::null_mut();
    napi_get_undefined(env, &mut undef);
    undef
}

/// Return `undefined` — convenience shorthand.
unsafe fn undef(env: napi_env) -> napi_value {
    let mut u: napi_value = ptr::null_mut();
    napi_get_undefined(env, &mut u);
    u
}

// ─────────────────────────────────────────────────────────────────────────────
// FontFile class — wraps Box<FontFile> via napi_wrap
// ─────────────────────────────────────────────────────────────────────────────

/// GC finalizer: called by Node.js when the FontFile JS object is collected.
unsafe extern "C" fn finalize_font_file(
    _env: napi_env,
    data: *mut c_void,
    _hint: *mut c_void,
) {
    if !data.is_null() {
        let _ = Box::from_raw(data as *mut fp::FontFile);
    }
}

/// FontFile constructor — creates an empty shell; `napi_load` wraps data into it.
unsafe extern "C" fn font_file_constructor(
    env: napi_env,
    info: napi_callback_info,
) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    this
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper: extract &FontFile from a wrapped JS object
// ─────────────────────────────────────────────────────────────────────────────

unsafe fn get_font_ptr(env: napi_env, obj: napi_value) -> Option<&'static fp::FontFile> {
    let mut p: *mut c_void = ptr::null_mut();
    let status = napi_unwrap(env, obj, &mut p);
    if status != NAPI_OK || p.is_null() {
        throw_error(env, "argument is not a valid FontFile");
        return None;
    }
    Some(&*(p as *const fp::FontFile))
}

/// Extract and validate a u16 glyph ID from a JS argument.
///
/// Returns `None` (with a JS exception set) if the value is not a number
/// or is outside 0..65535.
unsafe fn get_glyph_id_arg(
    env: napi_env,
    val: napi_value,
    name: &str,
) -> Option<u16> {
    let mut n: i64 = 0;
    let status = napi_get_value_int64(env, val, &mut n);
    if status != NAPI_OK {
        let msg = format!("{}: argument must be a number", name);
        throw_error(env, &msg);
        return None;
    }
    if n < 0 || n > u16::MAX as i64 {
        let msg = format!("{}: value must be in range 0..65535", name);
        throw_error(env, &msg);
        return None;
    }
    Some(n as u16)
}

// ─────────────────────────────────────────────────────────────────────────────
// Exported functions
// ─────────────────────────────────────────────────────────────────────────────

/// `load(buffer: Buffer) → FontFile`
///
/// ## Security
///
/// SECURITY (Finding 3.2): We copy the Buffer bytes into a Rust-owned `Vec`
/// before calling `fp::load`. This prevents a use-after-free if the JS caller
/// detaches the underlying `ArrayBuffer` (via `buffer.buffer.transfer()`)
/// between when we read the pointer and when `fp::load` reads the data.
///
/// SECURITY (Finding 3.5): `napi_new_instance` status is checked before
/// creating the `Box`. If instance creation fails, we throw and return without
/// leaking the FontFile allocation.
unsafe extern "C" fn napi_load(env: napi_env, info: napi_callback_info) -> napi_value {
    let (_, args) = get_cb_info(env, info, 1);
    if args.is_empty() {
        throw_error(env, "load() requires one argument: Buffer");
        return undef(env);
    }

    let mut data_ptr: *mut c_void = ptr::null_mut();
    let mut data_len: usize = 0;
    let status = napi_get_buffer_info(env, args[0], &mut data_ptr, &mut data_len);
    if status != NAPI_OK {
        throw_error(env, "load() argument must be a Buffer");
        return undef(env);
    }

    // SECURITY: copy bytes before any further API calls that could trigger GC
    // or allow the backing ArrayBuffer to be transferred / detached.
    let bytes: Vec<u8> = std::slice::from_raw_parts(data_ptr as *const u8, data_len).to_vec();

    let font_file = match fp::load(&bytes) {
        Ok(f) => f,
        Err(e) => return throw_font_error(env, e),
    };

    // SECURITY (Finding 3.5): check napi_new_instance status BEFORE boxing.
    // If instance creation fails, we must NOT call Box::into_raw — there is
    // nothing to wrap and the pointer would leak.
    let ctor = load_ctor();
    if ctor.is_null() {
        throw_error(env, "load(): FontFile class not initialised");
        return undef(env);
    }
    let mut instance: napi_value = ptr::null_mut();
    let no_args: [napi_value; 0] = [];
    let inst_status = napi_new_instance(env, ctor, 0, no_args.as_ptr(), &mut instance);
    if inst_status != NAPI_OK || instance.is_null() {
        throw_error(env, "load(): failed to create FontFile wrapper");
        return undef(env);
    }

    let boxed = Box::into_raw(Box::new(font_file));
    let wrap_status = napi_wrap(
        env, instance, boxed as *mut c_void,
        Some(finalize_font_file), ptr::null_mut(), ptr::null_mut(),
    );
    if wrap_status != NAPI_OK {
        // Failed to wrap — drop the Box to avoid a leak, then throw.
        let _ = Box::from_raw(boxed);
        throw_error(env, "load(): failed to wrap FontFile data");
        return undef(env);
    }

    instance
}

/// `fontMetrics(font) → object`
unsafe extern "C" fn napi_font_metrics(env: napi_env, info: napi_callback_info) -> napi_value {
    let (_, args) = get_cb_info(env, info, 1);
    if args.is_empty() {
        throw_error(env, "fontMetrics() requires a FontFile argument");
        return undef(env);
    }
    let font = match get_font_ptr(env, args[0]) {
        Some(f) => f,
        None => return undef(env),
    };
    let m = fp::font_metrics(font);
    metrics_to_js_obj(env, &m)
}

/// `glyphId(font, codepoint: number) → number | null`
///
/// SECURITY (Finding 3.4): `napi_get_value_int64` status is checked. Passing
/// a non-number (e.g. a string or object) throws a TypeError rather than
/// silently using 0.
unsafe extern "C" fn napi_glyph_id(env: napi_env, info: napi_callback_info) -> napi_value {
    let (_, args) = get_cb_info(env, info, 2);
    if args.len() < 2 {
        throw_error(env, "glyphId() requires two arguments: font, codepoint");
        return undef(env);
    }
    let font = match get_font_ptr(env, args[0]) {
        Some(f) => f,
        None => return undef(env),
    };
    // For codepoint, accept the full u32 range (0..=0x10FFFF). We use i64
    // internally and cast to u32; glyph_id will return None for out-of-BMP.
    let mut cp_i64: i64 = 0;
    let status = napi_get_value_int64(env, args[1], &mut cp_i64);
    if status != NAPI_OK {
        throw_error(env, "glyphId(): codepoint must be a number");
        return undef(env);
    }
    if cp_i64 < 0 {
        throw_error(env, "glyphId(): codepoint must be non-negative");
        return undef(env);
    }
    let cp = cp_i64 as u32;

    match fp::glyph_id(font, cp) {
        Some(gid) => { let mut v = ptr::null_mut(); napi_create_int64(env, gid as i64, &mut v); v }
        None      => { let mut n = ptr::null_mut(); napi_get_null(env, &mut n); n }
    }
}

/// `glyphMetrics(font, glyphId: number) → object | null`
unsafe extern "C" fn napi_glyph_metrics(env: napi_env, info: napi_callback_info) -> napi_value {
    let (_, args) = get_cb_info(env, info, 2);
    if args.len() < 2 {
        throw_error(env, "glyphMetrics() requires two arguments: font, glyphId");
        return undef(env);
    }
    let font = match get_font_ptr(env, args[0]) {
        Some(f) => f,
        None => return undef(env),
    };
    let gid = match get_glyph_id_arg(env, args[1], "glyphMetrics()") {
        Some(g) => g,
        None => return undef(env),
    };

    match fp::glyph_metrics(font, gid) {
        Some(gm) => glyph_metrics_to_js_obj(env, &gm),
        None     => { let mut n = ptr::null_mut(); napi_get_null(env, &mut n); n }
    }
}

/// `kerning(font, left: number, right: number) → number`
unsafe extern "C" fn napi_kerning(env: napi_env, info: napi_callback_info) -> napi_value {
    let (_, args) = get_cb_info(env, info, 3);
    if args.len() < 3 {
        throw_error(env, "kerning() requires three arguments: font, left, right");
        return undef(env);
    }
    let font = match get_font_ptr(env, args[0]) {
        Some(f) => f,
        None => return undef(env),
    };
    let left = match get_glyph_id_arg(env, args[1], "kerning() left") {
        Some(g) => g,
        None => return undef(env),
    };
    let right = match get_glyph_id_arg(env, args[2], "kerning() right") {
        Some(g) => g,
        None => return undef(env),
    };

    let kern = fp::kerning(font, left, right);
    let mut v = ptr::null_mut();
    napi_create_int64(env, kern as i64, &mut v);
    v
}

// ─────────────────────────────────────────────────────────────────────────────
// Module registration — the entry point Node.js calls on require()
// ─────────────────────────────────────────────────────────────────────────────

#[no_mangle]
pub unsafe extern "C" fn napi_register_module_v1(
    env: napi_env,
    exports: napi_value,
) -> napi_value {
    let ff_name = CString::new("FontFile").unwrap();
    let mut ff_class: napi_value = ptr::null_mut();
    napi_define_class(
        env,
        ff_name.as_ptr(),
        usize::MAX,
        Some(font_file_constructor),
        ptr::null_mut(),
        0,
        ptr::null(),
        &mut ff_class,
    );
    // SECURITY (Finding 3.1): use atomic store (Release) so Worker threads
    // that read with Acquire ordering see the fully-initialised class value.
    store_ctor(ff_class);

    set_named_property(env, exports, "FontFile", ff_class);

    macro_rules! export_fn {
        ($name:expr, $cb:expr) => {
            let f = create_function(env, $name, Some($cb));
            set_named_property(env, exports, $name, f);
        };
    }

    export_fn!("load",         napi_load);
    export_fn!("fontMetrics",  napi_font_metrics);
    export_fn!("glyphId",      napi_glyph_id);
    export_fn!("glyphMetrics", napi_glyph_metrics);
    export_fn!("kerning",      napi_kerning);

    exports
}
