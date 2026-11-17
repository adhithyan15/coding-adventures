//! Golden fixture verification through C ABI FFI functions (MUS01/MUS04/MUS07).

use coding_adventures_sha256::sha256_hex;
use music_machine_ffi::{
    mm_free_buffer, mm_render_portable_json_to_pcm16, mm_render_portable_text_to_pcm16,
    MM_STATUS_OK,
};
use std::fs;
use std::path::{Path, PathBuf};

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/fixtures/music-machine")
        .join(name)
}

#[test]
fn test_ffi_golden_portable_text_parity() {
    let txt_path = fixture_path("canonical_score.txt");
    let txt_content = fs::read_to_string(&txt_path).expect("read canonical_score.txt");

    let mut out_ptr: *mut i16 = std::ptr::null_mut();
    let mut out_len: usize = 0;
    let mut out_clipped: usize = 0;

    unsafe {
        let status = mm_render_portable_text_to_pcm16(
            txt_content.as_ptr(),
            txt_content.len(),
            0,
            &mut out_ptr,
            &mut out_len,
            &mut out_clipped,
        );

        assert_eq!(status, MM_STATUS_OK);
        assert!(!out_ptr.is_null());
        assert_eq!(out_len, 96138);
        assert_eq!(out_clipped, 0);

        let slice = std::slice::from_raw_parts(out_ptr, out_len);

        // Convert to little-endian bytes and verify SHA-256
        let mut pcm_bytes = Vec::with_capacity(out_len * 2);
        for &s in slice {
            pcm_bytes.extend_from_slice(&s.to_le_bytes());
        }
        let digest = sha256_hex(&pcm_bytes);
        assert_eq!(
            digest,
            "25bc4eb1b0d7a10d40a55afc0f8b0b4d65132a82f538245ee2698700ff4acee1"
        );

        // Safe free
        mm_free_buffer(out_ptr, out_len);
    }
}

#[test]
fn test_ffi_golden_portable_json_parity() {
    let json_path = fixture_path("canonical_score.json");
    let json_content = fs::read_to_string(&json_path).expect("read canonical_score.json");

    let mut out_ptr: *mut i16 = std::ptr::null_mut();
    let mut out_len: usize = 0;
    let mut out_clipped: usize = 0;

    unsafe {
        let status = mm_render_portable_json_to_pcm16(
            json_content.as_ptr(),
            json_content.len(),
            0,
            &mut out_ptr,
            &mut out_len,
            &mut out_clipped,
        );

        assert_eq!(status, MM_STATUS_OK);
        assert!(!out_ptr.is_null());
        assert_eq!(out_len, 96138);
        assert_eq!(out_clipped, 0);

        let slice = std::slice::from_raw_parts(out_ptr, out_len);

        let mut pcm_bytes = Vec::with_capacity(out_len * 2);
        for &s in slice {
            pcm_bytes.extend_from_slice(&s.to_le_bytes());
        }
        let digest = sha256_hex(&pcm_bytes);
        assert_eq!(
            digest,
            "25bc4eb1b0d7a10d40a55afc0f8b0b4d65132a82f538245ee2698700ff4acee1"
        );

        mm_free_buffer(out_ptr, out_len);
    }
}
