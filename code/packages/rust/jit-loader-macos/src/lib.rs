//! # `jit-loader-macos` — runtime code installation for Apple Silicon.
//!
//! Allocates a page of executable memory, copies machine-code bytes into
//! it, and exposes a function pointer that can be `transmute`'d to an
//! `extern "C"` Rust function and called.  Used by `jit-core` /
//! `aarch64-backend` to install code generated at runtime so the
//! interpreter can dispatch to native versions of hot functions.
//!
//! ## The W^X dance
//!
//! macOS 11+ on Apple Silicon enforces W^X (write *xor* execute) on JIT
//! pages.  Pages allocated with `MAP_JIT` start in **executable** mode;
//! the per-thread sysctl `pthread_jit_write_protect_np` toggles between
//! "currently writable" (`enable = 0`) and "currently executable"
//! (`enable = 1`).  After writing code we must:
//!
//! 1. Flip back to executable: `pthread_jit_write_protect_np(1)`.
//! 2. Invalidate the instruction cache for the freshly-written range:
//!    `sys_icache_invalidate(ptr, len)` — without this, the CPU may
//!    execute stale bytes from its I-cache.
//!
//! ## Entitlements
//!
//! For local development the `MAP_JIT` flag works without any code-
//! signing entitlement.  Distributed apps need the
//! `com.apple.security.cs.allow-jit` entitlement and the
//! hardened-runtime — but that is the consumer's concern, not this
//! crate's.
//!
//! ## Thread-safety contract
//!
//! `pthread_jit_write_protect_np` is **per-thread** state.  A call from
//! thread A does not affect thread B.  This matters for multi-threaded
//! JIT: each thread that writes new code must flip its own protection
//! flag.  The struct exposed here is `Send` (you can move it across
//! threads) but writing from a different thread than the one that
//! constructed it is undefined unless that thread first calls
//! `pthread_jit_write_protect_np(0)` itself.  In practice JIT in this
//! repo is single-threaded.
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use jit_loader_macos::CodePage;
//!
//! // ARM64 machine code: `mov x0, #42; ret`
//! let bytes: [u8; 8] = [
//!     0x40, 0x05, 0x80, 0xD2,  // movz x0, #42
//!     0xC0, 0x03, 0x5F, 0xD6,  // ret
//! ];
//! let page = CodePage::new(&bytes).expect("install code");
//! let f: extern "C" fn() -> u64 = unsafe { page.as_function() };
//! assert_eq!(f(), 42);
//! ```

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]
#![cfg(all(target_os = "macos", target_arch = "aarch64"))]

use std::ffi::c_void;
use std::ptr;

// ---------------------------------------------------------------------------
// FFI declarations — minimal subset of <sys/mman.h>, <pthread.h>, <libkern/OSCacheControl.h>
// ---------------------------------------------------------------------------
//
// Declared inline rather than depending on the `libc` crate so this crate
// has zero non-workspace dependencies.

const PROT_READ:  i32 = 0x1;
const PROT_WRITE: i32 = 0x2;
const PROT_EXEC:  i32 = 0x4;

const MAP_PRIVATE: i32 = 0x0002;
const MAP_ANON:    i32 = 0x1000;
/// macOS-specific JIT-mapping flag.  Without this, modern macOS will
/// reject `PROT_WRITE | PROT_EXEC` on the same page outright.
const MAP_JIT:     i32 = 0x0800;
const MAP_FAILED:  *mut c_void = !0usize as *mut c_void;

/// Page size used for JIT allocations.  Apple Silicon has 16 KiB pages;
/// allocate at that granularity to match.
const PAGE_SIZE: usize = 16 * 1024;

#[link(name = "c")]
extern "C" {
    fn mmap(addr: *mut c_void, len: usize, prot: i32, flags: i32, fd: i32, offset: i64) -> *mut c_void;
    fn munmap(addr: *mut c_void, len: usize) -> i32;
}

#[link(name = "pthread")]
extern "C" {
    /// Toggle the calling thread's W^X protection on `MAP_JIT` pages.
    /// `enable == 0` → pages are writable.  `enable == 1` → pages are
    /// executable.
    fn pthread_jit_write_protect_np(enable: i32);
}

