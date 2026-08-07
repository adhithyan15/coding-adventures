// JNI type names follow the C convention (lowercase with j prefix) and must
// not be renamed to CamelCase.
#![allow(non_camel_case_types)]

//! # jni-bridge — Zero-dependency Rust wrapper for the Java Native Interface
//!
//! Provides JNI types and helper functions for writing Rust-based JNI native
//! libraries without any external dependencies.  No `jni-sys`, no `jni`
//! crate, no bindgen, no JDK headers at build time.
//!
//! ## How JNI works
//!
//! When a Java class calls `System.loadLibrary("my_lib")`, the JVM loads the
//! shared library (`libmy_lib.so` / `my_lib.dll`).  For each method declared
//! `native` in Java, the JVM looks for an exported symbol named:
//!
//! ```text
//! Java_<package_underscores>_<ClassName>_<methodName>
//! ```
//!
//! For example, `public static native double thermalVoltage(double t)` in
//! class `com.codingadventures.silicon.SiliconSim` maps to:
//!
//! ```text
//! Java_com_codingadventures_silicon_SiliconSim_thermalVoltage
//! ```
//!
//! Every JNI native function receives a `*mut JNIEnv` as its first argument.
//! `JNIEnv` is a pointer to a function-pointer table defined by the JVM.  All
//! JNI operations (string conversion, object creation, exception throwing) are
//! performed by calling through that table.
//!
//! ## Why offset-based dispatch instead of the full struct?
//!
//! The JNI function table (`JNINativeInterface_`) has 232 slots.  Defining
//! the full struct requires every slot to be in exactly the right position.
//! Instead, we read function pointers at fixed offsets specified by the JNI
//! specification — this is identical to what a C program would do with
//! `(*env)->FindClass(env, name)`, which is macro sugar for
//! `(*(*env)->FindClass)(env, name)` (i.e. calling the function pointer at
//! the `FindClass` slot in the table).
//!
//! Offsets are defined as constants (e.g. `FIND_CLASS_OFFSET = 6`) and
//! documented with the JNI spec section they come from.
//!
//! ## Safety
//!
//! All helper functions in this crate are `unsafe`.  JNI inherently requires
//! passing raw pointers across an FFI boundary.  Callers must ensure:
//! - `env` is a valid, non-null `JNIEnv` pointer supplied by the JVM
//! - `jstring` arguments are valid references from the JVM
//! - `jclass` / `jmethodID` arguments are non-null before dereferencing
//!
//! ## Example
//!
//! ```rust,no_run
//! use jni_bridge::*;
//!
//! #[unsafe(no_mangle)]
//! pub unsafe extern "C" fn Java_com_example_Foo_hello(
//!     env: *mut JNIEnv,
//!     _class: jclass,
//!     name: jstring,
//! ) -> jstring {
//!     let name_str = jni_get_string_utf(env, name).unwrap_or_default();
//!     let greeting = format!("Hello, {}!", name_str);
//!     jni_new_string_utf(env, &greeting)
//! }
//! ```

// C `char` is signed (i8) on x86_64/aarch64 desktop but UNSIGNED (u8) on
// Android/aarch64, so JNI string-pointer types must use `c_char`, not a hardcoded
// `i8`, to cross-compile for Android. `CString::as_ptr()` already returns `*const
// c_char`, so this is a no-op on desktop and a fix on Android.
use core::ffi::c_char;

use std::ffi::{CStr, CString, c_void};
use std::ptr::null_mut;

// ─────────────────────────────────────────────────────────────────────────────
// Primitive JNI types (JNI spec §3.1)
// ─────────────────────────────────────────────────────────────────────────────
//
// These match the C typedefs in <jni.h> exactly.  They are the same on all
// platforms that the JVM runs on (32-bit and 64-bit).

/// `jboolean` — Java `boolean`, unsigned 8-bit.  JNI_TRUE = 1, JNI_FALSE = 0.
pub type jboolean = u8;
/// `jbyte` — Java `byte`, signed 8-bit.
pub type jbyte    = i8;
/// `jchar` — Java `char`, unsigned 16-bit UTF-16 code unit.
pub type jchar    = u16;
/// `jshort` — Java `short`, signed 16-bit.
pub type jshort   = i16;
/// `jint` — Java `int`, signed 32-bit.
pub type jint     = i32;
/// `jlong` — Java `long`, signed 64-bit.
pub type jlong    = i64;
/// `jfloat` — Java `float`, IEEE 754 32-bit.
pub type jfloat   = f32;
/// `jdouble` — Java `double`, IEEE 754 64-bit.
pub type jdouble  = f64;
/// `jsize` — same as `jint`; used for array lengths and string lengths.
pub type jsize    = jint;

// ─────────────────────────────────────────────────────────────────────────────
// Reference types (JNI spec §3.1)
// ─────────────────────────────────────────────────────────────────────────────
//
// All object references are opaque pointers in JNI.  The underlying GC may
// move objects, but the JNI reference remains stable until explicitly deleted.

/// Opaque JNI object reference (root of the reference hierarchy).
pub type jobject   = *mut c_void;
/// JNI class reference (`jclass` is a subtype of `jobject`).
pub type jclass    = jobject;
/// JNI throwable reference.
pub type jthrowable = jobject;
/// JNI string reference.
pub type jstring   = jobject;
/// JNI array reference.
pub type jarray    = jobject;
/// JNI `byte[]` reference (a `jarray` whose elements are `jbyte`).
pub type jbyteArray = jarray;
/// JNI method ID — identifies a Java method; not a GC root.
pub type jmethodID = *mut c_void;
/// JNI field ID — identifies a Java field; not a GC root.
pub type jfieldID  = *mut c_void;

