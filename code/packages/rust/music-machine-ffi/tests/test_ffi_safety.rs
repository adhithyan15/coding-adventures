//! Family-1 memory safety tests for C ABI FFI (CWE-476, CWE-125, CWE-787, CWE-415, CWE-416).

use music_machine_core::{
    parse_score, render_score_to_pcm, DEFAULT_MAX_SAMPLE_COUNT, MAX_SAMPLE_COUNT, MAX_SCORE_BYTES,
};
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

#[test]
fn test_interior_nul_bytes_rejected_cwe_125() {
    let mut out_ptr: *mut i16 = 0x100usize as *mut i16;
    let mut out_len: usize = 99;
    let mut out_clipped: usize = 99;

    unsafe {
        let nul_v1 = b"C4/q\0D4/q";
        assert_eq!(
            mm_render_score_text_to_pcm16(
                nul_v1.as_ptr(),
                nul_v1.len(),
                44100,
                &mut out_ptr,
                &mut out_len,
                &mut out_clipped,
            ),
            MM_STATUS_INVALID_ARGUMENT
        );
        assert!(out_ptr.is_null());
        assert_eq!(out_len, 0);
        assert_eq!(out_clipped, 0);

        let nul_json = b"{\"format_version\":\"music-machine-score/v2\"}\0";
        assert_eq!(
            mm_render_portable_json_to_pcm16(
                nul_json.as_ptr(),
                nul_json.len(),
                0,
                &mut out_ptr,
                &mut out_len,
                &mut out_clipped,
            ),
            MM_STATUS_INVALID_ARGUMENT
        );
        assert!(out_ptr.is_null());

        let nul_port = b"format: music-machine-score/v2\0\n";
        assert_eq!(
            mm_render_portable_text_to_pcm16(
                nul_port.as_ptr(),
                nul_port.len(),
                0,
                &mut out_ptr,
                &mut out_len,
                &mut out_clipped,
            ),
            MM_STATUS_INVALID_ARGUMENT
        );
        assert!(out_ptr.is_null());
    }
}

#[test]
fn test_preflight_sample_rate_and_max_samples_cwe_190_cwe_400() {
    let mut out_ptr: *mut i16 = std::ptr::null_mut();
    let mut out_len: usize = 0;
    let mut out_clipped: usize = 0;

    let valid_v1 = b"C4/s";
    unsafe {
        // Below MIN_SAMPLE_RATE_HZ (1000)
        assert_eq!(
            mm_render_score_text_to_pcm16(
                valid_v1.as_ptr(),
                valid_v1.len(),
                999,
                &mut out_ptr,
                &mut out_len,
                &mut out_clipped,
            ),
            MM_STATUS_INVALID_ARGUMENT
        );
        // Above MAX_SAMPLE_RATE_HZ (192000)
        assert_eq!(
            mm_render_score_text_to_pcm16(
                valid_v1.as_ptr(),
                valid_v1.len(),
                192_001,
                &mut out_ptr,
                &mut out_len,
                &mut out_clipped,
            ),
            MM_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            mm_render_score_text_to_pcm16(
                valid_v1.as_ptr(),
                valid_v1.len(),
                u32::MAX,
                &mut out_ptr,
                &mut out_len,
                &mut out_clipped,
            ),
            MM_STATUS_INVALID_ARGUMENT
        );
        // Score-declared rate below MIN_SAMPLE_RATE_HZ when override == 0
        let low_rate_score = b"sample_rate: 500\nC4/s";
        assert_eq!(
            mm_render_score_text_to_pcm16(
                low_rate_score.as_ptr(),
                low_rate_score.len(),
                0,
                &mut out_ptr,
                &mut out_len,
                &mut out_clipped,
            ),
            MM_STATUS_INVALID_ARGUMENT
        );

        // max_samples > MAX_SAMPLE_COUNT must fail pre-flight with BUDGET_EXCEEDED
        let invalid_utf8 = [0xFFu8, 0xFE];
        assert_eq!(
            mm_render_portable_json_to_pcm16(
                invalid_utf8.as_ptr(),
                invalid_utf8.len(),
                MAX_SAMPLE_COUNT + 1,
                &mut out_ptr,
                &mut out_len,
                &mut out_clipped,
            ),
            MM_STATUS_BUDGET_EXCEEDED
        );
        assert_eq!(
            mm_render_portable_text_to_pcm16(
                invalid_utf8.as_ptr(),
                invalid_utf8.len(),
                usize::MAX,
                &mut out_ptr,
                &mut out_len,
                &mut out_clipped,
            ),
            MM_STATUS_BUDGET_EXCEEDED
        );

        // Exact MAX_SCORE_BYTES + 1 boundary
        assert_eq!(
            mm_render_portable_text_to_pcm16(
                valid_v1.as_ptr(),
                MAX_SCORE_BYTES + 1,
                0,
                &mut out_ptr,
                &mut out_len,
                &mut out_clipped,
            ),
            MM_STATUS_BUDGET_EXCEEDED
        );
    }
}

