# jni-bridge

Zero-dependency Rust wrapper for the Java Native Interface (JNI).

## Overview

`jni-bridge` lets you write Rust-based JNI native libraries without pulling
in the `jni` or `jni-sys` crates, without running `bindgen`, and without
JDK headers at build time.

It provides:
- All JNI primitive and reference types (`jdouble`, `jstring`, `jclass`, …)
- The `JNIEnv` pointer type
- The `jvalue` union for `NewObjectA` constructor calls
- Safe helper functions that call into the JVM's function-pointer table

## How it works

JNI exports named `Java_<package>_<Class>_<method>` are regular `extern "C"`
functions.  The first argument is always `*mut JNIEnv` — a pointer to the
JVM's dispatch table.  All JNI operations (string conversion, object
creation, exception throwing) go through that table.

Instead of defining the 232-slot `JNINativeInterface_` struct (which would
require every slot to be in the exact right position), `jni-bridge` reads
function pointers at fixed offsets from the JNI specification:

```rust
// (*env)->FindClass(env, "com/foo/Bar")  in C becomes:
let fn_ptr = *(*env).add(6);          // FindClass is at index 6
let f: unsafe extern "C" fn(*mut JNIEnv, *const i8) -> jclass = transmute(fn_ptr);
let cls = f(env, b"com/foo/Bar\0".as_ptr() as *const i8);
```

The public helpers wrap this pattern so callsites are readable.

## Quick example

```rust
use jni_bridge::*;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_example_Foo_greet(
    env: *mut JNIEnv,
    _class: jclass,
    name: jstring,
) -> jstring {
    let s = jni_get_string_utf(env, name).unwrap_or_default();
    jni_new_string_utf(env, &format!("Hello, {}!", s))
}
```

## API

| Function | JNI slot offset | Description |
|---|---|---|
| `jni_find_class(env, name)` | 6 | Find a class by internal name |
| `jni_throw_new(env, class, msg)` | 14 | Throw a pending exception |
| `jni_exception_clear(env)` | 17 | Clear pending exception |
| `jni_exception_check(env)` | 228 | Check for pending exception |
| `jni_get_string_utf(env, s)` | 169, 170 | Java string → `Option<String>` |
| `jni_new_string_utf(env, s)` | 167 | `&str` → Java string |
| `jni_get_method_id(env, cls, name, sig)` | 33 | Look up method ID |
| `jni_get_field_id(env, cls, name, sig)` | 94 | Look up field ID |
| `jni_new_object_a(env, cls, ctor, args)` | 30 | Create Java object (array args) |
| `jni_set_double_field(env, obj, fid, val)` | 112 | Set `double` field |
| `jni_set_object_field(env, obj, fid, val)` | 104 | Set object field |
| `jni_new_double_array(env, len)` | 182 | Create `double[]` |
| `jni_set_double_array_region(env, arr, s, l, buf)` | 214 | Fill `double[]` from buffer |
