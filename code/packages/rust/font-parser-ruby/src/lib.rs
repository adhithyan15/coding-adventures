//! # font-parser-ruby
//!
//! Ruby C extension that wraps the Rust `font-parser` core library.
//! Exposes five module functions under `CodingAdventures::FontParserNative`:
//!
//! ```ruby
//! require "font_parser_native"
//!
//! data = File.binread("Inter-Regular.ttf")
//! font = CodingAdventures::FontParserNative.load(data)
//!
//! m = CodingAdventures::FontParserNative.font_metrics(font)
//! # m[:units_per_em]  → 2048
//! # m[:family_name]   → "Inter"
//!
//! gid = CodingAdventures::FontParserNative.glyph_id(font, 0x0041)  # 'A'
//! gm  = CodingAdventures::FontParserNative.glyph_metrics(font, gid)
//! # gm[:advance_width]      → 1401
//! # gm[:left_side_bearing]  → 7
//!
//! k = CodingAdventures::FontParserNative.kerning(font, gid,
//!         CodingAdventures::FontParserNative.glyph_id(font, 0x0056))
//! # k → 0 (Inter v4 uses GPOS)
//! ```
//!
//! ## How font handles work
//!
//! `load()` uses `rb_data_object_wrap` to store a heap-allocated `Box<FontFile>`
//! pointer directly inside a Ruby `Data` object at creation time. There is no
//! two-phase init: the pointer is always valid when the Ruby object exists.
//!
//! Ruby GC calls `free_font_file_data` when the object is collected,
//! which does `Box::from_raw` to drop the Rust-side memory.
//!
//! ## Security notes
//!
//! - `rb_load` copies the Ruby String bytes into a Rust-owned `Vec<u8>` before
//!   calling `fp::load` to guard against GC compaction moving the String buffer
//!   between the pointer read and the actual parse (TOCTOU mitigation).
//!
//! - `unwrap_font` checks `rb_obj_is_kind_of` before dereferencing the pointer to
//!   prevent type-confusion attacks from callers passing arbitrary Ruby objects.
//!
//! - `rb_load` calls `rb_data_object_wrap` directly with the live FontFile pointer
//!   (never wraps null, never writes past the API).  This avoids the dangerous
//!   "alloc null then overwrite offset-4" pattern which relies on undocumented
//!   RData struct layout.

use std::ffi::{c_char, c_int, c_long, c_void, CString};

use ruby_bridge::{
    VALUE, QNIL, QFALSE,
    define_module, define_module_under,
    define_class_under, define_module_function_raw,
    str_to_rb,
    raise_arg_error, raise_runtime_error,
    object_class,
};
use font_parser::{self as fp, FontError};

// ─────────────────────────────────────────────────────────────────────────────
// Additional Ruby C API functions not in ruby-bridge
// ─────────────────────────────────────────────────────────────────────────────

extern "C" {
    // rb_hash_new / rb_hash_aset: create and populate Ruby Hash objects.
    fn rb_hash_new() -> VALUE;
    fn rb_hash_aset(hash: VALUE, key: VALUE, val: VALUE) -> VALUE;

    // rb_intern: intern a C string as a Ruby Symbol ID.
    fn rb_intern(name: *const c_char) -> usize;

    // rb_id2sym: convert a Symbol ID to its VALUE.
    // Stable since Ruby 2.0 — use instead of the ID2SYM macro.
    fn rb_id2sym(id: usize) -> VALUE;

    // rb_string_value_ptr: coerce the VALUE to a String and return its C buffer.
    fn rb_string_value_ptr(ptr: *mut VALUE) -> *const c_char;

    // rb_str_strlen: return byte length of a Ruby String.
    fn rb_str_strlen(str_val: VALUE) -> c_long;

    // rb_fix2int: convert a Fixnum VALUE to a C int.
    fn rb_fix2int(v: VALUE) -> c_int;

    // rb_int2inum: convert a C long to a Ruby Integer VALUE.
    fn rb_int2inum(v: c_long) -> VALUE;

    // rb_data_object_wrap: create a Ruby Data object wrapping a C pointer.
    // This is the ONLY supported way to associate a C pointer with a Ruby object.
    // The dfree function is called by the GC when the object is collected.
    fn rb_data_object_wrap(
        klass: VALUE,
        data: *mut c_void,
        dmark: Option<unsafe extern "C" fn(*mut c_void)>,
        dfree: Option<unsafe extern "C" fn(*mut c_void)>,
    ) -> VALUE;

    // rb_data_object_get: extract the C pointer stored by rb_data_object_wrap.
    // This is the counterpart to rb_data_object_wrap and is ABI-stable.
    fn rb_data_object_get(obj: VALUE) -> *mut c_void;

    // rb_obj_is_kind_of: type-safe class membership check.
    // Returns Qtrue if `obj` is an instance of `klass` or a subclass; Qfalse otherwise.
    fn rb_obj_is_kind_of(obj: VALUE, klass: VALUE) -> VALUE;
}

