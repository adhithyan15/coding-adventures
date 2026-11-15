//! # Music Machine C FFI Bridge
//!
//! Confined C ABI for `music-machine-core` with Family-1 memory safety guarantees:
//!
//! - **CWE-476**: Comprehensive NULL pointer validation with safe no-op on `mm_free_buffer(NULL, ...)`.
//! - **CWE-125 / CWE-787**: Bounded buffer reads strictly enforced against `MAX_SCORE_BYTES`.
//! - **CWE-190**: Checked integer arithmetic on all buffer sizes and slice conversions.
//! - **CWE-415 / CWE-416**: Single-owner `Box<[i16]>` raw pointer lifecycle (`Box::into_raw` / `Box::from_raw`).
//! - **Unwind Safety**: Every entry point catches unwinding panics with `std::panic::catch_unwind`.

#![deny(unsafe_op_in_unsafe_fn)]

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::slice;

use music_machine_core::{
    parse_portable_score, parse_score, render_portable_score_to_pcm, render_score_to_pcm,
    DEFAULT_MAX_SAMPLE_COUNT, MAX_SCORE_BYTES,
};

/// Return status codes for C ABI functions.
pub const MM_STATUS_OK: i32 = 0;
pub const MM_STATUS_NULL_POINTER: i32 = 1;
pub const MM_STATUS_INVALID_ARGUMENT: i32 = 2;
pub const MM_STATUS_BUDGET_EXCEEDED: i32 = 3;
pub const MM_STATUS_PARSE_ERROR: i32 = 4;
pub const MM_STATUS_RENDER_ERROR: i32 = 5;
pub const MM_STATUS_PANIC: i32 = 6;

/// Free a PCM sample buffer allocated by `music-machine-ffi`.
///
/// # Safety
/// - If `ptr` is NULL or `len == 0`, this is a safe no-op (CWE-476).
/// - If non-null, `ptr` must have originated from a previous successful call to
///   `mm_render_*` and `len` must equal the exact value returned in `out_samples_len`.
/// - Memory is reclaimed exactly once (CWE-415 / CWE-416).
#[no_mangle]
pub unsafe extern "C" fn mm_free_buffer(ptr: *mut i16, len: usize) {
    if ptr.is_null() || len == 0 {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let slice_ptr = std::ptr::slice_from_raw_parts_mut(ptr, len);
        // SAFETY: ptr was created by Box::into_raw with length len and is non-null.
        unsafe {
            drop(Box::from_raw(slice_ptr));
        }
    }));
}

/// Render a beginner text score to signed 16-bit mono PCM audio.
///
/// # Safety
/// - `text_ptr` must point to at least `text_len` readable bytes.
/// - Output pointers must be valid and non-null (checked up front).
/// - Output buffer is allocated on heap and must be freed by caller via `mm_free_buffer`.
#[no_mangle]
pub unsafe extern "C" fn mm_render_score_text_to_pcm16(
    text_ptr: *const u8,
    text_len: usize,
    sample_rate_hz: u32,
    out_samples_ptr: *mut *mut i16,
    out_samples_len: *mut usize,
    out_clipped_count: *mut usize,
) -> i32 {
    if out_samples_ptr.is_null() || out_samples_len.is_null() || out_clipped_count.is_null() {
        return MM_STATUS_NULL_POINTER;
    }

    // Initialize out pointers before any failure path
    // SAFETY: pointers verified non-null above.
    unsafe {
        *out_samples_ptr = std::ptr::null_mut();
        *out_samples_len = 0;
        *out_clipped_count = 0;
    }

    if text_ptr.is_null() {
        return MM_STATUS_NULL_POINTER;
    }
    if text_len == 0 {
        return MM_STATUS_INVALID_ARGUMENT;
    }
    if text_len > MAX_SCORE_BYTES {
        return MM_STATUS_BUDGET_EXCEEDED;
    }

    let unwind_res = catch_unwind(AssertUnwindSafe(|| -> i32 {
        // SAFETY: text_ptr verified non-null and text_len <= MAX_SCORE_BYTES.
        let bytes = unsafe { slice::from_raw_parts(text_ptr, text_len) };
        let text = match std::str::from_utf8(bytes) {
            Ok(t) => t,
            Err(_) => return MM_STATUS_INVALID_ARGUMENT,
        };

        let mut score = match parse_score(text) {
            Ok(s) => s,
            Err(e) => {
                return match e {
                    music_machine_core::MusicMachineError::BudgetExceeded(_) => {
                        MM_STATUS_BUDGET_EXCEEDED
                    }
                    music_machine_core::MusicMachineError::InvalidArgument(_) => {
                        MM_STATUS_INVALID_ARGUMENT
                    }
                    _ => MM_STATUS_PARSE_ERROR,
                };
            }
        };

        if sample_rate_hz > 0 {
            score.sample_rate_hz = sample_rate_hz;
        }

        let rendered = match render_score_to_pcm(&score, DEFAULT_MAX_SAMPLE_COUNT) {
            Ok(r) => r,
            Err(e) => {
                return match e {
                    music_machine_core::MusicMachineError::BudgetExceeded(_) => {
                        MM_STATUS_BUDGET_EXCEEDED
                    }
                    music_machine_core::MusicMachineError::InvalidArgument(_) => {
                        MM_STATUS_INVALID_ARGUMENT
                    }
                    _ => MM_STATUS_RENDER_ERROR,
                };
            }
        };

        let samples = rendered.pcm_buffer.samples().to_vec();
        let clipped = rendered.pcm_buffer.clipped_sample_count();
        let len = samples.len();
        let boxed = samples.into_boxed_slice();
        let raw_ptr = Box::into_raw(boxed) as *mut i16;

        // SAFETY: out pointers verified non-null.
        unsafe {
            *out_samples_ptr = raw_ptr;
            *out_samples_len = len;
            *out_clipped_count = clipped;
        }

        MM_STATUS_OK
    }));

    unwind_res.unwrap_or(MM_STATUS_PANIC)
}

