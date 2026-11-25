//! # Music Machine C FFI Bridge
//!
//! Confined C ABI for `music-machine-core` with Family-1 memory safety guarantees:
//!
//! - **CWE-476**: Comprehensive NULL pointer, alignment, and out-pointer aliasing validation
//!   with safe no-op on `mm_free_buffer(NULL, ...)`.
//! - **CWE-125 / CWE-787**: Bounded buffer reads strictly enforced against `MAX_SCORE_BYTES`,
//!   with interior NUL byte rejection and address-space wraparound checks before `slice::from_raw_parts`.
//! - **CWE-190 / CWE-400**: Pre-flight bounds on `sample_rate_hz`, `max_samples`, `MAX_SCORE_BYTES`,
//!   and `MAX_SAMPLE_COUNT` plus checked byte-size arithmetic prior to parsing or allocation.
//! - **CWE-415 / CWE-416**: Single-owner `Box<[i16]>` raw pointer lifecycle (`Box::into_raw` /
//!   `Box::from_raw`) with volatile zero-on-free wipe in `mm_free_buffer`.
//! - **Unwind Safety**: Every `extern "C"` entry point wraps its entire body in
//!   `std::panic::catch_unwind`, preventing undefined behavior across the ABI boundary.

#![deny(unsafe_op_in_unsafe_fn)]

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::slice;
use std::sync::atomic::{compiler_fence, Ordering};

use music_machine_core::{
    parse_portable_score, parse_score, render_portable_score_to_pcm, render_score_to_pcm,
    MusicMachineError, PortableScore, DEFAULT_MAX_SAMPLE_COUNT, MAX_SAMPLE_COUNT,
    MAX_SAMPLE_RATE_HZ, MAX_SCORE_BYTES, MIN_SAMPLE_RATE_HZ,
};

/// Return status codes for C ABI functions.
pub const MM_STATUS_OK: i32 = 0;
pub const MM_STATUS_NULL_POINTER: i32 = 1;
pub const MM_STATUS_INVALID_ARGUMENT: i32 = 2;
pub const MM_STATUS_BUDGET_EXCEEDED: i32 = 3;
pub const MM_STATUS_PARSE_ERROR: i32 = 4;
pub const MM_STATUS_RENDER_ERROR: i32 = 5;
pub const MM_STATUS_PANIC: i32 = 6;

const MIN_RATE_HZ: u32 = MIN_SAMPLE_RATE_HZ as u32;
const MAX_RATE_HZ: u32 = MAX_SAMPLE_RATE_HZ as u32;

#[inline]
fn is_aligned<T>(ptr: *const T) -> bool {
    (ptr as usize).is_multiple_of(std::mem::align_of::<T>())
}

#[inline]
fn map_core_error(err: MusicMachineError, default_status: i32) -> i32 {
    match err {
        MusicMachineError::BudgetExceeded(_) => MM_STATUS_BUDGET_EXCEEDED,
        MusicMachineError::InvalidArgument(_) => MM_STATUS_INVALID_ARGUMENT,
        MusicMachineError::ParseError(_) => MM_STATUS_PARSE_ERROR,
        MusicMachineError::RenderError(_) => MM_STATUS_RENDER_ERROR,
        MusicMachineError::Io(_) => default_status,
    }
}

#[inline]
fn validate_sample_rate_override(sample_rate_hz: u32) -> Result<(), i32> {
    if sample_rate_hz != 0 && !(MIN_RATE_HZ..=MAX_RATE_HZ).contains(&sample_rate_hz) {
        return Err(MM_STATUS_INVALID_ARGUMENT);
    }
    Ok(())
}

#[inline]
fn validate_effective_sample_rate(sample_rate_hz: u32) -> Result<(), i32> {
    if !(MIN_RATE_HZ..=MAX_RATE_HZ).contains(&sample_rate_hz) {
        return Err(MM_STATUS_INVALID_ARGUMENT);
    }
    Ok(())
}

#[inline]
fn validate_max_samples_limit(max_samples: usize) -> Result<usize, i32> {
    if max_samples == 0 {
        return Ok(DEFAULT_MAX_SAMPLE_COUNT);
    }
    if max_samples > MAX_SAMPLE_COUNT {
        return Err(MM_STATUS_BUDGET_EXCEEDED);
    }
    Ok(max_samples)
}