#[link(name = "System", kind = "dylib")]
extern "C" {
    /// Flush the I-cache for `[start, start + len)` so the CPU sees the
    /// freshly-written bytes.
    fn sys_icache_invalidate(start: *const c_void, len: usize);
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Failure modes for code-page installation.
#[derive(Debug)]
#[allow(dead_code)]
pub enum LoaderError {
    /// `mmap` returned `MAP_FAILED`.  Most often: the kernel's JIT
    /// quota for the process is exhausted, or the address space is
    /// fragmented.  The wrapped `errno` is reported.
    MmapFailed { errno: i32 },
    /// Provided code is empty.  Calling a 0-byte function is a bug.
    EmptyCode,
}

impl std::fmt::Display for LoaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoaderError::MmapFailed { errno } =>
                write!(f, "mmap MAP_JIT failed: errno={errno}"),
            LoaderError::EmptyCode =>
                write!(f, "cannot install zero bytes of code"),
        }
    }
}

impl std::error::Error for LoaderError {}

// ---------------------------------------------------------------------------
// CodePage
// ---------------------------------------------------------------------------

/// A page of executable memory holding machine code installed at
/// runtime.
///
/// Constructing a `CodePage` allocates a 16-KiB page via
/// `mmap(MAP_JIT)`, writes the supplied bytes into it (under W^X
/// protection toggling), and flushes the instruction cache.  On
/// `Drop` the page is `munmap`'d.
///
/// The `entry` pointer is the start of the page — i.e. the first
/// instruction the function-call ABI expects.  Cast via
/// [`Self::as_function`] to a typed `extern "C"` fn pointer.
pub struct CodePage {
    base: *mut u8,
    len:  usize,
}

// `*mut u8` is !Send by default; CodePage is logically owned and the
// FFI calls are thread-safe, so we manually mark it Send.  Sync is NOT
// granted: concurrent writes from multiple threads would race the W^X
// flag.
unsafe impl Send for CodePage {}

impl CodePage {
    /// Allocate a fresh page, copy `code` into it, and prepare it for
    /// execution.
    ///
    /// Returns `Err(EmptyCode)` if `code` is empty (a 0-byte function
    /// is undefined behaviour to call), or `Err(MmapFailed)` if the
    /// kernel rejects the allocation.
    pub fn new(code: &[u8]) -> Result<Self, LoaderError> {
        if code.is_empty() {
            return Err(LoaderError::EmptyCode);
        }

        // Allocate at page granularity.  16 KiB is the Apple Silicon
        // page size; smaller allocations get rounded up by the kernel
        // anyway, but we make it explicit so `len` matches `munmap`'s
        // accounting.
        let len = round_up(code.len(), PAGE_SIZE);

        // SAFETY: `mmap` with `MAP_ANON` does not read from the
        // supplied `addr`/`fd`; we pass null/-1 by convention.
        let base = unsafe {
            mmap(
                ptr::null_mut(),
                len,
                PROT_READ | PROT_WRITE | PROT_EXEC,
                MAP_PRIVATE | MAP_ANON | MAP_JIT,
                -1,
                0,
            )
        };
        if base == MAP_FAILED {
            // SAFETY: `errno` is just an integer read.
            let errno = unsafe { *errno_location() };
            return Err(LoaderError::MmapFailed { errno });
        }

        let page = CodePage { base: base as *mut u8, len };
        page.write_code(code);
        page.flush_icache();
        Ok(page)
    }

    /// Internal: copy `code` into the page while W^X says "writable".
    fn write_code(&self, code: &[u8]) {
        // SAFETY: this is the canonical macOS JIT-write sequence.
        // `pthread_jit_write_protect_np(0)` is only legal for pages
        // mapped with `MAP_JIT` and is per-thread; we set it from the
        // same thread that allocated the page, immediately before
        // writing.
        unsafe {
            pthread_jit_write_protect_np(0);
            ptr::copy_nonoverlapping(code.as_ptr(), self.base, code.len());
            pthread_jit_write_protect_np(1);
        }
    }

    /// Internal: flush the instruction cache for the freshly-written
    /// range so the CPU executes the new bytes, not stale ones.
    fn flush_icache(&self) {
        // SAFETY: `sys_icache_invalidate` reads no memory; it just
        // issues `ic ivau` instructions on each cache line in the
        // range.  Safe even if `len > code.len()` (the trailing
        // unused bytes are zeroed by `mmap` and harmless to flush).
        unsafe { sys_icache_invalidate(self.base as *const c_void, self.len); }
    }

    /// Pointer to the first byte of the installed code.  Cast via
    /// [`Self::as_function`] to call.
    pub fn entry(&self) -> *const u8 { self.base }

    /// Total bytes mapped (page-rounded).
    pub fn len(&self) -> usize { self.len }

    /// `true` if the page contains no code (impossible to construct
    /// such a page via `new`; provided for completeness).
    pub fn is_empty(&self) -> bool { self.len == 0 }