/// Render a canonical JSON score to signed 16-bit mono PCM audio.
///
/// # Safety
/// - `json_ptr` must point to at least `json_len` readable bytes.
/// - Output pointers must be valid and non-null (checked up front).
/// - Output buffer is allocated on heap and must be freed by caller via `mm_free_buffer`.
#[no_mangle]
pub unsafe extern "C" fn mm_render_portable_json_to_pcm16(
    json_ptr: *const u8,
    json_len: usize,
    max_samples: usize,
    out_samples_ptr: *mut *mut i16,
    out_samples_len: *mut usize,
    out_clipped_count: *mut usize,
) -> i32 {
    if out_samples_ptr.is_null() || out_samples_len.is_null() || out_clipped_count.is_null() {
        return MM_STATUS_NULL_POINTER;
    }

    // Initialize out pointers before any failure path
    // SAFETY: pointers verified non-null above.
    unsafe {
        *out_samples_ptr = std::ptr::null_mut();
        *out_samples_len = 0;
        *out_clipped_count = 0;
    }

    if json_ptr.is_null() {
        return MM_STATUS_NULL_POINTER;
    }
    if json_len == 0 {
        return MM_STATUS_INVALID_ARGUMENT;
    }
    if json_len > MAX_SCORE_BYTES {
        return MM_STATUS_BUDGET_EXCEEDED;
    }

    let unwind_res = catch_unwind(AssertUnwindSafe(|| -> i32 {
        // SAFETY: json_ptr verified non-null and json_len <= MAX_SCORE_BYTES.
        let bytes = unsafe { slice::from_raw_parts(json_ptr, json_len) };
        let json_text = match std::str::from_utf8(bytes) {
            Ok(t) => t,
            Err(_) => return MM_STATUS_INVALID_ARGUMENT,
        };

        let score = match music_machine_core::PortableScore::from_json(json_text) {
            Ok(s) => s,
            Err(e) => {
                return match e {
                    music_machine_core::MusicMachineError::BudgetExceeded(_) => {
                        MM_STATUS_BUDGET_EXCEEDED
                    }
                    music_machine_core::MusicMachineError::InvalidArgument(_) => {
                        MM_STATUS_INVALID_ARGUMENT
                    }
                    _ => MM_STATUS_PARSE_ERROR,
                };
            }
        };

        let max_s = if max_samples == 0 {
            DEFAULT_MAX_SAMPLE_COUNT
        } else {
            max_samples
        };

        let rendered = match render_portable_score_to_pcm(&score, max_s) {
            Ok(r) => r,
            Err(e) => {
                return match e {
                    music_machine_core::MusicMachineError::BudgetExceeded(_) => {
                        MM_STATUS_BUDGET_EXCEEDED
                    }
                    music_machine_core::MusicMachineError::InvalidArgument(_) => {
                        MM_STATUS_INVALID_ARGUMENT
                    }
                    _ => MM_STATUS_RENDER_ERROR,
                };
            }
        };

        let samples = rendered.pcm_buffer.samples().to_vec();
        let clipped = rendered.clipped_sample_count;
        let len = samples.len();
        let boxed = samples.into_boxed_slice();
        let raw_ptr = Box::into_raw(boxed) as *mut i16;

        // SAFETY: out pointers verified non-null.
        unsafe {
            *out_samples_ptr = raw_ptr;
            *out_samples_len = len;
            *out_clipped_count = clipped;
        }

        MM_STATUS_OK
    }));

    unwind_res.unwrap_or(MM_STATUS_PANIC)
}

