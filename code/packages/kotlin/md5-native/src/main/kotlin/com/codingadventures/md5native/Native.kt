package com.codingadventures.md5native

/**
 * JNI bindings to the `md5_native_jni` Rust cdylib (shared with
 * `java/md5-native`), which wraps `coding_adventures_md5`. The external methods
 * resolve to the same `Java_com_codingadventures_md5native_Native_*` exports, so
 * Kotlin reuses the existing cdylib with no new Rust crate. MD5 is broken —
 * checksum use only.
 */
internal object Native {
    init {
        System.loadLibrary("md5_native_jni")
    }

    external fun nativeDigest(data: ByteArray): ByteArray
    external fun nativeHasherNew(): Long
    external fun nativeHasherUpdate(handle: Long, data: ByteArray)
    external fun nativeHasherDigest(handle: Long): ByteArray
    external fun nativeHasherClone(handle: Long): Long
    external fun nativeHasherFree(handle: Long)
}
