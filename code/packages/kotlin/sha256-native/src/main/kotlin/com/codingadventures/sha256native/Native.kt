package com.codingadventures.sha256native

/**
 * JNI bindings to the `sha256_native_jni` Rust cdylib (shared with
 * `java/sha256-native`), which wraps `coding_adventures_sha256`. The external
 * methods resolve to the same `Java_com_codingadventures_sha256native_Native_*`
 * exports, so Kotlin reuses the existing cdylib with no new Rust crate.
 */
internal object Native {
    init {
        System.loadLibrary("sha256_native_jni")
    }

    external fun nativeDigest(data: ByteArray): ByteArray
    external fun nativeHasherNew(): Long
    external fun nativeHasherUpdate(handle: Long, data: ByteArray)
    external fun nativeHasherDigest(handle: Long): ByteArray
    external fun nativeHasherClone(handle: Long): Long
    external fun nativeHasherFree(handle: Long)
}