/// Render a canonical V2 portable text score to signed 16-bit mono PCM audio.
///
/// # Safety
/// - `text_ptr` must point to at least `text_len` readable bytes.
/// - Output pointers must be valid and non-null (checked up front).
/// - Output buffer is allocated on heap and must be freed by caller via `mm_free_buffer`.
#[no_mangle]
pub unsafe extern "C" fn mm_render_portable_text_to_pcm16(
    text_ptr: *const u8,
    text_len: usize,
    max_samples: usize,
    out_samples_ptr: *mut *mut i16,
    out_samples_len: *mut usize,
    out_clipped_count: *mut usize,
) -> i32 {
    if out_samples_ptr.is_null() || out_samples_len.is_null() || out_clipped_count.is_null() {
        return MM_STATUS_NULL_POINTER;
    }

    // SAFETY: pointers verified non-null above.
    unsafe {
        *out_samples_ptr = std::ptr::null_mut();
        *out_samples_len = 0;
        *out_clipped_count = 0;
    }

    if text_ptr.is_null() {
        return MM_STATUS_NULL_POINTER;
    }
    if text_len == 0 {
        return MM_STATUS_INVALID_ARGUMENT;
    }
    if text_len > MAX_SCORE_BYTES {
        return MM_STATUS_BUDGET_EXCEEDED;
    }

    let unwind_res = catch_unwind(AssertUnwindSafe(|| -> i32 {
        // SAFETY: text_ptr verified non-null and text_len <= MAX_SCORE_BYTES.
        let bytes = unsafe { slice::from_raw_parts(text_ptr, text_len) };
        let text = match std::str::from_utf8(bytes) {
            Ok(t) => t,
            Err(_) => return MM_STATUS_INVALID_ARGUMENT,
        };

        let score = match parse_portable_score(text) {
            Ok(s) => s,
            Err(e) => {
                return match e {
                    music_machine_core::MusicMachineError::BudgetExceeded(_) => {
                        MM_STATUS_BUDGET_EXCEEDED
                    }
                    music_machine_core::MusicMachineError::InvalidArgument(_) => {
                        MM_STATUS_INVALID_ARGUMENT
                    }
                    _ => MM_STATUS_PARSE_ERROR,
                };
            }
        };

        let max_s = if max_samples == 0 {
            DEFAULT_MAX_SAMPLE_COUNT
        } else {
            max_samples
        };

        let rendered = match render_portable_score_to_pcm(&score, max_s) {
            Ok(r) => r,
            Err(e) => {
                return match e {
                    music_machine_core::MusicMachineError::BudgetExceeded(_) => {
                        MM_STATUS_BUDGET_EXCEEDED
                    }
                    music_machine_core::MusicMachineError::InvalidArgument(_) => {
                        MM_STATUS_INVALID_ARGUMENT
                    }
                    _ => MM_STATUS_RENDER_ERROR,
                };
            }
        };

        let samples = rendered.pcm_buffer.samples().to_vec();
        let clipped = rendered.clipped_sample_count;
        let len = samples.len();
        let boxed = samples.into_boxed_slice();
        let raw_ptr = Box::into_raw(boxed) as *mut i16;

        // SAFETY: out pointers verified non-null.
        unsafe {
            *out_samples_ptr = raw_ptr;
            *out_samples_len = len;
            *out_clipped_count = clipped;
        }

        MM_STATUS_OK
    }));

    unwind_res.unwrap_or(MM_STATUS_PANIC)
}