/// Convert a Rust `&str` to a Ruby Symbol.
fn sym(name: &str) -> VALUE {
    let c_name = CString::new(name).expect("name must not contain NUL");
    unsafe {
        let id = rb_intern(c_name.as_ptr());
        rb_id2sym(id)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper: build a Ruby Hash from FontMetrics
// ─────────────────────────────────────────────────────────────────────────────

unsafe fn metrics_to_hash(m: &fp::FontMetrics) -> VALUE {
    let h = rb_hash_new();

    macro_rules! set_int {
        ($key:expr, $val:expr) => {
            rb_hash_aset(h, sym($key), rb_int2inum($val as c_long));
        };
    }
    macro_rules! set_str {
        ($key:expr, $val:expr) => {
            rb_hash_aset(h, sym($key), str_to_rb($val));
        };
    }
    macro_rules! set_opt_int {
        ($key:expr, $val:expr) => {
            let v = match $val {
                Some(n) => rb_int2inum(n as c_long),
                None => QNIL,
            };
            rb_hash_aset(h, sym($key), v);
        };
    }

    set_int!("units_per_em",   m.units_per_em);
    set_int!("ascender",       m.ascender);
    set_int!("descender",      m.descender);
    set_int!("line_gap",       m.line_gap);
    set_opt_int!("x_height",   m.x_height);
    set_opt_int!("cap_height", m.cap_height);
    set_int!("num_glyphs",     m.num_glyphs);
    set_str!("family_name",    &m.family_name);
    set_str!("subfamily_name", &m.subfamily_name);

    h
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper: build a Ruby Hash from GlyphMetrics
// ─────────────────────────────────────────────────────────────────────────────

unsafe fn glyph_metrics_to_hash(gm: &fp::GlyphMetrics) -> VALUE {
    let h = rb_hash_new();
    rb_hash_aset(h, sym("advance_width"),     rb_int2inum(gm.advance_width as c_long));
    rb_hash_aset(h, sym("left_side_bearing"), rb_int2inum(gm.left_side_bearing as c_long));
    h
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper: map FontError → Ruby RuntimeError message
// ─────────────────────────────────────────────────────────────────────────────

fn font_error_message(err: FontError) -> String {
    match err {
        FontError::InvalidMagic          => "invalid magic: not a TrueType/OpenType font".to_string(),
        FontError::InvalidHeadMagic      => "invalid head magic number".to_string(),
        FontError::TableNotFound(t)      => format!("required table not found: {}", t),
        FontError::BufferTooShort        => "buffer too short".to_string(),
        FontError::UnsupportedCmapFormat => "unsupported cmap format".to_string(),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// The FontFile class: a Ruby Data object wrapping Box<FontFile>
// ─────────────────────────────────────────────────────────────────────────────
//
// We define a Ruby class `CodingAdventures::FontParserNative::FontFile`.
// Instances are created *only* by `rb_load`; there is no public `.new`.
// The GC destructor (`free_font_file_data`) drops the `Box<FontFile>`.

/// Global: the `CodingAdventures::FontParserNative::FontFile` class VALUE.
/// Written once in `Init_font_parser_native`; read in `rb_load` and `unwrap_font`.
static mut FONT_FILE_CLASS: VALUE = 0;

/// GC destructor — called by Ruby when a FontFile object is collected.
///
/// Reconstructs the `Box<FontFile>` from the raw pointer and drops it,
/// freeing the Rust-side allocation.
unsafe extern "C" fn free_font_file_data(ptr: *mut c_void) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr as *mut fp::FontFile);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Module functions
// ─────────────────────────────────────────────────────────────────────────────

/// `CodingAdventures::FontParserNative.load(data) → FontFile`
///
/// Parses a font from a binary Ruby String (e.g. `File.binread(...)`).
/// Returns an opaque `FontFile` Data object.
/// Raises `RuntimeError` on parse failure, `ArgumentError` on wrong type.
///
/// Ruby calls this as `fn(data: VALUE, self: VALUE) -> VALUE` (argc = 1).
///
/// ## Security
///
/// The Ruby String bytes are copied into a Rust-owned `Vec<u8>` immediately
/// after reading the pointer and length. This prevents a TOCTOU race where
/// Ruby's GC compactor could move the String buffer between the pointer read
/// and the actual parse call.
///
/// We use `rb_data_object_wrap` directly with the live pointer (never null),
/// which is the only sanctioned Ruby C API for Data objects. This avoids the
/// unsafe "allocate null then write to offset-4" anti-pattern.
unsafe extern "C" fn rb_load(data: VALUE, _self: VALUE) -> VALUE {
    // Coerce argument to a String and get its buffer pointer + length.
    let mut v = data;
    let ptr = rb_string_value_ptr(&mut v);
    let len = rb_str_strlen(data);
    if ptr.is_null() || len < 0 {
        raise_arg_error("load: argument must be a String containing font bytes");
    }

    // SECURITY (TOCTOU mitigation): copy bytes into a Rust-owned Vec before
    // making any further Ruby API calls that could trigger GC compaction,
    // which would move the String's backing buffer and invalidate `ptr`.
    let bytes: Vec<u8> = std::slice::from_raw_parts(ptr as *const u8, len as usize).to_vec();

    let font_file = match fp::load(&bytes) {
        Ok(f) => f,
        Err(e) => raise_runtime_error(&font_error_message(e)),
    };

    // SECURITY: use rb_data_object_wrap with the actual, live pointer — never
    // null-init and overwrite later. This is the only Ruby-sanctioned API.
    // The GC will call free_font_file_data when the object is collected.
    let boxed = Box::into_raw(Box::new(font_file));
    rb_data_object_wrap(FONT_FILE_CLASS, boxed as *mut c_void, None, Some(free_font_file_data))
}

/// `CodingAdventures::FontParserNative.font_metrics(font) → Hash`
unsafe extern "C" fn rb_font_metrics(font_obj: VALUE, _self: VALUE) -> VALUE {
    let font = unwrap_font(font_obj);
    let m = fp::font_metrics(font);
    metrics_to_hash(&m)
}

/// `CodingAdventures::FontParserNative.glyph_id(font, codepoint) → Integer | nil`
unsafe extern "C" fn rb_glyph_id(font_obj: VALUE, cp_val: VALUE, _self: VALUE) -> VALUE {
    let font = unwrap_font(font_obj);

    // SECURITY: validate range to prevent silent u16 truncation.
    let cp_raw = rb_fix2int(cp_val) as c_long;
    if cp_raw < 0 {
        raise_arg_error("glyph_id: codepoint must be non-negative");
    }

    match fp::glyph_id(font, cp_raw as u32) {
        Some(gid) => rb_int2inum(gid as c_long),
        None => QNIL,
    }
}

/// `CodingAdventures::FontParserNative.glyph_metrics(font, glyph_id) → Hash | nil`
unsafe extern "C" fn rb_glyph_metrics(font_obj: VALUE, gid_val: VALUE, _self: VALUE) -> VALUE {
    let font = unwrap_font(font_obj);

    // SECURITY: validate range before casting to u16.
    let gid_raw = rb_fix2int(gid_val) as c_long;
    if gid_raw < 0 || gid_raw > u16::MAX as c_long {
        raise_arg_error("glyph_metrics: glyph_id must be in range 0..65535");
    }

    match fp::glyph_metrics(font, gid_raw as u16) {
        Some(gm) => glyph_metrics_to_hash(&gm),
        None => QNIL,
    }
}

/// `CodingAdventures::FontParserNative.kerning(font, left, right) → Integer`
unsafe extern "C" fn rb_kerning(
    font_obj: VALUE,
    left_val: VALUE,
    right_val: VALUE,
    _self: VALUE,
) -> VALUE {
    let font = unwrap_font(font_obj);

    // SECURITY: validate range before casting to u16.
    let left_raw = rb_fix2int(left_val) as c_long;
    let right_raw = rb_fix2int(right_val) as c_long;
    if left_raw < 0 || left_raw > u16::MAX as c_long
        || right_raw < 0 || right_raw > u16::MAX as c_long
    {
        raise_arg_error("kerning: glyph IDs must be in range 0..65535");
    }

    rb_int2inum(fp::kerning(font, left_raw as u16, right_raw as u16) as c_long)
}

/// Extract `&FontFile` from a Ruby Data object.
///
/// ## Security
///
/// Calls `rb_obj_is_kind_of` to verify the object is actually a
/// `FontFile` instance before dereferencing the data pointer.
/// This prevents type-confusion attacks where an attacker passes
/// an arbitrary Ruby object to a module function.
///
/// Uses `rb_data_object_get` (the official Ruby C API accessor) instead
/// of manual pointer arithmetic — the only safe way to read the pointer.
unsafe fn unwrap_font(obj: VALUE) -> &'static fp::FontFile {
    // SECURITY: type-check before dereferencing.
    if rb_obj_is_kind_of(obj, FONT_FILE_CLASS) == QFALSE {
        raise_runtime_error("expected a FontFile object");
    }

    let raw = rb_data_object_get(obj) as *const fp::FontFile;
    if raw.is_null() {
        raise_runtime_error("FontFile: internal data pointer is null");
    }
    &*raw
}

// ─────────────────────────────────────────────────────────────────────────────
// Init — called by Ruby when `require "font_parser_native"` executes
// ─────────────────────────────────────────────────────────────────────────────
//
// The name MUST be `Init_<lib_name>` where `<lib_name>` is the `.so`/`.bundle`
// file name without the extension.

#[no_mangle]
pub unsafe extern "C" fn Init_font_parser_native() {
    // Define the module hierarchy: CodingAdventures::FontParserNative
    let ca  = define_module("CodingAdventures");
    let fpn = define_module_under(ca, "FontParserNative");

    // Define the opaque FontFile class. No alloc func — instances are only
    // created via load(), never via FontFile.new. The dfree is registered
    // per-object by rb_data_object_wrap inside rb_load.
    let ff_class = define_class_under(fpn, "FontFile", object_class());
    FONT_FILE_CLASS = ff_class;

    // Bind module functions with fixed arity.
    //
    // Ruby's rb_define_module_function with argc=N means the Rust function
    // receives (arg1, ..., argN, self_module) — N named args plus self.

    define_module_function_raw(fpn, "load",         rb_load as *const c_void, 1);
    define_module_function_raw(fpn, "font_metrics",  rb_font_metrics as *const c_void, 1);
    define_module_function_raw(fpn, "glyph_id",      rb_glyph_id as *const c_void, 2);
    define_module_function_raw(fpn, "glyph_metrics", rb_glyph_metrics as *const c_void, 2);
    define_module_function_raw(fpn, "kerning",        rb_kerning as *const c_void, 3);
}
