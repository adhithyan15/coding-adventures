//! spreadsheet-android-jni — JNI bridge from the `com.example.visicalc` Android
//! host to the shared Rust spreadsheet engine (`spreadsheet-core`, reached through
//! the same `spreadsheet-core-wasm::SpreadsheetSession` the C-ABI facade wraps).
//!
//! Why JNI and not the JVM Foreign Function & Memory API? The Compose *Desktop*
//! demo loads the engine through FFM (JDK 21+). Android's ART runtime has no FFM,
//! so a native library there is reached the classic way: `System.loadLibrary` +
//! `native` methods whose symbols are `Java_<package>_<Class>_<method>`. We build
//! those `extern "C"` exports directly on top of the zero-dependency `jni-bridge`
//! crate (no `jni`/`jni-sys`/`bindgen`), cross-compile this crate to a per-ABI
//! `.so`, and drop it in the app's `jniLibs/`.
//!
//! The session lives on the native heap as a boxed `SpreadsheetSession`; the Java
//! side holds it as an opaque `long` handle and must call `nativeFree` when done.
//! All calls are expected on a single (UI) thread — the handle is not synchronised.

use jni_bridge::{jboolean, jbyteArray, jclass, jint, jlong, jstring, JNIEnv};
use jni_bridge::{jni_get_byte_array, jni_new_byte_array_from};
use jni_bridge::{jni_get_string_utf, jni_new_string_utf};
use spreadsheet_core_wasm::SpreadsheetSession;

/// `long` handle → `&mut SpreadsheetSession`. Returns from the caller with a
/// default value when the handle is null (a disposed/never-created session).
macro_rules! session {
    ($ptr:expr, $env:expr, $ret:expr) => {{
        if $ptr == 0 {
            return $ret;
        }
        &mut *($ptr as *mut SpreadsheetSession)
    }};
}

/// Create a new, empty session. Free it with `nativeFree`.
///
/// # Safety
/// Called by the JVM. Returns a heap pointer as a `long`; the Java side must pass
/// it back unchanged and free it exactly once.
#[no_mangle]
pub unsafe extern "C" fn Java_com_example_visicalc_Engine_nativeNewSession(
    _env: *mut JNIEnv,
    _class: jclass,
) -> jlong {
    let session = Box::new(SpreadsheetSession::new());
    Box::into_raw(session) as jlong
}

/// Free a session created by `nativeNewSession`. Safe to call with 0 (no-op).
///
/// # Safety
/// `ptr` must be a handle from `nativeNewSession` that has not already been freed.
#[no_mangle]
pub unsafe extern "C" fn Java_com_example_visicalc_Engine_nativeFree(
    _env: *mut JNIEnv,
    _class: jclass,
    ptr: jlong,
) {
    if ptr != 0 {
        drop(Box::from_raw(ptr as *mut SpreadsheetSession));
    }
}

/// `set_cell(a1, raw)` → JSON status string (`{"ok":true}` / error).
///
/// # Safety
/// `ptr` must be a live session handle; `a1`/`raw` valid `jstring`s or null.
#[no_mangle]
pub unsafe extern "C" fn Java_com_example_visicalc_Engine_nativeSetCell(
    env: *mut JNIEnv,
    _class: jclass,
    ptr: jlong,
    a1: jstring,
    raw: jstring,
) -> jstring {
    let session = session!(ptr, env, jni_new_string_utf(env, "{\"ok\":false}"));
    let a1 = jni_get_string_utf(env, a1).unwrap_or_default();
    let raw = jni_get_string_utf(env, raw).unwrap_or_default();
    let status = session.set_cell(&a1, &raw);
    jni_new_string_utf(env, &status)
}

/// `get_display(a1)` → the cell's formatted display string.
///
/// # Safety
/// `ptr` must be a live session handle; `a1` a valid `jstring` or null.
#[no_mangle]
pub unsafe extern "C" fn Java_com_example_visicalc_Engine_nativeGetDisplay(
    env: *mut JNIEnv,
    _class: jclass,
    ptr: jlong,
    a1: jstring,
) -> jstring {
    let session = session!(ptr, env, jni_new_string_utf(env, ""));
    let a1 = jni_get_string_utf(env, a1).unwrap_or_default();
    jni_new_string_utf(env, &session.get_display(&a1))
}

