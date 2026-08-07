//! `sha256-native-jni` — a JNI bridge from Java/Kotlin to the Rust
//! `coding_adventures_sha256` crate.
//!
//! Each `native` method on `com.codingadventures.sha256native.Native` maps to a
//! `Java_com_codingadventures_sha256native_Native_*` export here. The JVM loads
//! the compiled cdylib via `System.loadLibrary("sha256_native_jni")`.
//!
//! ## Design
//!
//! * `nativeDigest(byte[])` → `byte[]` — one-shot digest; the input `byte[]` is
//!   copied into a Rust `Vec<u8>` and the 32-byte result into a fresh `byte[]`.
//! * The streaming hasher is an opaque **`long` peer pointer**
//!   (`Box::into_raw` / `Box::from_raw`): `nativeHasherNew` returns it,
//!   `nativeHasherUpdate` / `nativeHasherDigest` / `nativeHasherClone` operate on
//!   it, and `nativeHasherFree` reclaims it. Every call validates the pointer
//!   against 0, so a null handle is a safe no-op.
//!
//! There is no callback into the JVM, so no thread attachment or global-ref
//! machinery is needed.

// JNI entry points are `Java_<Pkg>_<Class>_<method>` (not snake_case) and share
// one uniform safety contract (called by the JVM), so per-fn `# Safety` docs
// would be noise.
#![allow(non_snake_case, clippy::missing_safety_doc)]

use jni_bridge::{
    jbyteArray, jclass, jlong, jni_get_byte_array, jni_new_byte_array_from, JNIEnv,
};

use coding_adventures_sha256::{sha256, Sha256Hasher};

/// `nativeDigest(byte[] data) -> byte[]` — the 32-byte SHA-256 digest.
#[no_mangle]
pub unsafe extern "C" fn Java_com_codingadventures_sha256native_Native_nativeDigest(
    env: *mut JNIEnv,
    _class: jclass,
    data: jbyteArray,
) -> jbyteArray {
    let input = jni_get_byte_array(env, data);
    let digest = sha256(&input);
    jni_new_byte_array_from(env, &digest)
}

/// `nativeHasherNew() -> long` — allocate a streaming hasher, return its pointer.
#[no_mangle]
pub extern "C" fn Java_com_codingadventures_sha256native_Native_nativeHasherNew(
    _env: *mut JNIEnv,
    _class: jclass,
) -> jlong {
    Box::into_raw(Box::new(Sha256Hasher::new())) as jlong
}

/// `nativeHasherUpdate(long h, byte[] data)` — feed bytes into the hasher.
#[no_mangle]
pub unsafe extern "C" fn Java_com_codingadventures_sha256native_Native_nativeHasherUpdate(
    env: *mut JNIEnv,
    _class: jclass,
    h: jlong,
    data: jbyteArray,
) {
    if h == 0 {
        return;
    }
    let hasher = &mut *(h as *mut Sha256Hasher);
    let input = jni_get_byte_array(env, data);
    hasher.update(&input);
}

/// `nativeHasherDigest(long h) -> byte[]` — the current 32-byte digest
/// (non-destructive). A null handle yields an empty array.
#[no_mangle]
pub unsafe extern "C" fn Java_com_codingadventures_sha256native_Native_nativeHasherDigest(
    env: *mut JNIEnv,
    _class: jclass,
    h: jlong,
) -> jbyteArray {
    if h == 0 {
        return jni_new_byte_array_from(env, &[]);
    }
    let hasher = &*(h as *mut Sha256Hasher);
    jni_new_byte_array_from(env, &hasher.digest())
}

/// `nativeHasherClone(long h) -> long` — an independent copy (0 if `h` is 0).
#[no_mangle]
pub unsafe extern "C" fn Java_com_codingadventures_sha256native_Native_nativeHasherClone(
    _env: *mut JNIEnv,
    _class: jclass,
    h: jlong,
) -> jlong {
    if h == 0 {
        return 0;
    }
    let hasher = &*(h as *mut Sha256Hasher);
    Box::into_raw(Box::new(hasher.clone_hasher())) as jlong
}

/// `nativeHasherFree(long h)` — free a hasher handle. A 0 handle is a no-op.
#[no_mangle]
pub unsafe extern "C" fn Java_com_codingadventures_sha256native_Native_nativeHasherFree(
    _env: *mut JNIEnv,
    _class: jclass,
    h: jlong,
) {
    if h != 0 {
        drop(Box::from_raw(h as *mut Sha256Hasher));
    }
}

// Pure-Rust unit tests (no JVM): exercise the digest logic the JNI layer wraps.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_and_streaming_agree() {
        let one_shot = sha256(b"abc");
        let mut h = Sha256Hasher::new();
        h.update(b"ab");
        h.update(b"c");
        assert_eq!(h.digest(), one_shot);
        let hex: String = one_shot.iter().map(|b| format!("{:02x}", b)).collect();
        assert_eq!(
            hex,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