// ─────────────────────────────────────────────────────────────────────────────
// jvalue — the polymorphic argument union (JNI spec §3.2)
// ─────────────────────────────────────────────────────────────────────────────
//
// `NewObjectA` (and the other `*A` variants) take a `*const jvalue` array
// instead of varargs.  Each element of the array holds one argument, and the
// active union arm is determined by the method signature.
//
// The union is 8 bytes wide on both 32-bit and 64-bit platforms (sized to
// hold jlong / jdouble / jobject, whichever is largest).

/// Polymorphic value union for JNI `*A` method calls.
#[repr(C)]
pub union jvalue {
    pub z: jboolean,
    pub b: jbyte,
    pub c: jchar,
    pub s: jshort,
    pub i: jint,
    pub j: jlong,
    pub f: jfloat,
    pub d: jdouble,
    pub l: jobject,
}

// ─────────────────────────────────────────────────────────────────────────────
// JNIEnv — the dispatch table pointer (JNI spec §4.4)
// ─────────────────────────────────────────────────────────────────────────────
//
// In C, `JNIEnv` is defined as `typedef const struct JNINativeInterface_ *JNIEnv`.
// Native functions receive `JNIEnv *env` — a *pointer to that pointer*.
//
// In our offset-based representation:
//
//   env:  *mut JNIEnv          = *mut *const *const c_void
//   *env: *const *const c_void  = pointer to the function table
//   (*env).add(N):              = pointer to the Nth function pointer slot
//   *(*env).add(N):             = the Nth function pointer (as *const c_void)
//
// We transmute each slot to the appropriate function type before calling.
// This is safe as long as the offsets match the JNI specification (which is
// a contract all JVM implementations must honour).

/// The JNI environment pointer type.
///
/// Native functions receive `env: *mut JNIEnv` as their first argument.
/// Internally this is a pointer-to-pointer-to-function-table.
pub type JNIEnv = *const *const c_void;

/// The Java VM pointer type.
///
/// Unlike `JNIEnv` (which is thread-local and points at the
/// `JNINativeInterface_` table), `JavaVM` is process-global, valid from
/// any thread, and points at a *different* table — `JNIInvokeInterface_`.
/// You obtain it once with `jni_get_java_vm(env)` on a JVM thread, then use
/// it from Rust-spawned threads to `AttachCurrentThreadAsDaemon` and get a
/// thread-local `JNIEnv` for that thread.
pub type JavaVM = *const *const c_void;

// ─────────────────────────────────────────────────────────────────────────────
// JNI function table offsets (JNI spec §Table 4-1)
// ─────────────────────────────────────────────────────────────────────────────
//
// The `JNINativeInterface_` struct has 232 pointer-sized slots.  These
// constants are the 0-based indices of the functions we use.  They match
// the JNI 21 specification and are identical across all compliant JVMs
// (HotSpot, OpenJ9, GraalVM, Android ART, Dalvik, etc.).

const FIND_CLASS_OFFSET:              usize = 6;   // jclass FindClass(env, name)
const THROW_NEW_OFFSET:               usize = 14;  // jint ThrowNew(env, cls, msg)
const EXCEPTION_OCCURRED_OFFSET:      usize = 15;  // jthrowable ExceptionOccurred(env)
const EXCEPTION_CLEAR_OFFSET:         usize = 17;  // void ExceptionClear(env)
const PUSH_LOCAL_FRAME_OFFSET:        usize = 19;  // jint PushLocalFrame(env, capacity)
const POP_LOCAL_FRAME_OFFSET:         usize = 20;  // jobject PopLocalFrame(env, result)
const NEW_GLOBAL_REF_OFFSET:          usize = 21;  // jobject NewGlobalRef(env, obj)
const DELETE_GLOBAL_REF_OFFSET:       usize = 22;  // void DeleteGlobalRef(env, obj)
const NEW_OBJECT_A_OFFSET:            usize = 30;  // jobject NewObjectA(env, cls, ctor, args)
const GET_OBJECT_CLASS_OFFSET:        usize = 31;  // jclass GetObjectClass(env, obj)
const IS_INSTANCE_OF_OFFSET:          usize = 32;  // jboolean IsInstanceOf(env, obj, cls)
const GET_METHOD_ID_OFFSET:           usize = 33;  // jmethodID GetMethodID(env, cls, name, sig)
const CALL_OBJECT_METHOD_A_OFFSET:    usize = 36;  // jobject CallObjectMethodA(env, obj, mid, args)
const CALL_INT_METHOD_A_OFFSET:       usize = 51;  // jint CallIntMethodA(env, obj, mid, args)
const CALL_VOID_METHOD_A_OFFSET:      usize = 63;  // void CallVoidMethodA(env, obj, mid, args)
const GET_FIELD_ID_OFFSET:            usize = 94;  // jfieldID GetFieldID(env, cls, name, sig)
const SET_OBJECT_FIELD_OFFSET:        usize = 104; // void SetObjectField(env, obj, fid, val)
const SET_DOUBLE_FIELD_OFFSET:        usize = 112; // void SetDoubleField(env, obj, fid, val)
const NEW_STRING_UTF_OFFSET:          usize = 167; // jstring NewStringUTF(env, utf8)
const GET_STRING_UTF_CHARS_OFFSET:    usize = 169; // const char* GetStringUTFChars(env, str, isCopy)
const RELEASE_STRING_UTF_CHARS_OFFSET:usize = 170; // void ReleaseStringUTFChars(env, str, chars)
const GET_ARRAY_LENGTH_OFFSET:        usize = 171; // jsize GetArrayLength(env, array)
const NEW_BYTE_ARRAY_OFFSET:          usize = 176; // jbyteArray NewByteArray(env, len)
const NEW_DOUBLE_ARRAY_OFFSET:        usize = 182; // jarray NewDoubleArray(env, len)
const GET_BYTE_ARRAY_REGION_OFFSET:   usize = 200; // void GetByteArrayRegion(env, arr, start, len, buf)
const SET_BYTE_ARRAY_REGION_OFFSET:   usize = 208; // void SetByteArrayRegion(env, arr, start, len, buf)
const SET_DOUBLE_ARRAY_REGION_OFFSET: usize = 214; // void SetDoubleArrayRegion(env, arr, start, len, buf)
const GET_JAVA_VM_OFFSET:             usize = 219; // jint GetJavaVM(env, JavaVM**)
const EXCEPTION_CHECK_OFFSET:         usize = 228; // jboolean ExceptionCheck(env)