/// `get_raw(a1)` → the cell's typed source string (for the formula bar).
///
/// # Safety
/// `ptr` must be a live session handle; `a1` a valid `jstring` or null.
#[no_mangle]
pub unsafe extern "C" fn Java_com_example_visicalc_Engine_nativeGetRaw(
    env: *mut JNIEnv,
    _class: jclass,
    ptr: jlong,
    a1: jstring,
) -> jstring {
    let session = session!(ptr, env, jni_new_string_utf(env, ""));
    let a1 = jni_get_string_utf(env, a1).unwrap_or_default();
    jni_new_string_utf(env, &session.get_raw(&a1))
}

/// `get_display_window(row0, col0, row1, col1)` → display-window JSON (each cell
/// rendered through its format). The Kotlin side parses this into the grid rows.
///
/// # Safety
/// `ptr` must be a live session handle.
#[no_mangle]
pub unsafe extern "C" fn Java_com_example_visicalc_Engine_nativeGetDisplayWindow(
    env: *mut JNIEnv,
    _class: jclass,
    ptr: jlong,
    row0: jint,
    col0: jint,
    row1: jint,
    col1: jint,
) -> jstring {
    let session = session!(ptr, env, jni_new_string_utf(env, "{}"));
    let json = session.get_display_window(row0 as u32, col0 as u32, row1 as u32, col1 as u32);
    jni_new_string_utf(env, &json)
}

/// `column_letters(index)` → the A1-style letters for a 1-based column index.
///
/// # Safety
/// `ptr` must be a live session handle.
#[no_mangle]
pub unsafe extern "C" fn Java_com_example_visicalc_Engine_nativeColumnLetters(
    env: *mut JNIEnv,
    _class: jclass,
    ptr: jlong,
    index: jint,
) -> jstring {
    let session = session!(ptr, env, jni_new_string_utf(env, ""));
    jni_new_string_utf(env, &session.column_letters(index as u32))
}

// ─────────────────────────────────────────────────────────────────────────────
// File open / save — a Java `byte[]` in, a `byte[]` out.
// ─────────────────────────────────────────────────────────────────────────────
//
// The Android host reads a file the user picked into a `byte[]` and passes it to
// `nativeLoad<Fmt>` (returns a `boolean`); `nativeSave<Fmt>` returns a `byte[]`
// the host writes back out. A `byte[]` carries the raw file bytes intact — an
// `.xlsx` is a ZIP, an `.xls` an OLE2 file — with no UTF-8 / NUL surprises. This
// is the Android arm of the same open/save story as the WASM and C-ABI facades,
// over the one engine. `.xlsx` keeps live formulas; the others are
// lower-fidelity per `spreadsheet-io`.

/// Open `.xlsx` file bytes as the current document. Returns `true` if opened,
/// `false` if the bytes are not a readable `.xlsx` (or the handle is dead); a
/// failed open leaves the current document untouched.
///
/// # Safety
/// `ptr` must be a live session handle; `data` a valid `byte[]` or null.
#[no_mangle]
pub unsafe extern "C" fn Java_com_example_visicalc_Engine_nativeLoadXlsx(
    env: *mut JNIEnv,
    _class: jclass,
    ptr: jlong,
    data: jbyteArray,
) -> jboolean {
    let session = session!(ptr, env, 0);
    session.load_xlsx_bytes(&jni_get_byte_array(env, data)) as jboolean
}

/// Save the current document to `.xlsx` bytes (a fresh `byte[]`).
///
/// # Safety
/// `ptr` must be a live session handle.
#[no_mangle]
pub unsafe extern "C" fn Java_com_example_visicalc_Engine_nativeSaveXlsx(
    env: *mut JNIEnv,
    _class: jclass,
    ptr: jlong,
) -> jbyteArray {
    let session = session!(ptr, env, jni_new_byte_array_from(env, &[]));
    jni_new_byte_array_from(env, &session.save_xlsx_bytes())
}

