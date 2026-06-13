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
const EXCEPTION_CLEAR_OFFSET:         usize = 17;  // void ExceptionClear(env)
const NEW_OBJECT_A_OFFSET:            usize = 30;  // jobject NewObjectA(env, cls, ctor, args)
const GET_METHOD_ID_OFFSET:           usize = 33;  // jmethodID GetMethodID(env, cls, name, sig)
const GET_FIELD_ID_OFFSET:            usize = 94;  // jfieldID GetFieldID(env, cls, name, sig)
const SET_OBJECT_FIELD_OFFSET:        usize = 104; // void SetObjectField(env, obj, fid, val)
const SET_DOUBLE_FIELD_OFFSET:        usize = 112; // void SetDoubleField(env, obj, fid, val)
const NEW_STRING_UTF_OFFSET:          usize = 167; // jstring NewStringUTF(env, utf8)
const GET_STRING_UTF_CHARS_OFFSET:    usize = 169; // const char* GetStringUTFChars(env, str, isCopy)
const RELEASE_STRING_UTF_CHARS_OFFSET:usize = 170; // void ReleaseStringUTFChars(env, str, chars)
const NEW_DOUBLE_ARRAY_OFFSET:        usize = 182; // jarray NewDoubleArray(env, len)
const SET_DOUBLE_ARRAY_REGION_OFFSET: usize = 214; // void SetDoubleArrayRegion(env, arr, start, len, buf)
const EXCEPTION_CHECK_OFFSET:         usize = 228; // jboolean ExceptionCheck(env)

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
    type F = unsafe extern "C" fn(*mut JNIEnv, *const i8) -> jclass;
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
    let cls = jni_find_class(env, class_name);
    if cls.is_null() {
        // ClassNotFoundException is already pending; can't throw our exception.
        return;
    }
    let cs = match CString::new(msg) {
        Ok(c) => c,
        Err(_) => CString::new("<message contained NUL byte>").unwrap(),
    };
    type F = unsafe extern "C" fn(*mut JNIEnv, jclass, *const i8) -> jint;
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
    type GetF = unsafe extern "C" fn(*mut JNIEnv, jstring, *mut jboolean) -> *const i8;
    type RelF = unsafe extern "C" fn(*mut JNIEnv, jstring, *const i8);
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
    type F = unsafe extern "C" fn(*mut JNIEnv, *const i8) -> jstring;
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
    type F = unsafe extern "C" fn(*mut JNIEnv, jclass, *const i8, *const i8) -> jmethodID;
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
    type F = unsafe extern "C" fn(*mut JNIEnv, jclass, *const i8, *const i8) -> jfieldID;
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
