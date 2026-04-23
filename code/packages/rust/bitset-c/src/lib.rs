//! # bitset-c — Stable C ABI wrapper for the `bitset` crate.
//!
//! The pure Rust `bitset` crate is already the canonical implementation for
//! compact boolean arrays in this repo. This wrapper turns that implementation
//! into a small C ABI so higher-level runtimes can reuse it without embedding
//! Rust-specific concepts like iterators or unwinding panics.
//!
//! ## Why use opaque handles?
//!
//! C callers cannot own a Rust `Bitset` value directly because its layout is
//! not part of Rust's stability guarantees. Instead, each bitset lives on the
//! heap behind an opaque pointer. Callers create a handle, call exported
//! functions, and finally release it with `bitset_c_free`.
//!
//! ## Why thread-local error state?
//!
//! Several constructors can fail (`from_binary_str`, null pointers, invalid
//! UTF-8, values larger than the local platform can index). Those failures are
//! reported through a familiar FFI pattern:
//!
//! 1. the function returns a sentinel (`NULL`, `0`, or `false`)
//! 2. it stores an error code and message in thread-local state
//! 3. the caller immediately reads that error state
//!
//! This avoids cross-FFI heap ownership while still giving managed callers a
//! useful error message to surface as an exception.

use bitset::{Bitset, BitsetError};
use std::cell::{Cell, RefCell};
use std::ffi::{c_char, CStr, CString};
use std::panic::{self, AssertUnwindSafe};
use std::ptr;

