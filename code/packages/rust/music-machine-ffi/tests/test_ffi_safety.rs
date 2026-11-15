//! Family-1 memory safety tests for C ABI FFI (CWE-476, CWE-125, CWE-787, CWE-415, CWE-416).

use music_machine_ffi::{
    mm_free_buffer, mm_render_portable_json_to_pcm16, mm_render_portable_text_to_pcm16,
    mm_render_score_text_to_pcm16, MM_STATUS_BUDGET_EXCEEDED, MM_STATUS_INVALID_ARGUMENT,
    MM_STATUS_NULL_POINTER, MM_STATUS_OK, MM_STATUS_PARSE_ERROR,
};

#[test]
fn test_null_pointer_rejections_cwe_476() {
    let mut out_ptr: *mut i16 = std::ptr::null_mut();
    let mut out_len: usize = 0;
    let mut out_clipped: usize = 0;

    let valid_text = b"C4/q D4/q";

    // Text score NULL pointer tests
    unsafe {
        // text_ptr is NULL
        assert_eq!(
            mm_render_score_text_to_pcm16(
                std::ptr::null(),
                valid_text.len(),
                44100,
                &mut out_ptr,
                &mut out_len,
                &mut out_clipped,
            ),
            MM_STATUS_NULL_POINTER
        );

        // out_samples_ptr is NULL
        assert_eq!(
            mm_render_score_text_to_pcm16(
                valid_text.as_ptr(),
                valid_text.len(),
                44100,
                std::ptr::null_mut(),
                &mut out_len,
                &mut out_clipped,
            ),
            MM_STATUS_NULL_POINTER
        );

        // out_samples_len is NULL
        assert_eq!(
            mm_render_score_text_to_pcm16(
                valid_text.as_ptr(),
                valid_text.len(),
                44100,
                &mut out_ptr,
                std::ptr::null_mut(),
                &mut out_clipped,
            ),
            MM_STATUS_NULL_POINTER
        );

        // out_clipped_count is NULL
        assert_eq!(
            mm_render_score_text_to_pcm16(
                valid_text.as_ptr(),
                valid_text.len(),
                44100,
                &mut out_ptr,
                &mut out_len,
                std::ptr::null_mut(),
            ),
            MM_STATUS_NULL_POINTER
        );
    }

    // Portable JSON NULL pointer tests
    let valid_json = b"{\"format_version\": \"music-machine-score/v2\"}";
    unsafe {
        assert_eq!(
            mm_render_portable_json_to_pcm16(
                std::ptr::null(),
                valid_json.len(),
                0,
                &mut out_ptr,
                &mut out_len,
                &mut out_clipped,
            ),
            MM_STATUS_NULL_POINTER
        );

        assert_eq!(
            mm_render_portable_json_to_pcm16(
                valid_json.as_ptr(),
                valid_json.len(),
                0,
                std::ptr::null_mut(),
                &mut out_len,
                &mut out_clipped,
            ),
            MM_STATUS_NULL_POINTER
        );
    }

    // Portable text NULL pointer tests
    let valid_port = b"format: music-machine-score/v2\n";
    unsafe {
        assert_eq!(
            mm_render_portable_text_to_pcm16(
                std::ptr::null(),
                valid_port.len(),
                0,
                &mut out_ptr,
                &mut out_len,
                &mut out_clipped,
            ),
            MM_STATUS_NULL_POINTER
        );

        assert_eq!(
            mm_render_portable_text_to_pcm16(
                valid_port.as_ptr(),
                valid_port.len(),
                0,
                std::ptr::null_mut(),
                &mut out_len,
                &mut out_clipped,
            ),
            MM_STATUS_NULL_POINTER
        );
    }
}

#[test]
fn test_free_buffer_safety_cwe_476() {
    unsafe {
        // Free NULL pointer is a safe no-op
        mm_free_buffer(std::ptr::null_mut(), 0);
        mm_free_buffer(std::ptr::null_mut(), 1024);

        // Non-null with length 0 is a safe no-op
        let mut dummy: i16 = 42;
        mm_free_buffer(&mut dummy as *mut i16, 0);
    }
}

#[test]
fn test_invalid_arguments_and_bounds_cwe_125_cwe_400() {
    let mut out_ptr: *mut i16 = std::ptr::null_mut();
    let mut out_len: usize = 0;
    let mut out_clipped: usize = 0;

    let text = b"C4/q";

    unsafe {
        // Zero text length
        assert_eq!(
            mm_render_score_text_to_pcm16(
                text.as_ptr(),
                0,
                44100,
                &mut out_ptr,
                &mut out_len,
                &mut out_clipped,
            ),
            MM_STATUS_INVALID_ARGUMENT
        );

        // Oversized length (> 1 MB)
        assert_eq!(
            mm_render_score_text_to_pcm16(
                text.as_ptr(),
                2_000_000,
                44100,
                &mut out_ptr,
                &mut out_len,
                &mut out_clipped,
            ),
            MM_STATUS_BUDGET_EXCEEDED
        );

        // Invalid UTF-8 bytes
        let invalid_utf8 = [0xFF, 0xFE, 0xFD];
        assert_eq!(
            mm_render_score_text_to_pcm16(
                invalid_utf8.as_ptr(),
                invalid_utf8.len(),
                44100,
                &mut out_ptr,
                &mut out_len,
                &mut out_clipped,
            ),
            MM_STATUS_INVALID_ARGUMENT
        );

        // Parse error
        let bad_score = b"invalid syntax here";
        assert_eq!(
            mm_render_score_text_to_pcm16(
                bad_score.as_ptr(),
                bad_score.len(),
                44100,
                &mut out_ptr,
                &mut out_len,
                &mut out_clipped,
            ),
            MM_STATUS_PARSE_ERROR
        );
    }
}

#[test]
fn test_render_and_free_lifecycle_cwe_415_cwe_416() {
    let score_text = b"title: Simple Test\ntempo: 120\nC4/q D4/q\n";

    let mut out_ptr: *mut i16 = std::ptr::null_mut();
    let mut out_len: usize = 0;
    let mut out_clipped: usize = 0;

    unsafe {
        let status = mm_render_score_text_to_pcm16(
            score_text.as_ptr(),
            score_text.len(),
            44100,
            &mut out_ptr,
            &mut out_len,
            &mut out_clipped,
        );
        assert_eq!(status, MM_STATUS_OK);
        assert!(!out_ptr.is_null());
        assert!(out_len > 0);

        // Read sample slice safely
        let slice = std::slice::from_raw_parts(out_ptr, out_len);
        assert!(slice.iter().any(|&s| s != 0));

        // Free allocated memory exactly once
        mm_free_buffer(out_ptr, out_len);
    }
}
