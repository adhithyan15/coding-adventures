# sha256-native-jni

JNI native library bridging Java/Kotlin to the Rust `coding_adventures_sha256`
crate. Compiles to a cdylib (`libsha256_native_jni.{so,dylib,dll}`) loaded via
`System.loadLibrary("sha256_native_jni")`.

Each `native` method on `com.codingadventures.sha256native.Native` maps to a
`Java_com_codingadventures_sha256native_Native_*` export. Used by
`java/sha256-native`. Uses the zero-dependency `jni-bridge` (no `jni` crate, no
bindgen).