thread_local! {
    static LAST_ERROR_CODE: Cell<u32> = const { Cell::new(0) };
    static LAST_ERROR_MESSAGE: RefCell<Option<CString>> = RefCell::new(None);
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ErrorCode {
    None = 0,
    InvalidBinaryString = 1,
    NullPointer = 2,
    InvalidUtf8 = 3,
    ValueTooLarge = 4,
    Panic = 5,
}

pub struct BitsetHandle {
    inner: Bitset,
}

fn clear_error() {
    LAST_ERROR_CODE.with(|slot| slot.set(ErrorCode::None as u32));
    LAST_ERROR_MESSAGE.with(|slot| *slot.borrow_mut() = None);
}

fn sanitize_message(message: &str) -> CString {
    match CString::new(message) {
        Ok(message) => message,
        Err(_) => CString::new(message.replace('\0', " ")).expect("nul-stripped message must be a valid CString"),
    }
}

fn set_error(code: ErrorCode, message: impl AsRef<str>) {
    LAST_ERROR_CODE.with(|slot| slot.set(code as u32));
    LAST_ERROR_MESSAGE.with(|slot| *slot.borrow_mut() = Some(sanitize_message(message.as_ref())));
}

fn catch_ptr(operation: impl FnOnce() -> *mut BitsetHandle) -> *mut BitsetHandle {
    clear_error();
    match panic::catch_unwind(AssertUnwindSafe(operation)) {
        Ok(result) => result,
        Err(_) => {
            set_error(ErrorCode::Panic, "bitset-c caught a Rust panic before it crossed the C ABI boundary.");
            ptr::null_mut()
        }
    }
}

fn catch_u64(operation: impl FnOnce() -> u64) -> u64 {
    clear_error();
    match panic::catch_unwind(AssertUnwindSafe(operation)) {
        Ok(result) => result,
        Err(_) => {
            set_error(ErrorCode::Panic, "bitset-c caught a Rust panic before it crossed the C ABI boundary.");
            0
        }
    }
}

fn catch_u8(operation: impl FnOnce() -> u8) -> u8 {
    clear_error();
    match panic::catch_unwind(AssertUnwindSafe(operation)) {
        Ok(result) => result,
        Err(_) => {
            set_error(ErrorCode::Panic, "bitset-c caught a Rust panic before it crossed the C ABI boundary.");
            0
        }
    }
}

fn catch_void(operation: impl FnOnce()) {
    clear_error();
    if panic::catch_unwind(AssertUnwindSafe(operation)).is_err() {
        set_error(ErrorCode::Panic, "bitset-c caught a Rust panic before it crossed the C ABI boundary.");
    }
}

fn usize_from_u64(value: u64) -> Result<usize, ()> {
    usize::try_from(value).map_err(|_| {
        set_error(
            ErrorCode::ValueTooLarge,
            format!("value {value} does not fit into this platform's usize."),
        );
    })
}

unsafe fn bitset_ref<'a>(handle: *const BitsetHandle, operation: &str) -> Result<&'a Bitset, ()> {
    unsafe { handle.as_ref() }
        .map(|handle| &handle.inner)
        .ok_or_else(|| {
            set_error(ErrorCode::NullPointer, format!("{operation} received a null bitset handle."));
        })
}

unsafe fn bitset_mut<'a>(handle: *mut BitsetHandle, operation: &str) -> Result<&'a mut Bitset, ()> {
    unsafe { handle.as_mut() }
        .map(|handle| &mut handle.inner)
        .ok_or_else(|| {
            set_error(ErrorCode::NullPointer, format!("{operation} received a null bitset handle."));
        })
}

fn wrap_bitset(bitset: Bitset) -> *mut BitsetHandle {
    Box::into_raw(Box::new(BitsetHandle { inner: bitset }))
}

fn build_u128(low: u64, high: u64) -> u128 {
    ((high as u128) << 64) | (low as u128)
}

fn message_for_bitset_error(error: &BitsetError) -> String {
    match error {
        BitsetError::InvalidBinaryString(value) => format!("invalid binary string: {:?}", value),
    }
}

#[no_mangle]
pub extern "C" fn bitset_c_new(size: u64) -> *mut BitsetHandle {
    catch_ptr(|| {
        usize_from_u64(size)
            .map(|size| wrap_bitset(Bitset::new(size)))
            .unwrap_or(ptr::null_mut())
    })
}

#[no_mangle]
pub extern "C" fn bitset_c_from_u128(low: u64, high: u64) -> *mut BitsetHandle {
    catch_ptr(|| wrap_bitset(Bitset::from_integer(build_u128(low, high))))
}

#[no_mangle]
pub unsafe extern "C" fn bitset_c_from_binary_str(binary: *const c_char) -> *mut BitsetHandle {
    catch_ptr(|| {
        if binary.is_null() {
            set_error(ErrorCode::NullPointer, "bitset_c_from_binary_str requires a non-null string pointer.");
            ptr::null_mut()
        } else {
            let binary = match unsafe { CStr::from_ptr(binary) }.to_str() {
                Ok(binary) => binary,
                Err(_) => {
                    set_error(ErrorCode::InvalidUtf8, "bitset_c_from_binary_str requires valid UTF-8 input.");
                    return ptr::null_mut();
                }
            };

            match Bitset::from_binary_str(binary) {
                Ok(bitset) => wrap_bitset(bitset),
                Err(error) => {
                    set_error(ErrorCode::InvalidBinaryString, message_for_bitset_error(&error));
                    ptr::null_mut()
                }
            }
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn bitset_c_free(handle: *mut BitsetHandle) {
    if !handle.is_null() {
        drop(unsafe { Box::from_raw(handle) });
    }
}

#[no_mangle]
pub unsafe extern "C" fn bitset_c_set(handle: *mut BitsetHandle, index: u64) {
    catch_void(|| {
        if let (Ok(bitset), Ok(index)) = (unsafe { bitset_mut(handle, "bitset_c_set") }, usize_from_u64(index)) {
            bitset.set(index);
        }
    });
}

#[no_mangle]
pub unsafe extern "C" fn bitset_c_clear(handle: *mut BitsetHandle, index: u64) {
    catch_void(|| {
        if let (Ok(bitset), Ok(index)) = (unsafe { bitset_mut(handle, "bitset_c_clear") }, usize_from_u64(index)) {
            bitset.clear(index);
        }
    });
}

#[no_mangle]
pub unsafe extern "C" fn bitset_c_test(handle: *const BitsetHandle, index: u64) -> u8 {
    catch_u8(|| match (unsafe { bitset_ref(handle, "bitset_c_test") }, usize_from_u64(index)) {
        (Ok(bitset), Ok(index)) => u8::from(bitset.test(index)),
        _ => 0,
    })
}

#[no_mangle]
pub unsafe extern "C" fn bitset_c_toggle(handle: *mut BitsetHandle, index: u64) {
    catch_void(|| {
        if let (Ok(bitset), Ok(index)) = (unsafe { bitset_mut(handle, "bitset_c_toggle") }, usize_from_u64(index)) {
            bitset.toggle(index);
        }
    });
}

#[no_mangle]
pub unsafe extern "C" fn bitset_c_and(
    left: *const BitsetHandle,
    right: *const BitsetHandle,
) -> *mut BitsetHandle {
    catch_ptr(|| match (unsafe { bitset_ref(left, "bitset_c_and") }, unsafe { bitset_ref(right, "bitset_c_and") }) {
        (Ok(left), Ok(right)) => wrap_bitset(left.and(right)),
        _ => ptr::null_mut(),
    })
}

#[no_mangle]
pub unsafe extern "C" fn bitset_c_or(
    left: *const BitsetHandle,
    right: *const BitsetHandle,
) -> *mut BitsetHandle {
    catch_ptr(|| match (unsafe { bitset_ref(left, "bitset_c_or") }, unsafe { bitset_ref(right, "bitset_c_or") }) {
        (Ok(left), Ok(right)) => wrap_bitset(left.or(right)),
        _ => ptr::null_mut(),
    })
}

#[no_mangle]
pub unsafe extern "C" fn bitset_c_xor(
    left: *const BitsetHandle,
    right: *const BitsetHandle,
) -> *mut BitsetHandle {
    catch_ptr(|| match (unsafe { bitset_ref(left, "bitset_c_xor") }, unsafe { bitset_ref(right, "bitset_c_xor") }) {
        (Ok(left), Ok(right)) => wrap_bitset(left.xor(right)),
        _ => ptr::null_mut(),
    })
}

#[no_mangle]
pub unsafe extern "C" fn bitset_c_not(handle: *const BitsetHandle) -> *mut BitsetHandle {
    catch_ptr(|| match unsafe { bitset_ref(handle, "bitset_c_not") } {
        Ok(bitset) => wrap_bitset(bitset.not()),
        Err(_) => ptr::null_mut(),
    })
}

#[no_mangle]
pub unsafe extern "C" fn bitset_c_and_not(
    left: *const BitsetHandle,
    right: *const BitsetHandle,
) -> *mut BitsetHandle {
    catch_ptr(|| match (unsafe { bitset_ref(left, "bitset_c_and_not") }, unsafe { bitset_ref(right, "bitset_c_and_not") }) {
        (Ok(left), Ok(right)) => wrap_bitset(left.and_not(right)),
        _ => ptr::null_mut(),
    })
}

#[no_mangle]
pub unsafe extern "C" fn bitset_c_popcount(handle: *const BitsetHandle) -> u64 {
    catch_u64(|| match unsafe { bitset_ref(handle, "bitset_c_popcount") } {
        Ok(bitset) => bitset.popcount() as u64,
        Err(_) => 0,
    })
}

#[no_mangle]
pub unsafe extern "C" fn bitset_c_len(handle: *const BitsetHandle) -> u64 {
    catch_u64(|| match unsafe { bitset_ref(handle, "bitset_c_len") } {
        Ok(bitset) => bitset.len() as u64,
        Err(_) => 0,
    })
}

#[no_mangle]
pub unsafe extern "C" fn bitset_c_capacity(handle: *const BitsetHandle) -> u64 {
    catch_u64(|| match unsafe { bitset_ref(handle, "bitset_c_capacity") } {
        Ok(bitset) => bitset.capacity() as u64,
        Err(_) => 0,
    })
}

#[no_mangle]
pub unsafe extern "C" fn bitset_c_any(handle: *const BitsetHandle) -> u8 {
    catch_u8(|| match unsafe { bitset_ref(handle, "bitset_c_any") } {
        Ok(bitset) => u8::from(bitset.any()),
        Err(_) => 0,
    })
}

#[no_mangle]
pub unsafe extern "C" fn bitset_c_all(handle: *const BitsetHandle) -> u8 {
    catch_u8(|| match unsafe { bitset_ref(handle, "bitset_c_all") } {
        Ok(bitset) => u8::from(bitset.all()),
        Err(_) => 0,
    })
}

#[no_mangle]
pub unsafe extern "C" fn bitset_c_none(handle: *const BitsetHandle) -> u8 {
    catch_u8(|| match unsafe { bitset_ref(handle, "bitset_c_none") } {
        Ok(bitset) => u8::from(bitset.none()),
        Err(_) => 0,
    })
}

#[no_mangle]
pub unsafe extern "C" fn bitset_c_is_empty(handle: *const BitsetHandle) -> u8 {
    catch_u8(|| match unsafe { bitset_ref(handle, "bitset_c_is_empty") } {
        Ok(bitset) => u8::from(bitset.is_empty()),
        Err(_) => 0,
    })
}

#[no_mangle]
pub unsafe extern "C" fn bitset_c_to_u64(handle: *const BitsetHandle, out: *mut u64) -> u8 {
    catch_u8(|| {
        if out.is_null() {
            set_error(ErrorCode::NullPointer, "bitset_c_to_u64 requires a non-null output pointer.");
            0
        } else {
            match unsafe { bitset_ref(handle, "bitset_c_to_u64") } {
                Ok(bitset) => match bitset.to_integer() {
                    Some(value) => {
                        unsafe { *out = value };
                        1
                    }
                    None => 0,
                },
                Err(_) => 0,
            }
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn bitset_c_equals(
    left: *const BitsetHandle,
    right: *const BitsetHandle,
) -> u8 {
    catch_u8(|| match (unsafe { bitset_ref(left, "bitset_c_equals") }, unsafe { bitset_ref(right, "bitset_c_equals") }) {
        (Ok(left), Ok(right)) => u8::from(left == right),
        _ => 0,
    })
}

#[no_mangle]
pub extern "C" fn bitset_c_had_error() -> u8 {
    LAST_ERROR_CODE.with(|slot| u8::from(slot.get() != ErrorCode::None as u32))
}

#[no_mangle]
pub extern "C" fn bitset_c_last_error_code() -> u32 {
    LAST_ERROR_CODE.with(|slot| slot.get())
}

#[no_mangle]
pub extern "C" fn bitset_c_last_error_message() -> *const c_char {
    LAST_ERROR_MESSAGE.with(|slot| {
        slot.borrow()
            .as_ref()
            .map(|message| message.as_ptr())
            .unwrap_or(ptr::null())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn error_message() -> String {
        let ptr = bitset_c_last_error_message();
        if ptr.is_null() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(ptr) }.to_str().unwrap().to_string()
        }
    }

    #[test]
    fn creates_mutates_and_queries_handles() {
        unsafe {
            let bitset = bitset_c_new(10);
            assert!(!bitset.is_null());
            assert_eq!(bitset_c_len(bitset), 10);
            assert_eq!(bitset_c_capacity(bitset), 64);
            assert_eq!(bitset_c_popcount(bitset), 0);
            assert_eq!(bitset_c_none(bitset), 1);

            bitset_c_set(bitset, 5);
            assert_eq!(bitset_c_test(bitset, 5), 1);
            assert_eq!(bitset_c_popcount(bitset), 1);

            bitset_c_toggle(bitset, 5);
            assert_eq!(bitset_c_test(bitset, 5), 0);

            bitset_c_toggle(bitset, 80);
            assert_eq!(bitset_c_len(bitset), 81);
            assert_eq!(bitset_c_capacity(bitset), 128);
            assert_eq!(bitset_c_test(bitset, 80), 1);

            bitset_c_free(bitset);
        }
    }

    #[test]
    fn bulk_operations_match_reference_values() {
        unsafe {
            let left = bitset_c_from_u128(0b1100, 0);
            let right = bitset_c_from_u128(0b1010, 0);

            let and_result = bitset_c_and(left, right);
            let or_result = bitset_c_or(left, right);
            let xor_result = bitset_c_xor(left, right);
            let and_not_result = bitset_c_and_not(left, right);

            let mut value = 0;
            assert_eq!(bitset_c_to_u64(and_result, &mut value), 1);
            assert_eq!(value, 0b1000);

            assert_eq!(bitset_c_to_u64(or_result, &mut value), 1);
            assert_eq!(value, 0b1110);

            assert_eq!(bitset_c_to_u64(xor_result, &mut value), 1);
            assert_eq!(value, 0b0110);

            assert_eq!(bitset_c_to_u64(and_not_result, &mut value), 1);
            assert_eq!(value, 0b0100);

            bitset_c_free(and_result);
            bitset_c_free(or_result);
            bitset_c_free(xor_result);
            bitset_c_free(and_not_result);
            bitset_c_free(left);
            bitset_c_free(right);
        }
    }

    #[test]
    fn from_binary_string_reports_invalid_input() {
        unsafe {
            let invalid = CString::new("10x1").unwrap();
            let bitset = bitset_c_from_binary_str(invalid.as_ptr());
            assert!(bitset.is_null());
            assert_eq!(bitset_c_had_error(), 1);
            assert_eq!(bitset_c_last_error_code(), ErrorCode::InvalidBinaryString as u32);
            assert!(error_message().contains("invalid binary string"));
        }
    }

    #[test]
    fn round_trips_split_u128_and_reports_u64_overflow() {
        unsafe {
            let bitset = bitset_c_from_u128(1, 1);
            assert!(!bitset.is_null());
            assert_eq!(bitset_c_len(bitset), 65);
            assert_eq!(bitset_c_test(bitset, 0), 1);
            assert_eq!(bitset_c_test(bitset, 64), 1);

            let mut value = 0;
            assert_eq!(bitset_c_to_u64(bitset, &mut value), 0);

            bitset_c_free(bitset);
        }
    }

    #[test]
    fn null_handles_set_error_state() {
        unsafe {
            assert_eq!(bitset_c_test(ptr::null(), 0), 0);
            assert_eq!(bitset_c_had_error(), 1);
            assert_eq!(bitset_c_last_error_code(), ErrorCode::NullPointer as u32);
            assert!(error_message().contains("null bitset handle"));
        }
    }
}
