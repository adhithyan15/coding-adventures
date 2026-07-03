# Changelog — jni-bridge

## [0.1.0] — 2026-06-13

### Added

Initial release.  Zero-dependency Rust wrapper for the Java Native Interface.

- JNI primitive types: `jboolean`, `jbyte`, `jchar`, `jshort`, `jint`,
  `jlong`, `jfloat`, `jdouble`, `jsize`
- JNI reference types: `jobject`, `jclass`, `jstring`, `jarray`,
  `jthrowable`, `jmethodID`, `jfieldID`
- `JNIEnv` type alias
- `jvalue` union for `NewObjectA` variadic-free calls
- Helper functions using offset-based dispatch:
  - `jni_find_class`
  - `jni_throw_new`
  - `jni_exception_clear`
  - `jni_exception_check`
  - `jni_get_string_utf`
  - `jni_new_string_utf`
  - `jni_get_method_id`
  - `jni_get_field_id`
  - `jni_new_object_a`
  - `jni_set_double_field`
  - `jni_set_object_field`
  - `jni_new_double_array`
  - `jni_set_double_array_region`