// ─────────────────────────────────────────────────────────────────────────────
// JavaVM invocation-interface offsets (JNI spec §Table 4-2)
// ─────────────────────────────────────────────────────────────────────────────
//
// The JavaVM points at the `JNIInvokeInterface_` table, NOT the JNIEnv table.
// It has only a handful of slots; these are the ones we need for managing
// the attachment of Rust-spawned threads to the JVM.

const VM_ATTACH_CURRENT_THREAD_OFFSET:           usize = 4; // jint AttachCurrentThread(vm, void** env, void* args)
const VM_DETACH_CURRENT_THREAD_OFFSET:           usize = 5; // jint DetachCurrentThread(vm)
const VM_ATTACH_CURRENT_THREAD_AS_DAEMON_OFFSET: usize = 7; // jint AttachCurrentThreadAsDaemon(vm, void** env, void* args)

// ─────────────────────────────────────────────────────────────────────────────
// Internal: read and call a function pointer from the JNI table
// ─────────────────────────────────────────────────────────────────────────────

/// Read the function pointer at `offset` from `*env` and transmute it to `F`.
///
/// # Safety
/// - `env` must be a valid non-null JNIEnv supplied by the JVM
/// - `offset` must be a valid slot index in the JNI function table
/// - `F` must exactly match the actual function type at that slot
#[inline(always)]
unsafe fn table_fn<F: Copy>(env: *mut JNIEnv, offset: usize) -> F {
    // Guard against null env or null function table (defensive; JVM guarantees
    // these are valid, but instrumented environments may violate that).
    debug_assert!(!env.is_null(), "JNIEnv pointer must not be null");
    debug_assert!(!(*env).is_null(), "JNI function table must not be null");
    // *env  → *const *const c_void  (the function table)
    // .add(offset) → pointer to the slot
    // *…  → *const c_void  (the function pointer, as an opaque void*)
    let fn_ptr = *(*env).add(offset);
    std::mem::transmute_copy::<*const c_void, F>(&fn_ptr)
}

// ─────────────────────────────────────────────────────────────────────────────
// Public helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Find a Java class by its internal name (e.g. `"com/pkg/ClassName"`).
///
/// Returns null and leaves a `ClassNotFoundException` pending if the class
/// cannot be found.  Callers should check for null before using the result.
///
/// # Safety
/// `env` must be a valid JNIEnv.
pub unsafe fn jni_find_class(env: *mut JNIEnv, name: &str) -> jclass {
    // CString::new can only fail if name contains an interior NUL — class
    // names never do, but we fall back to null rather than panic.
    let cs = match CString::new(name) {
        Ok(c) => c,
        Err(_) => return null_mut(),
    };
    type F = unsafe extern "C" fn(*mut JNIEnv, *const c_char) -> jclass;
    let f: F = table_fn(env, FIND_CLASS_OFFSET);
    f(env, cs.as_ptr())
}

/// Throw a new exception of `class_name` with `msg`.
///
/// After this call a pending exception is recorded in the JVM.  The native
/// function must still return a value (null / 0.0); the JVM propagates the
/// exception once control returns to Java.
///
/// `class_name` uses the internal JNI slash format (e.g.
/// `"com/codingadventures/silicon/SiliconException"`).
///
/// # Safety
/// `env` must be a valid JNIEnv.
pub unsafe fn jni_throw_new(env: *mut JNIEnv, class_name: &str, msg: &str) {
    // JNI spec §2.6: most JNI calls are illegal while an exception is pending.
    // FindClass is not in the safe set, so clear any pre-existing exception
    // before looking up the exception class we want to throw.
    jni_exception_clear(env);
    let cls = jni_find_class(env, class_name);
    if cls.is_null() {
        // ClassNotFoundException is already pending; can't throw our exception.
        return;
    }
    let cs = match CString::new(msg) {
        Ok(c) => c,
        Err(_) => CString::new("<message contained NUL byte>").unwrap(),
    };
    type F = unsafe extern "C" fn(*mut JNIEnv, jclass, *const c_char) -> jint;
    let f: F = table_fn(env, THROW_NEW_OFFSET);
    f(env, cls, cs.as_ptr());
}

/// Clear any pending exception in the JVM.
///
/// # Safety
/// `env` must be a valid JNIEnv.
pub unsafe fn jni_exception_clear(env: *mut JNIEnv) {
    type F = unsafe extern "C" fn(*mut JNIEnv);
    let f: F = table_fn(env, EXCEPTION_CLEAR_OFFSET);
    f(env);
}

/// Check whether a pending exception exists.
///
/// # Safety
/// `env` must be a valid JNIEnv.
pub unsafe fn jni_exception_check(env: *mut JNIEnv) -> bool {
    type F = unsafe extern "C" fn(*mut JNIEnv) -> jboolean;
    let f: F = table_fn(env, EXCEPTION_CHECK_OFFSET);
    f(env) != 0
}