/// Validate output pointers (non-null, aligned, pairwise distinct) and zero-initialize them.
///
/// # Safety
/// Caller must pass pointers that are either null or point to valid writable storage.
unsafe fn validate_and_init_out_ptrs(
    out_samples_ptr: *mut *mut i16,
    out_samples_len: *mut usize,
    out_clipped_count: *mut usize,
) -> Result<(), i32> {
    if out_samples_ptr.is_null() || out_samples_len.is_null() || out_clipped_count.is_null() {
        return Err(MM_STATUS_NULL_POINTER);
    }
    if !is_aligned(out_samples_ptr)
        || !is_aligned(out_samples_len)
        || !is_aligned(out_clipped_count)
    {
        return Err(MM_STATUS_INVALID_ARGUMENT);
    }
    let addr_ptr = out_samples_ptr as usize;
    let addr_len = out_samples_len as usize;
    let addr_clipped = out_clipped_count as usize;
    if addr_ptr == addr_len || addr_ptr == addr_clipped || addr_len == addr_clipped {
        return Err(MM_STATUS_INVALID_ARGUMENT);
    }

    // SAFETY: Pointers are verified non-null, properly aligned, and pairwise distinct.
    unsafe {
        *out_samples_ptr = std::ptr::null_mut();
        *out_samples_len = 0;
        *out_clipped_count = 0;
    }
    Ok(())
}

/// Bounded read of a UTF-8 input buffer, rejecting NULL, empty, oversized, wraparound, or interior NUL.
///
/// # Safety
/// If `ptr` is non-null, it must point to at least `min(len, MAX_SCORE_BYTES)` readable bytes.
unsafe fn read_bounded_utf8<'a>(ptr: *const u8, len: usize) -> Result<&'a str, i32> {
    if ptr.is_null() {
        return Err(MM_STATUS_NULL_POINTER);
    }
    if len == 0 {
        return Err(MM_STATUS_INVALID_ARGUMENT);
    }
    if len > MAX_SCORE_BYTES || len > (isize::MAX as usize) {
        return Err(MM_STATUS_BUDGET_EXCEEDED);
    }
    if (ptr as usize).checked_add(len).is_none() {
        return Err(MM_STATUS_INVALID_ARGUMENT);
    }

    // SAFETY: ptr is verified non-null, non-wrapping, and len <= MAX_SCORE_BYTES <= isize::MAX.
    let bytes = unsafe { slice::from_raw_parts(ptr, len) };
    if bytes.contains(&0) {
        return Err(MM_STATUS_INVALID_ARGUMENT);
    }
    std::str::from_utf8(bytes).map_err(|_| MM_STATUS_INVALID_ARGUMENT)
}

/// Transfer ownership of rendered PCM samples to the C caller via `Box::into_raw`.
///
/// # Safety
/// Output pointers must be valid, aligned, non-null, and pairwise distinct.
unsafe fn write_pcm_output(
    samples: Vec<i16>,
    clipped: usize,
    out_samples_ptr: *mut *mut i16,
    out_samples_len: *mut usize,
    out_clipped_count: *mut usize,
) -> i32 {
    let len = samples.len();
    if len > MAX_SAMPLE_COUNT {
        return MM_STATUS_BUDGET_EXCEEDED;
    }
    let Some(byte_len) = len.checked_mul(std::mem::size_of::<i16>()) else {
        return MM_STATUS_BUDGET_EXCEEDED;
    };
    if byte_len > (isize::MAX as usize) {
        return MM_STATUS_BUDGET_EXCEEDED;
    }

    if len == 0 {
        // SAFETY: Output pointers verified by validate_and_init_out_ptrs.
        unsafe {
            *out_samples_ptr = std::ptr::null_mut();
            *out_samples_len = 0;
            *out_clipped_count = clipped;
        }
        return MM_STATUS_OK;
    }

    let boxed = samples.into_boxed_slice();
    let raw_ptr = Box::into_raw(boxed) as *mut i16;

    // SAFETY: Output pointers verified by validate_and_init_out_ptrs.
    unsafe {
        *out_samples_ptr = raw_ptr;
        *out_samples_len = len;
        *out_clipped_count = clipped;
    }
    MM_STATUS_OK
}