/// Open legacy `.xls` (BIFF8) file bytes. Returns `true`/`false` like
/// [`nativeLoadXlsx`](Java_com_example_visicalc_Engine_nativeLoadXlsx).
///
/// # Safety
/// `ptr` must be a live session handle; `data` a valid `byte[]` or null.
#[no_mangle]
pub unsafe extern "C" fn Java_com_example_visicalc_Engine_nativeLoadXls(
    env: *mut JNIEnv,
    _class: jclass,
    ptr: jlong,
    data: jbyteArray,
) -> jboolean {
    let session = session!(ptr, env, 0);
    session.load_xls_bytes(&jni_get_byte_array(env, data)) as jboolean
}

/// Save the current document to legacy `.xls` bytes.
///
/// # Safety
/// `ptr` must be a live session handle.
#[no_mangle]
pub unsafe extern "C" fn Java_com_example_visicalc_Engine_nativeSaveXls(
    env: *mut JNIEnv,
    _class: jclass,
    ptr: jlong,
) -> jbyteArray {
    let session = session!(ptr, env, jni_new_byte_array_from(env, &[]));
    jni_new_byte_array_from(env, &session.save_xls_bytes())
}

/// Open `.csv` file bytes. Returns `true`/`false` (`false` on invalid UTF-8).
///
/// # Safety
/// `ptr` must be a live session handle; `data` a valid `byte[]` or null.
#[no_mangle]
pub unsafe extern "C" fn Java_com_example_visicalc_Engine_nativeLoadCsv(
    env: *mut JNIEnv,
    _class: jclass,
    ptr: jlong,
    data: jbyteArray,
) -> jboolean {
    let session = session!(ptr, env, 0);
    session.load_csv_bytes(&jni_get_byte_array(env, data)) as jboolean
}

/// Save the current document's first sheet to `.csv` bytes.
///
/// # Safety
/// `ptr` must be a live session handle.
#[no_mangle]
pub unsafe extern "C" fn Java_com_example_visicalc_Engine_nativeSaveCsv(
    env: *mut JNIEnv,
    _class: jclass,
    ptr: jlong,
) -> jbyteArray {
    let session = session!(ptr, env, jni_new_byte_array_from(env, &[]));
    jni_new_byte_array_from(env, &session.save_csv_bytes())
}

/// Open `.tsv` file bytes. Returns `true`/`false` like `nativeLoadCsv`.
///
/// # Safety
/// `ptr` must be a live session handle; `data` a valid `byte[]` or null.
#[no_mangle]
pub unsafe extern "C" fn Java_com_example_visicalc_Engine_nativeLoadTsv(
    env: *mut JNIEnv,
    _class: jclass,
    ptr: jlong,
    data: jbyteArray,
) -> jboolean {
    let session = session!(ptr, env, 0);
    session.load_tsv_bytes(&jni_get_byte_array(env, data)) as jboolean
}

/// Save the current document's first sheet to `.tsv` bytes.
///
/// # Safety
/// `ptr` must be a live session handle.
#[no_mangle]
pub unsafe extern "C" fn Java_com_example_visicalc_Engine_nativeSaveTsv(
    env: *mut JNIEnv,
    _class: jclass,
    ptr: jlong,
) -> jbyteArray {
    let session = session!(ptr, env, jni_new_byte_array_from(env, &[]));
    jni_new_byte_array_from(env, &session.save_tsv_bytes())
}

/// Open `.json` file bytes (a top-level array of record objects → a table).
/// Returns `true`/`false` (`false` on malformed JSON — parsing is panic-free).
///
/// # Safety
/// `ptr` must be a live session handle; `data` a valid `byte[]` or null.
#[no_mangle]
pub unsafe extern "C" fn Java_com_example_visicalc_Engine_nativeLoadJson(
    env: *mut JNIEnv,
    _class: jclass,
    ptr: jlong,
    data: jbyteArray,
) -> jboolean {
    let session = session!(ptr, env, 0);
    session.load_json_bytes(&jni_get_byte_array(env, data)) as jboolean
}

/// Save the current document's first sheet to `.json` bytes (array of records).
///
/// # Safety
/// `ptr` must be a live session handle.
#[no_mangle]
pub unsafe extern "C" fn Java_com_example_visicalc_Engine_nativeSaveJson(
    env: *mut JNIEnv,
    _class: jclass,
    ptr: jlong,
) -> jbyteArray {
    let session = session!(ptr, env, jni_new_byte_array_from(env, &[]));
    jni_new_byte_array_from(env, &session.save_json_bytes())
}
