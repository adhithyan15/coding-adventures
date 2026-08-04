//! `md5-native-jni` — a JNI bridge from Java/Kotlin to the Rust
//! `coding_adventures_md5` crate. Each `native` method on
//! `com.codingadventures.md5native.Native` maps to a
//! `Java_com_codingadventures_md5native_Native_*` export. Loaded via
//! `System.loadLibrary("md5_native_jni")`. Mirrors `sha256-native-jni`
//! (16-byte digest). MD5 is broken — checksum use only.
#![allow(non_snake_case, clippy::missing_safety_doc)]

use jni_bridge::{jbyteArray, jclass, jlong, jni_get_byte_array, jni_new_byte_array_from, JNIEnv};

use coding_adventures_md5::{sum_md5, Digest};

#[no_mangle]
pub unsafe extern "C" fn Java_com_codingadventures_md5native_Native_nativeDigest(
    env: *mut JNIEnv,
    _class: jclass,
    data: jbyteArray,
) -> jbyteArray {
    let input = jni_get_byte_array(env, data);
    jni_new_byte_array_from(env, &sum_md5(&input))
}

#[no_mangle]
pub extern "C" fn Java_com_codingadventures_md5native_Native_nativeHasherNew(
    _env: *mut JNIEnv,
    _class: jclass,
) -> jlong {
    Box::into_raw(Box::new(Digest::new())) as jlong
}

#[no_mangle]
pub unsafe extern "C" fn Java_com_codingadventures_md5native_Native_nativeHasherUpdate(
    env: *mut JNIEnv,
    _class: jclass,
    h: jlong,
    data: jbyteArray,
) {
    if h == 0 {
        return;
    }
    let hasher = &mut *(h as *mut Digest);
    let input = jni_get_byte_array(env, data);
    hasher.update(&input);
}

#[no_mangle]
pub unsafe extern "C" fn Java_com_codingadventures_md5native_Native_nativeHasherDigest(
    env: *mut JNIEnv,
    _class: jclass,
    h: jlong,
) -> jbyteArray {
    if h == 0 {
        return jni_new_byte_array_from(env, &[]);
    }
    let hasher = &*(h as *mut Digest);
    jni_new_byte_array_from(env, &hasher.sum_md5())
}

#[no_mangle]
pub unsafe extern "C" fn Java_com_codingadventures_md5native_Native_nativeHasherClone(
    _env: *mut JNIEnv,
    _class: jclass,
    h: jlong,
) -> jlong {
    if h == 0 {
        return 0;
    }
    let hasher = &*(h as *mut Digest);
    Box::into_raw(Box::new(hasher.clone_digest())) as jlong
}

#[no_mangle]
pub unsafe extern "C" fn Java_com_codingadventures_md5native_Native_nativeHasherFree(
    _env: *mut JNIEnv,
    _class: jclass,
    h: jlong,
) {
    if h != 0 {
        drop(Box::from_raw(h as *mut Digest));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn digest_and_streaming_agree() {
        let one = sum_md5(b"abc");
        let mut h = Digest::new();
        h.update(b"ab");
        h.update(b"c");
        assert_eq!(h.sum_md5(), one);
        let hex: String = one.iter().map(|b| format!("{:02x}", b)).collect();
        assert_eq!(hex, "900150983cd24fb0d6963f7d28e17f72");
    }
}