/// Free a PCM sample buffer allocated by `music-machine-ffi`.
///
/// # Safety
/// - If `ptr` is NULL or `len == 0`, this is a safe no-op (CWE-476).
/// - If non-null, `ptr` must have originated from a previous successful call to
///   `mm_render_*` and `len` must equal the exact value returned in `out_samples_len`.
/// - Buffer memory is wiped with volatile zeros before reclamation (CWE-415 / CWE-416).
#[no_mangle]
pub unsafe extern "C" fn mm_free_buffer(ptr: *mut i16, len: usize) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if ptr.is_null() || len == 0 {
            return;
        }
        if !is_aligned(ptr) || len > MAX_SAMPLE_COUNT {
            return;
        }
        let Some(byte_len) = len.checked_mul(std::mem::size_of::<i16>()) else {
            return;
        };
        if byte_len > (isize::MAX as usize) || (ptr as usize).checked_add(byte_len).is_none() {
            return;
        }

        let slice_ptr = std::ptr::slice_from_raw_parts_mut(ptr, len);
        // SAFETY: ptr was created by Box::into_raw with length len, is non-null, aligned, and in bounds.
        let mut boxed = unsafe { Box::from_raw(slice_ptr) };
        for slot in boxed.iter_mut() {
            // SAFETY: slot points to a valid element within the owned Box<[i16]>.
            unsafe { std::ptr::write_volatile(slot, 0) };
        }
        compiler_fence(Ordering::SeqCst);
        drop(boxed);
    }));
}

/// Render a beginner text score to signed 16-bit mono PCM audio.
///
/// # Safety
/// - `text_ptr` must point to at least `text_len` readable bytes.
/// - Output pointers must be valid, aligned, non-null, and pairwise distinct.
/// - Output buffer is allocated on heap (unless `out_samples_len == 0`) and must be
///   freed by caller via `mm_free_buffer`.
#[no_mangle]
pub unsafe extern "C" fn mm_render_score_text_to_pcm16(
    text_ptr: *const u8,
    text_len: usize,
    sample_rate_hz: u32,
    out_samples_ptr: *mut *mut i16,
    out_samples_len: *mut usize,
    out_clipped_count: *mut usize,
) -> i32 {
    let unwind_res = catch_unwind(AssertUnwindSafe(|| -> i32 {
        // SAFETY: Caller guarantees out pointers are either null or valid writable storage.
        if let Err(code) = unsafe {
            validate_and_init_out_ptrs(out_samples_ptr, out_samples_len, out_clipped_count)
        } {
            return code;
        }

        if let Err(code) = validate_sample_rate_override(sample_rate_hz) {
            return code;
        }

        // SAFETY: Caller guarantees text_ptr (if non-null) points to text_len readable bytes.
        let text = match unsafe { read_bounded_utf8(text_ptr, text_len) } {
            Ok(t) => t,
            Err(code) => return code,
        };

        let mut score = match parse_score(text) {
            Ok(s) => s,
            Err(e) => return map_core_error(e, MM_STATUS_PARSE_ERROR),
        };

        if sample_rate_hz > 0 {
            score.sample_rate_hz = sample_rate_hz;
        }
        if let Err(code) = validate_effective_sample_rate(score.sample_rate_hz) {
            return code;
        }

        let rendered = match render_score_to_pcm(&score, DEFAULT_MAX_SAMPLE_COUNT) {
            Ok(r) => r,
            Err(e) => return map_core_error(e, MM_STATUS_RENDER_ERROR),
        };

        let samples = rendered.pcm_buffer.samples().to_vec();
        let clipped = rendered.clipped_sample_count;
        // SAFETY: out pointers validated and initialized above.
        unsafe {
            write_pcm_output(
                samples,
                clipped,
                out_samples_ptr,
                out_samples_len,
                out_clipped_count,
            )
        }
    }));

    unwind_res.unwrap_or(MM_STATUS_PANIC)
}

