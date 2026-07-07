# Changelog — jni-bridge

## [0.2.0] — 2026-07-07

### Added

Byte-array support, so file bytes (a spreadsheet a user opened, or a document to
download) can cross the JNI boundary as a Java `byte[]`.

- `jbyteArray` type alias (a `jarray` of `jbyte`).
- Raw slot wrappers at their JNI-spec offsets (derived from the existing
  `NewDoubleArray` = 182 / `SetDoubleArrayRegion` = 214 anchors): `GetArrayLength`
  = 171, `NewByteArray` = 176, `GetByteArrayRegion` = 200, `SetByteArrayRegion` =
  208 → `jni_get_array_length`, `jni_new_byte_array`, `jni_get_byte_array_region`,
  `jni_set_byte_array_region`.
- Convenience wrappers: `jni_get_byte_array(env, arr) -> Vec<u8>` (length query +
  one region copy; `jbyte`/`u8` share a bit pattern) and
  `jni_new_byte_array_from(env, &[u8]) -> jbyteArray`. Both null-safe.
- 2 tests against a **mock JNIEnv function table** (a `[*const c_void; 232]` with
  the byte-array slots pointing at Rust emulators; a `jbyteArray` modelled as a
  `*mut Vec<u8>`): a bytes round-trip that keeps a high bit (`0xD0`, the `.xls`
  magic), and empty/null-array safety.

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