/// Get the UTF-8 content of a Java string.
///
/// Returns `None` if `s` is null or if `GetStringUTFChars` returns null
/// (out of memory).  Always calls `ReleaseStringUTFChars` to avoid leaking
/// the pinned buffer.
///
/// JNI strings use "Modified UTF-8" (MUTF-8).  For the characters we use
/// (ASCII material names, numbers), MUTF-8 == standard UTF-8.
///
/// # Safety
/// `env` must be a valid JNIEnv.  `s` must be a valid JNI local/global ref
/// or null.
pub unsafe fn jni_get_string_utf(env: *mut JNIEnv, s: jstring) -> Option<String> {
    if s.is_null() {
        return None;
    }
    type GetF = unsafe extern "C" fn(*mut JNIEnv, jstring, *mut jboolean) -> *const c_char;
    type RelF = unsafe extern "C" fn(*mut JNIEnv, jstring, *const c_char);
    let get_chars: GetF = table_fn(env, GET_STRING_UTF_CHARS_OFFSET);
    let release:   RelF = table_fn(env, RELEASE_STRING_UTF_CHARS_OFFSET);

    let chars = get_chars(env, s, null_mut());
    if chars.is_null() {
        return None;
    }
    // SAFETY: `chars` is valid UTF-8 (MUTF-8 is a superset of ASCII; our
    // use cases are purely ASCII).  We convert before releasing.
    let result = CStr::from_ptr(chars).to_string_lossy().into_owned();
    release(env, s, chars);
    Some(result)
}

/// Create a new Java string from a Rust `&str`.
///
/// Returns null if `NewStringUTF` fails (out of memory) or if `s` contains
/// an interior NUL byte.
///
/// # Safety
/// `env` must be a valid JNIEnv.
pub unsafe fn jni_new_string_utf(env: *mut JNIEnv, s: &str) -> jstring {
    let cs = match CString::new(s) {
        Ok(c) => c,
        Err(_) => return null_mut(),
    };
    type F = unsafe extern "C" fn(*mut JNIEnv, *const c_char) -> jstring;
    let f: F = table_fn(env, NEW_STRING_UTF_OFFSET);
    f(env, cs.as_ptr())
}

/// Look up a Java method ID by class, name, and JNI signature string.
///
/// Returns null if the method cannot be found (a `NoSuchMethodError` is
/// pending).
///
/// Signature format: `"(arg_types)return_type"` where types are:
///   `Z`=boolean, `B`=byte, `C`=char, `S`=short, `I`=int, `J`=long,
///   `F`=float, `D`=double, `V`=void,
///   `Lclass/name;`=object, `[T`=array of T.
///
/// # Safety
/// `env` and `cls` must be valid non-null JNI values.
pub unsafe fn jni_get_method_id(
    env: *mut JNIEnv,
    cls: jclass,
    name: &str,
    sig:  &str,
) -> jmethodID {
    let cn = match CString::new(name) { Ok(c) => c, Err(_) => return null_mut() };
    let cs = match CString::new(sig)  { Ok(c) => c, Err(_) => return null_mut() };
    type F = unsafe extern "C" fn(*mut JNIEnv, jclass, *const c_char, *const c_char) -> jmethodID;
    let f: F = table_fn(env, GET_METHOD_ID_OFFSET);
    f(env, cls, cn.as_ptr(), cs.as_ptr())
}

/// Look up a Java instance field ID by class, name, and type descriptor.
///
/// Returns null if not found (a `NoSuchFieldError` is pending).
///
/// # Safety
/// `env` and `cls` must be valid non-null JNI values.
pub unsafe fn jni_get_field_id(
    env: *mut JNIEnv,
    cls: jclass,
    name: &str,
    sig:  &str,
) -> jfieldID {
    let cn = match CString::new(name) { Ok(c) => c, Err(_) => return null_mut() };
    let cs = match CString::new(sig)  { Ok(c) => c, Err(_) => return null_mut() };
    type F = unsafe extern "C" fn(*mut JNIEnv, jclass, *const c_char, *const c_char) -> jfieldID;
    let f: F = table_fn(env, GET_FIELD_ID_OFFSET);
    f(env, cls, cn.as_ptr(), cs.as_ptr())
}

/// Create a new Java object using the `*A` (jvalue-array) constructor call.
///
/// `cls` — the class to instantiate
/// `ctor` — method ID of the constructor (obtained via `GetMethodID` with
///   name `"<init>"`)
/// `args` — pointer to a `jvalue` array with one element per constructor arg
///
/// Returns null and leaves an exception pending on failure.
///
/// # Safety
/// `env`, `cls`, and `ctor` must be valid non-null JNI values.
/// `args` must point to at least as many `jvalue` elements as the constructor
/// expects.
pub unsafe fn jni_new_object_a(
    env:  *mut JNIEnv,
    cls:  jclass,
    ctor: jmethodID,
    args: *const jvalue,
) -> jobject {
    type F = unsafe extern "C" fn(*mut JNIEnv, jclass, jmethodID, *const jvalue) -> jobject;
    let f: F = table_fn(env, NEW_OBJECT_A_OFFSET);
    f(env, cls, ctor, args)
}

/// Set a `double` instance field on `obj`.
///
/// # Safety
/// `env`, `obj`, and `fid` must be valid non-null JNI values.
pub unsafe fn jni_set_double_field(
    env: *mut JNIEnv,
    obj: jobject,
    fid: jfieldID,
    val: jdouble,
) {
    type F = unsafe extern "C" fn(*mut JNIEnv, jobject, jfieldID, jdouble);
    let f: F = table_fn(env, SET_DOUBLE_FIELD_OFFSET);
    f(env, obj, fid, val);
}

/// Set an object instance field on `obj`.
///
/// # Safety
/// `env`, `obj`, `fid`, and `val` must be valid JNI values.
pub unsafe fn jni_set_object_field(
    env: *mut JNIEnv,
    obj: jobject,
    fid: jfieldID,
    val: jobject,
) {
    type F = unsafe extern "C" fn(*mut JNIEnv, jobject, jfieldID, jobject);
    let f: F = table_fn(env, SET_OBJECT_FIELD_OFFSET);
    f(env, obj, fid, val);
}