    /// Cast the page entry to a typed `extern "C"` function pointer.
    ///
    /// # Safety
    ///
    /// The caller is responsible for ensuring `F` matches the actual
    /// AAPCS64 signature of the installed code.  A mismatch (wrong
    /// arg count, wrong types, wrong return type) is undefined
    /// behaviour.  In this repo the contract is: the code was
    /// produced by `aarch64-backend::compile_function` against an
    /// `IIRFunction` whose `params` and `return_type` match `F`.
    pub unsafe fn as_function<F: Copy>(&self) -> F {
        debug_assert!(
            std::mem::size_of::<F>() == std::mem::size_of::<*const ()>(),
            "F must be a function pointer (same size as *const ())"
        );
        // SAFETY: caller's contract per the doc above.
        std::mem::transmute_copy(&self.base)
    }
}

impl Drop for CodePage {
    fn drop(&mut self) {
        // SAFETY: `base` came from a successful `mmap` with `len`
        // bytes; `munmap` is always safe with those exact arguments.
        // We ignore the return value — there is nothing useful to do
        // with `munmap` failure in `Drop`.
        unsafe { let _ = munmap(self.base as *mut c_void, self.len); }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn round_up(n: usize, align: usize) -> usize {
    (n + align - 1) & !(align - 1)
}

#[cfg(target_os = "macos")]
extern "C" {
    /// Per-thread `errno` slot on Darwin.  Safe to read after a failing
    /// libc call.
    fn __error() -> *const i32;
}

unsafe fn errno_location() -> *const i32 {
    __error()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Hand-encoded ARM64 for `fn() -> u64 { 42 }`:
    ///   movz x0, #42      → 0xD2800540
    ///   ret               → 0xD65F03C0
    fn return_42_bytes() -> [u8; 8] {
        let mut out = [0u8; 8];
        out[0..4].copy_from_slice(&0xD28005_40u32.to_le_bytes());
        out[4..8].copy_from_slice(&0xD65F03C0u32.to_le_bytes());
        out
    }

    /// Hand-encoded ARM64 for `fn(a: u64, b: u64) -> u64 { a + b }`:
    ///   add x0, x0, x1    → 0x8B010000
    ///   ret               → 0xD65F03C0
    fn add_two_args_bytes() -> [u8; 8] {
        let mut out = [0u8; 8];
        out[0..4].copy_from_slice(&0x8B010000u32.to_le_bytes());
        out[4..8].copy_from_slice(&0xD65F03C0u32.to_le_bytes());
        out
    }

    #[test]
    fn empty_code_rejected() {
        assert!(matches!(CodePage::new(&[]), Err(LoaderError::EmptyCode)));
    }

    #[test]
    fn page_constructs_for_simple_code() {
        let _page = CodePage::new(&return_42_bytes()).expect("ok");
    }

    #[test]
    fn page_round_trips_42() {
        let page = CodePage::new(&return_42_bytes()).unwrap();
        let f: extern "C" fn() -> u64 = unsafe { page.as_function() };
        assert_eq!(f(), 42);
    }

    #[test]
    fn page_round_trips_addition() {
        let page = CodePage::new(&add_two_args_bytes()).unwrap();
        let f: extern "C" fn(u64, u64) -> u64 = unsafe { page.as_function() };
        assert_eq!(f(7, 35), 42);
        assert_eq!(f(100, 200), 300);
        assert_eq!(f(0, 0), 0);
    }

    #[test]
    fn page_can_be_called_repeatedly() {
        let page = CodePage::new(&return_42_bytes()).unwrap();
        let f: extern "C" fn() -> u64 = unsafe { page.as_function() };
        for _ in 0..1000 {
            assert_eq!(f(), 42);
        }
    }

    #[test]
    fn drop_releases_memory() {
        // Construct + drop a few thousand times; if munmap leaks, the
        // process's JIT quota fills up and later allocations fail.
        for _ in 0..1000 {
            let _ = CodePage::new(&return_42_bytes()).unwrap();
        }
    }

    #[test]
    fn entry_pointer_is_page_aligned() {
        let page = CodePage::new(&return_42_bytes()).unwrap();
        let addr = page.entry() as usize;
        assert_eq!(addr & (PAGE_SIZE - 1), 0, "entry must be page-aligned");
    }

    #[test]
    fn page_len_is_at_least_one_page() {
        let page = CodePage::new(&return_42_bytes()).unwrap();
        assert!(page.len() >= PAGE_SIZE);
    }
}