#[test]
fn test_out_ptr_aliasing_misalignment_and_free_guards() {
    let mut out_ptr: *mut i16 = std::ptr::null_mut();
    let mut out_len: usize = 0;
    let mut out_clipped: usize = 0;
    let valid_v1 = b"C4/s";

    unsafe {
        // Aliased out_samples_len and out_clipped_count
        assert_eq!(
            mm_render_score_text_to_pcm16(
                valid_v1.as_ptr(),
                valid_v1.len(),
                44100,
                &mut out_ptr,
                &mut out_len,
                &mut out_len,
            ),
            MM_STATUS_INVALID_ARGUMENT
        );

        // Misaligned out_samples_len pointer
        let mut raw_bytes = [0u8; 32];
        let mis_ptr = raw_bytes.as_mut_ptr().add(1) as *mut usize;
        assert_eq!(
            mm_render_score_text_to_pcm16(
                valid_v1.as_ptr(),
                valid_v1.len(),
                44100,
                &mut out_ptr,
                mis_ptr,
                &mut out_clipped,
            ),
            MM_STATUS_INVALID_ARGUMENT
        );

        // mm_free_buffer ignores misaligned or oversized len without panicking/UB
        let mut odd_buf = [0u8; 8];
        let odd_i16 = odd_buf.as_mut_ptr().add(1) as *mut i16;
        mm_free_buffer(odd_i16, 2);

        let mut valid_i16: i16 = 7;
        mm_free_buffer(&mut valid_i16 as *mut i16, MAX_SAMPLE_COUNT + 1);
        mm_free_buffer(&mut valid_i16 as *mut i16, usize::MAX);
    }
}

#[test]
fn test_empty_portable_score_and_clipped_parity() {
    let mut out_ptr: *mut i16 = 0x100usize as *mut i16;
    let mut out_len: usize = 99;
    let mut out_clipped: usize = 99;

    // Empty portable score (0 events -> 0 samples -> null pointer & 0 len)
    let empty_port = b"format: music-machine-score/v2\ntitle: Empty\n";
    unsafe {
        let status = mm_render_portable_text_to_pcm16(
            empty_port.as_ptr(),
            empty_port.len(),
            0,
            &mut out_ptr,
            &mut out_len,
            &mut out_clipped,
        );
        assert_eq!(status, MM_STATUS_OK);
        assert!(out_ptr.is_null());
        assert_eq!(out_len, 0);
        assert_eq!(out_clipped, 0);
        mm_free_buffer(out_ptr, out_len);
    }

    // Verify mm_render_score_text_to_pcm16 matches core clipped_sample_count
    let text = "instrument: drumkit\namplitude: 1.0\nkick/s kick/s crash/s\n";
    let parsed = parse_score(text).expect("valid drum score");
    let expected = render_score_to_pcm(&parsed, DEFAULT_MAX_SAMPLE_COUNT).expect("render");
    unsafe {
        let status = mm_render_score_text_to_pcm16(
            text.as_ptr(),
            text.len(),
            0,
            &mut out_ptr,
            &mut out_len,
            &mut out_clipped,
        );
        assert_eq!(status, MM_STATUS_OK);
        assert_eq!(out_len, expected.pcm_buffer.samples().len());
        assert_eq!(out_clipped, expected.clipped_sample_count);
        mm_free_buffer(out_ptr, out_len);
    }
}