/// Create a new Java `double[]` of length `len`.
///
/// Returns null if allocation fails (an `OutOfMemoryError` is pending).
///
/// # Safety
/// `env` must be a valid JNIEnv.
pub unsafe fn jni_new_double_array(env: *mut JNIEnv, len: jsize) -> jarray {
    type F = unsafe extern "C" fn(*mut JNIEnv, jsize) -> jarray;
    let f: F = table_fn(env, NEW_DOUBLE_ARRAY_OFFSET);
    f(env, len)
}

/// Copy `len` `f64` values from `buf` into a Java `double[]` starting at
/// `start`.
///
/// # Safety
/// `env` must be a valid JNIEnv.  `arr` must be a valid `double[]`.
/// `buf` must point to at least `len` valid `jdouble` values.
pub unsafe fn jni_set_double_array_region(
    env:   *mut JNIEnv,
    arr:   jarray,
    start: jsize,
    len:   jsize,
    buf:   *const jdouble,
) {
    type F = unsafe extern "C" fn(*mut JNIEnv, jarray, jsize, jsize, *const jdouble);
    let f: F = table_fn(env, SET_DOUBLE_ARRAY_REGION_OFFSET);
    f(env, arr, start, len, buf);
}

// ─────────────────────────────────────────────────────────────────────────────
// Byte arrays (JNI spec §Array Operations) — for passing file bytes to/from Java
// ─────────────────────────────────────────────────────────────────────────────
//
// A Java `byte[]` is the natural carrier for a spreadsheet file's raw bytes
// (an `.xlsx` a user opened, or a document to download). The two convenience
// wrappers below hide the raw JNI dance: `jni_get_byte_array` copies a `byte[]`
// into an owned `Vec<u8>` (length query + one region copy — `jbyte` is `i8`, the
// same bit pattern as `u8`), and `jni_new_byte_array_from` allocates a fresh
// `byte[]` and fills it from a `&[u8]`. Both are safe against a null array.

/// `GetArrayLength` — the element count of any Java array.
///
/// # Safety
/// `env` must be a valid JNIEnv; `arr` a valid array ref or null (→ 0).
pub unsafe fn jni_get_array_length(env: *mut JNIEnv, arr: jarray) -> jsize {
    if arr.is_null() {
        return 0;
    }
    type F = unsafe extern "C" fn(*mut JNIEnv, jarray) -> jsize;
    let f: F = table_fn(env, GET_ARRAY_LENGTH_OFFSET);
    f(env, arr)
}

/// `NewByteArray` — allocate an empty Java `byte[]` of `len` bytes. Null on OOM.
///
/// # Safety
/// `env` must be a valid JNIEnv.
pub unsafe fn jni_new_byte_array(env: *mut JNIEnv, len: jsize) -> jbyteArray {
    type F = unsafe extern "C" fn(*mut JNIEnv, jsize) -> jbyteArray;
    let f: F = table_fn(env, NEW_BYTE_ARRAY_OFFSET);
    f(env, len)
}

/// `GetByteArrayRegion` — copy `len` bytes from `arr` (starting at `start`) into
/// `buf`. `jbyte` is `i8`; the caller reinterprets the identical bits as `u8`.
///
/// # Safety
/// `env` valid; `arr` a valid `byte[]`; `buf` writable for `len` `jbyte`s.
pub unsafe fn jni_get_byte_array_region(
    env: *mut JNIEnv,
    arr: jbyteArray,
    start: jsize,
    len: jsize,
    buf: *mut jbyte,
) {
    type F = unsafe extern "C" fn(*mut JNIEnv, jbyteArray, jsize, jsize, *mut jbyte);
    let f: F = table_fn(env, GET_BYTE_ARRAY_REGION_OFFSET);
    f(env, arr, start, len, buf);
}

/// `SetByteArrayRegion` — copy `len` bytes from `buf` into `arr` at `start`.
///
/// # Safety
/// `env` valid; `arr` a valid `byte[]`; `buf` readable for `len` `jbyte`s.
pub unsafe fn jni_set_byte_array_region(
    env: *mut JNIEnv,
    arr: jbyteArray,
    start: jsize,
    len: jsize,
    buf: *const jbyte,
) {
    type F = unsafe extern "C" fn(*mut JNIEnv, jbyteArray, jsize, jsize, *const jbyte);
    let f: F = table_fn(env, SET_BYTE_ARRAY_REGION_OFFSET);
    f(env, arr, start, len, buf);
}

/// Copy a Java `byte[]` into an owned `Vec<u8>`. A null array yields an empty
/// vec. `jbyte` (`i8`) and `u8` share a bit pattern, so the region copies
/// straight into a `u8` buffer.
///
/// # Safety
/// `env` must be a valid JNIEnv; `arr` a valid `byte[]` ref or null.
pub unsafe fn jni_get_byte_array(env: *mut JNIEnv, arr: jbyteArray) -> Vec<u8> {
    let len = jni_get_array_length(env, arr);
    if arr.is_null() || len <= 0 {
        return Vec::new();
    }
    let mut buf = vec![0u8; len as usize];
    jni_get_byte_array_region(env, arr, 0, len, buf.as_mut_ptr() as *mut jbyte);
    buf
}

/// Allocate a fresh Java `byte[]` and fill it from a `&[u8]`. Returns null if
/// the allocation fails (an `OutOfMemoryError` is then pending).
///
/// # Safety
/// `env` must be a valid JNIEnv.
pub unsafe fn jni_new_byte_array_from(env: *mut JNIEnv, bytes: &[u8]) -> jbyteArray {
    let len = bytes.len() as jsize;
    let arr = jni_new_byte_array(env, len);
    if !arr.is_null() && len > 0 {
        jni_set_byte_array_region(env, arr, 0, len, bytes.as_ptr() as *const jbyte);
    }
    arr
}

