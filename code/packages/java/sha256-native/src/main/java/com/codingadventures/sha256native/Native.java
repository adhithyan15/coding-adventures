package com.codingadventures.sha256native;

/**
 * JNI bindings to the {@code sha256_native_jni} Rust cdylib, which wraps the
 * pure-Rust {@code coding_adventures_sha256} crate. Each {@code native} method
 * maps to a {@code Java_com_codingadventures_sha256native_Native_*} function in
 * {@code code/packages/rust/sha256-native-jni/src/lib.rs}.
 *
 * <p>The streaming hasher is represented as an opaque {@code long} peer pointer
 * to a Rust {@code Sha256Hasher}. A {@code 0} handle is a safe no-op on every
 * native call.
 */
final class Native {

    static {
        System.loadLibrary("sha256_native_jni");
    }

    private Native() {
    }

    /** One-shot: the 32-byte SHA-256 digest of {@code data}. */
    static native byte[] nativeDigest(byte[] data);

    /** Allocate a streaming hasher; returns its peer pointer. */
    static native long nativeHasherNew();

    /** Feed {@code data} into the hasher at {@code handle}. */
    static native void nativeHasherUpdate(long handle, byte[] data);

    /** The current 32-byte digest of the hasher (non-destructive). */
    static native byte[] nativeHasherDigest(long handle);

    /** An independent copy of the hasher; returns a new peer pointer. */
    static native long nativeHasherClone(long handle);

    /** Free a hasher peer pointer. */
    static native void nativeHasherFree(long handle);
}
