package com.codingadventures.md5native;

/**
 * JNI bindings to the {@code md5_native_jni} Rust cdylib, which wraps the
 * pure-Rust {@code coding_adventures_md5} crate. Each {@code native} method maps
 * to a {@code Java_com_codingadventures_md5native_Native_*} function.
 */
final class Native {
    static {
        System.loadLibrary("md5_native_jni");
    }

    private Native() {
    }

    static native byte[] nativeDigest(byte[] data);

    static native long nativeHasherNew();

    static native void nativeHasherUpdate(long handle, byte[] data);

    static native byte[] nativeHasherDigest(long handle);

    static native long nativeHasherClone(long handle);

    static native void nativeHasherFree(long handle);
}