// ─────────────────────────────────────────────────────────────────────────────
// Local reference frames (JNI spec §5.1.2)
// ─────────────────────────────────────────────────────────────────────────────
//
// Every JNI call that returns an object (FindClass, NewObjectA, NewStringUTF,
// CallObjectMethod, …) produces a *local* reference.  When a native method is
// invoked from Java, all its local refs are freed automatically once it
// returns.  But on a thread that Rust attached itself (and that runs a loop
// without ever returning to Java) local refs accumulate forever — a leak.
//
// `PushLocalFrame` / `PopLocalFrame` bracket a scope: every local ref created
// after the push is freed by the matching pop.  Wrap each request dispatch in
// a frame and copy out any data you need (into owned Rust values) before
// popping.

/// Create a new local-reference frame with room for at least `capacity`
/// local references.  Returns 0 on success, negative on failure (an
/// `OutOfMemoryError` is pending).
///
/// # Safety
/// `env` must be a valid JNIEnv.  Must be balanced by `jni_pop_local_frame`.
pub unsafe fn jni_push_local_frame(env: *mut JNIEnv, capacity: jint) -> jint {
    type F = unsafe extern "C" fn(*mut JNIEnv, jint) -> jint;
    let f: F = table_fn(env, PUSH_LOCAL_FRAME_OFFSET);
    f(env, capacity)
}

/// Pop the current local-reference frame, freeing every local ref created
/// since the matching `jni_push_local_frame`.
///
/// Pass null for `result` to discard all refs.  (To keep one ref alive past
/// the pop, pass it as `result`; the return value is a fresh local ref to the
/// same object in the enclosing frame — we don't use that form here.)
///
/// # Safety
/// `env` must be a valid JNIEnv with a frame previously pushed.
pub unsafe fn jni_pop_local_frame(env: *mut JNIEnv) {
    type F = unsafe extern "C" fn(*mut JNIEnv, jobject) -> jobject;
    let f: F = table_fn(env, POP_LOCAL_FRAME_OFFSET);
    f(env, null_mut());
}

// ─────────────────────────────────────────────────────────────────────────────
// Global references (JNI spec §5.1.1)
// ─────────────────────────────────────────────────────────────────────────────
//
// Local references (the kind returned by FindClass, NewObjectA, method calls,
// etc.) are only valid until the native function returns.  To keep a Java
// object alive and callable across multiple native calls — or, crucially,
// across *threads* — you must promote it to a global reference.  Global refs
// stay valid until explicitly deleted with DeleteGlobalRef and may be used
// from any thread.

/// Promote a local reference to a global reference.
///
/// The returned reference stays valid (and keeps the Java object from being
/// GC'd) until passed to `jni_delete_global_ref`.  Returns null if `obj` is
/// null or the JVM is out of memory.
///
/// # Safety
/// `env` must be a valid JNIEnv.  `obj` must be a valid local/global ref or
/// null.
pub unsafe fn jni_new_global_ref(env: *mut JNIEnv, obj: jobject) -> jobject {
    type F = unsafe extern "C" fn(*mut JNIEnv, jobject) -> jobject;
    let f: F = table_fn(env, NEW_GLOBAL_REF_OFFSET);
    f(env, obj)
}

/// Delete a global reference created with `jni_new_global_ref`.
///
/// After this call the underlying Java object becomes eligible for GC (unless
/// other references keep it alive).  Deleting null is a documented no-op.
///
/// # Safety
/// `env` must be a valid JNIEnv.  `obj` must be a global ref previously
/// returned by `jni_new_global_ref`, or null.
pub unsafe fn jni_delete_global_ref(env: *mut JNIEnv, obj: jobject) {
    if obj.is_null() {
        return;
    }
    type F = unsafe extern "C" fn(*mut JNIEnv, jobject);
    let f: F = table_fn(env, DELETE_GLOBAL_REF_OFFSET);
    f(env, obj);
}

// ─────────────────────────────────────────────────────────────────────────────
// Object inspection (JNI spec §5.3)
// ─────────────────────────────────────────────────────────────────────────────

/// Return the runtime class of `obj` (like Java's `obj.getClass()`).
///
/// The returned `jclass` is a local reference.
///
/// # Safety
/// `env` and `obj` must be valid non-null JNI values.
pub unsafe fn jni_get_object_class(env: *mut JNIEnv, obj: jobject) -> jclass {
    type F = unsafe extern "C" fn(*mut JNIEnv, jobject) -> jclass;
    let f: F = table_fn(env, GET_OBJECT_CLASS_OFFSET);
    f(env, obj)
}

/// Test whether `obj` is an instance of `cls` (like Java's `instanceof`).
///
/// Returns `false` if `obj` is null (consistent with `null instanceof X`).
///
/// # Safety
/// `env` and `cls` must be valid; `obj` may be null.
pub unsafe fn jni_is_instance_of(env: *mut JNIEnv, obj: jobject, cls: jclass) -> bool {
    type F = unsafe extern "C" fn(*mut JNIEnv, jobject, jclass) -> jboolean;
    let f: F = table_fn(env, IS_INSTANCE_OF_OFFSET);
    f(env, obj, cls) != 0
}

/// Return the pending exception (if any) without clearing it.
///
/// Returns null if no exception is pending.  The returned `jthrowable` is a
/// local reference.  Most JNI calls are illegal while an exception is
/// pending, so the usual sequence is: `jni_exception_occurred` →
/// `jni_exception_clear` → inspect the throwable.
///
/// # Safety
/// `env` must be a valid JNIEnv.
pub unsafe fn jni_exception_occurred(env: *mut JNIEnv) -> jthrowable {
    type F = unsafe extern "C" fn(*mut JNIEnv) -> jthrowable;
    let f: F = table_fn(env, EXCEPTION_OCCURRED_OFFSET);
    f(env)
}