/// Render a canonical JSON score to signed 16-bit mono PCM audio.
///
/// # Safety
/// - `json_ptr` must point to at least `json_len` readable bytes.
/// - Output pointers must be valid, aligned, non-null, and pairwise distinct.
/// - Output buffer is allocated on heap (unless `out_samples_len == 0`) and must be
///   freed by caller via `mm_free_buffer`.
#[no_mangle]
pub unsafe extern "C" fn mm_render_portable_json_to_pcm16(
    json_ptr: *const u8,
    json_len: usize,
    max_samples: usize,
    out_samples_ptr: *mut *mut i16,
    out_samples_len: *mut usize,
    out_clipped_count: *mut usize,
) -> i32 {
    let unwind_res = catch_unwind(AssertUnwindSafe(|| -> i32 {
        // SAFETY: Caller guarantees out pointers are either null or valid writable storage.
        if let Err(code) = unsafe {
            validate_and_init_out_ptrs(out_samples_ptr, out_samples_len, out_clipped_count)
        } {
            return code;
        }

        let max_s = match validate_max_samples_limit(max_samples) {
            Ok(limit) => limit,
            Err(code) => return code,
        };

        // SAFETY: Caller guarantees json_ptr (if non-null) points to json_len readable bytes.
        let json_text = match unsafe { read_bounded_utf8(json_ptr, json_len) } {
            Ok(t) => t,
            Err(code) => return code,
        };

        let score = match PortableScore::from_json(json_text) {
            Ok(s) => s,
            Err(e) => return map_core_error(e, MM_STATUS_PARSE_ERROR),
        };

        let rendered = match render_portable_score_to_pcm(&score, max_s) {
            Ok(r) => r,
            Err(e) => return map_core_error(e, MM_STATUS_RENDER_ERROR),
        };

        let samples = rendered.pcm_buffer.samples().to_vec();
        let clipped = rendered.clipped_sample_count;
        // SAFETY: out pointers validated and initialized above.
        unsafe {
            write_pcm_output(
                samples,
                clipped,
                out_samples_ptr,
                out_samples_len,
                out_clipped_count,
            )
        }
    }));

    unwind_res.unwrap_or(MM_STATUS_PANIC)
}

/// Render a canonical V2 portable text score to signed 16-bit mono PCM audio.
///
/// # Safety
/// - `text_ptr` must point to at least `text_len` readable bytes.
/// - Output pointers must be valid, aligned, non-null, and pairwise distinct.
/// - Output buffer is allocated on heap (unless `out_samples_len == 0`) and must be
///   freed by caller via `mm_free_buffer`.
#[no_mangle]
pub unsafe extern "C" fn mm_render_portable_text_to_pcm16(
    text_ptr: *const u8,
    text_len: usize,
    max_samples: usize,
    out_samples_ptr: *mut *mut i16,
    out_samples_len: *mut usize,
    out_clipped_count: *mut usize,
) -> i32 {
    let unwind_res = catch_unwind(AssertUnwindSafe(|| -> i32 {
        // SAFETY: Caller guarantees out pointers are either null or valid writable storage.
        if let Err(code) = unsafe {
            validate_and_init_out_ptrs(out_samples_ptr, out_samples_len, out_clipped_count)
        } {
            return code;
        }

        let max_s = match validate_max_samples_limit(max_samples) {
            Ok(limit) => limit,
            Err(code) => return code,
        };

        // SAFETY: Caller guarantees text_ptr (if non-null) points to text_len readable bytes.
        let text = match unsafe { read_bounded_utf8(text_ptr, text_len) } {
            Ok(t) => t,
            Err(code) => return code,
        };

        let score = match parse_portable_score(text) {
            Ok(s) => s,
            Err(e) => return map_core_error(e, MM_STATUS_PARSE_ERROR),
        };

        let rendered = match render_portable_score_to_pcm(&score, max_s) {
            Ok(r) => r,
            Err(e) => return map_core_error(e, MM_STATUS_RENDER_ERROR),
        };

        let samples = rendered.pcm_buffer.samples().to_vec();
        let clipped = rendered.clipped_sample_count;
        // SAFETY: out pointers validated and initialized above.
        unsafe {
            write_pcm_output(
                samples,
                clipped,
                out_samples_ptr,
                out_samples_len,
                out_clipped_count,
            )
        }
    }));

    unwind_res.unwrap_or(MM_STATUS_PANIC)
}