// ─────────────────────────────────────────────────────────────────────────────
// Instance method calls — the `*A` (jvalue-array) variants (JNI spec §4.2)
// ─────────────────────────────────────────────────────────────────────────────
//
// As with NewObjectA, we use the `*A` forms that take a `*const jvalue` array
// rather than C varargs (which Rust cannot express portably).  Pass a null
// `args` pointer for zero-argument methods.

/// Call a Java method returning an object (`CallObjectMethodA`).
///
/// `obj`  — the receiver (instance the method is called on)
/// `mid`  — method ID from `jni_get_method_id`
/// `args` — `jvalue` array (or null for no args)
///
/// Returns the result (a local reference, possibly null).  If the method
/// throws, a Java exception is left pending — check with
/// `jni_exception_check` afterwards.
///
/// # Safety
/// `env`, `obj`, and `mid` must be valid non-null JNI values.  `args` must
/// point to enough `jvalue`s for the method's arity.
pub unsafe fn jni_call_object_method_a(
    env: *mut JNIEnv,
    obj: jobject,
    mid: jmethodID,
    args: *const jvalue,
) -> jobject {
    type F = unsafe extern "C" fn(*mut JNIEnv, jobject, jmethodID, *const jvalue) -> jobject;
    let f: F = table_fn(env, CALL_OBJECT_METHOD_A_OFFSET);
    f(env, obj, mid, args)
}

/// Call a Java method returning an `int` (`CallIntMethodA`).
///
/// # Safety
/// `env`, `obj`, and `mid` must be valid non-null JNI values.
pub unsafe fn jni_call_int_method_a(
    env: *mut JNIEnv,
    obj: jobject,
    mid: jmethodID,
    args: *const jvalue,
) -> jint {
    type F = unsafe extern "C" fn(*mut JNIEnv, jobject, jmethodID, *const jvalue) -> jint;
    let f: F = table_fn(env, CALL_INT_METHOD_A_OFFSET);
    f(env, obj, mid, args)
}

/// Call a Java method returning `void` (`CallVoidMethodA`).
///
/// # Safety
/// `env`, `obj`, and `mid` must be valid non-null JNI values.
pub unsafe fn jni_call_void_method_a(
    env: *mut JNIEnv,
    obj: jobject,
    mid: jmethodID,
    args: *const jvalue,
) {
    type F = unsafe extern "C" fn(*mut JNIEnv, jobject, jmethodID, *const jvalue);
    let f: F = table_fn(env, CALL_VOID_METHOD_A_OFFSET);
    f(env, obj, mid, args);
}

// ─────────────────────────────────────────────────────────────────────────────
// JavaVM and thread attachment (JNI spec §5.4)
// ─────────────────────────────────────────────────────────────────────────────
//
// To call into Java from a thread the JVM did not create (e.g. a Rust I/O
// thread spawned by an event-loop runtime), that thread must first attach to
// the JVM to obtain its own thread-local `JNIEnv`.  The JavaVM pointer needed
// for attachment is process-global; capture it once on a JVM thread with
// `jni_get_java_vm`, then carry it to the background threads.

/// Read the function pointer at `offset` from the JavaVM invocation table.
///
/// The JavaVM points at `JNIInvokeInterface_`, a different (and much smaller)
/// table than the per-thread `JNINativeInterface_`.
///
/// # Safety
/// - `vm` must be a valid non-null JavaVM pointer
/// - `offset` must be a valid slot in the invocation table
/// - `F` must exactly match the function type at that slot
#[inline(always)]
unsafe fn vm_table_fn<F: Copy>(vm: *mut JavaVM, offset: usize) -> F {
    debug_assert!(!vm.is_null(), "JavaVM pointer must not be null");
    debug_assert!(!(*vm).is_null(), "JavaVM invocation table must not be null");
    let fn_ptr = *(*vm).add(offset);
    std::mem::transmute_copy::<*const c_void, F>(&fn_ptr)
}

/// Obtain the process-global `JavaVM` pointer from a thread-local `JNIEnv`.
///
/// Call this once on a JVM thread (e.g. inside a native registration method)
/// and stash the result; it is valid for the life of the JVM and may be used
/// from any thread.  Returns null if the call fails.
///
/// # Safety
/// `env` must be a valid JNIEnv.
pub unsafe fn jni_get_java_vm(env: *mut JNIEnv) -> *mut JavaVM {
    type F = unsafe extern "C" fn(*mut JNIEnv, *mut *mut JavaVM) -> jint;
    let f: F = table_fn(env, GET_JAVA_VM_OFFSET);
    let mut vm: *mut JavaVM = null_mut();
    let rc = f(env, &mut vm);
    if rc != 0 { null_mut() } else { vm }
}

/// Attach the current OS thread to the JVM as a *daemon* thread and return a
/// thread-local `JNIEnv` for it.
///
/// Daemon attachment is preferred for long-lived worker threads because such
/// threads do not need an explicit `DetachCurrentThread` and do not block JVM
/// shutdown.  Calling this on an already-attached thread simply returns the
/// existing `JNIEnv` (idempotent and cheap).  Returns null on failure.
///
/// # Safety
/// `vm` must be a valid JavaVM pointer obtained from `jni_get_java_vm`.
pub unsafe fn jni_attach_current_thread_as_daemon(vm: *mut JavaVM) -> *mut JNIEnv {
    type F = unsafe extern "C" fn(*mut JavaVM, *mut *mut c_void, *mut c_void) -> jint;
    let f: F = vm_table_fn(vm, VM_ATTACH_CURRENT_THREAD_AS_DAEMON_OFFSET);
    let mut env: *mut c_void = null_mut();
    let rc = f(vm, &mut env, null_mut());
    if rc != 0 {
        null_mut()
    } else {
        env as *mut JNIEnv
    }
}

/// Attach the current OS thread to the JVM (non-daemon) and return its
/// thread-local `JNIEnv`.
///
/// A non-daemon attached thread MUST be detached with
/// `jni_detach_current_thread` before it exits, or the JVM may abort.  Prefer
/// `jni_attach_current_thread_as_daemon` for pooled/long-lived threads.
///
/// # Safety
/// `vm` must be a valid JavaVM pointer.
pub unsafe fn jni_attach_current_thread(vm: *mut JavaVM) -> *mut JNIEnv {
    type F = unsafe extern "C" fn(*mut JavaVM, *mut *mut c_void, *mut c_void) -> jint;
    let f: F = vm_table_fn(vm, VM_ATTACH_CURRENT_THREAD_OFFSET);
    let mut env: *mut c_void = null_mut();
    let rc = f(vm, &mut env, null_mut());
    if rc != 0 {
        null_mut()
    } else {
        env as *mut JNIEnv
    }
}

/// Detach the current OS thread from the JVM.
///
/// Required before a non-daemon attached thread exits.  No-op semantics for a
/// thread that is not attached are implementation-defined, so only call this
/// on threads you attached with `jni_attach_current_thread`.
///
/// # Safety
/// `vm` must be a valid JavaVM pointer.
pub unsafe fn jni_detach_current_thread(vm: *mut JavaVM) {
    type F = unsafe extern "C" fn(*mut JavaVM) -> jint;
    let f: F = vm_table_fn(vm, VM_DETACH_CURRENT_THREAD_OFFSET);
    f(vm);
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────
//
// A real JNIEnv comes only from a running JVM, so we verify the byte-array
// helpers against a MOCK function table: a `[*const c_void; N]` with the four
// byte-array slots (at the same offset constants the helpers use) pointing at
// Rust functions that emulate the JVM's behaviour. A `jbyteArray` is modelled as
// a `*mut Vec<u8>`. This exercises the offset dispatch AND the get/new wrappers'
// length-query + region-copy logic end to end.
#[cfg(test)]
mod byte_array_tests {
    use super::*;
    use std::os::raw::c_void;

    unsafe extern "C" fn mock_new_byte_array(_env: *mut JNIEnv, len: jsize) -> jbyteArray {
        Box::into_raw(Box::new(vec![0u8; len.max(0) as usize])) as jbyteArray
    }
    unsafe extern "C" fn mock_get_array_length(_env: *mut JNIEnv, arr: jarray) -> jsize {
        (*(arr as *mut Vec<u8>)).len() as jsize
    }
    unsafe extern "C" fn mock_get_byte_array_region(
        _env: *mut JNIEnv,
        arr: jbyteArray,
        start: jsize,
        len: jsize,
        buf: *mut jbyte,
    ) {
        let v = &*(arr as *mut Vec<u8>);
        for i in 0..len as usize {
            *buf.add(i) = v[start as usize + i] as jbyte;
        }
    }
    unsafe extern "C" fn mock_set_byte_array_region(
        _env: *mut JNIEnv,
        arr: jbyteArray,
        start: jsize,
        len: jsize,
        buf: *const jbyte,
    ) {
        let v = &mut *(arr as *mut Vec<u8>);
        for i in 0..len as usize {
            v[start as usize + i] = *buf.add(i) as u8;
        }
    }

    /// Build a mock JNIEnv: a 232-slot table with the byte-array slots filled.
    /// Returns the boxed table (which must outlive `env`) and the env pointer.
    fn mock_env() -> (Box<[*const c_void; 232]>, Box<JNIEnv>) {
        let mut table: Box<[*const c_void; 232]> = Box::new([std::ptr::null(); 232]);
        table[GET_ARRAY_LENGTH_OFFSET] = mock_get_array_length as *const c_void;
        table[NEW_BYTE_ARRAY_OFFSET] = mock_new_byte_array as *const c_void;
        table[GET_BYTE_ARRAY_REGION_OFFSET] = mock_get_byte_array_region as *const c_void;
        table[SET_BYTE_ARRAY_REGION_OFFSET] = mock_set_byte_array_region as *const c_void;
        // JNIEnv is a pointer to the table base (`*const *const c_void`).
        let env: Box<JNIEnv> = Box::new(table.as_ptr());
        (table, env)
    }

    #[test]
    fn round_trips_bytes_through_a_java_byte_array() {
        unsafe {
            let (_table, mut env) = mock_env();
            let envp: *mut JNIEnv = &mut *env;
            // Rust bytes → new byte[] → read them back: identical, including a
            // high bit (0xD0, the .xls magic) that i8/u8 reinterpretation covers.
            let src = [0xD0u8, 0xCF, 0x00, 0x7F, 0xFFu8, 42];
            let arr = jni_new_byte_array_from(envp, &src);
            assert!(!arr.is_null());
            assert_eq!(jni_get_array_length(envp, arr), src.len() as jsize);
            assert_eq!(jni_get_byte_array(envp, arr), src);
            drop(Box::from_raw(arr as *mut Vec<u8>)); // free the mock object
        }
    }

    #[test]
    fn empty_and_null_arrays_are_safe() {
        unsafe {
            let (_table, mut env) = mock_env();
            let envp: *mut JNIEnv = &mut *env;
            // Empty input → a real, zero-length array; reads back empty.
            let arr = jni_new_byte_array_from(envp, &[]);
            assert!(!arr.is_null());
            assert_eq!(jni_get_array_length(envp, arr), 0);
            assert!(jni_get_byte_array(envp, arr).is_empty());
            drop(Box::from_raw(arr as *mut Vec<u8>));
            // Null array → length 0 and an empty vec, never a deref crash.
            assert_eq!(jni_get_array_length(envp, std::ptr::null_mut()), 0);
            assert!(jni_get_byte_array(envp, std::ptr::null_mut()).is_empty());
        }
    }
}
