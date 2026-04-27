//! # coding_adventures_zeroize — secure in-memory wiping for secrets
//!
//! ## Why this exists
//!
//! When a program writes a secret (a password, a master key, a session token)
//! into a chunk of RAM and later "clears" it by assigning `0`, an optimising
//! compiler is allowed to delete the clear. From the compiler's point of view,
//! nothing reads the buffer afterwards, so the store is "dead" and can be
//! eliminated as a pointless write. The secret then stays in RAM until the
//! page is reused — which is a problem if:
//!
//!   * the machine is later swapped or hibernated to disk,
//!   * the process gets core-dumped,
//!   * another process manages to read freed pages (cold-boot, sidechannel,
//!     untrusted debugger, VM snapshot, etc.),
//!   * an attacker can inspect freed heap memory through a use-after-free in
//!     unrelated code.
//!
//! To make the clear **observably happen**, the write has to be a *volatile*
//! store (so the compiler is forbidden from removing it) and it has to be
//! fenced (so it cannot be reordered past the end of the buffer's lifetime).
//!
//! This crate is a tiny, from-scratch implementation of exactly that. It is
//! intended for:
//!
//!   * the D18 Chief-of-Staff Vault's master key and derived-key slots,
//!   * the `cmk` (channel master key) and per-receiver session keys in the
//!     messaging channel primitive,
//!   * anything the Vault hands out as a short-lived lease.
//!
//! ## What it is NOT
//!
//! * **Not a swap/paging defence.** On a machine where the page has already
//!   been paged out, zeroing the in-RAM copy does nothing about the on-disk
//!   copy. Callers that care about that must `mlock` the buffer separately
//!   (the Vault does this at the OS-container level).
//! * **Not a defence against a currently-attached attacker.** An attacker who
//!   can read the process's memory while the secret is alive can read it
//!   before `zeroize()` runs. Zeroization shrinks the *window*, it does not
//!   remove it.
//! * **Not constant-time-comparison.** For equality on tags use a dedicated
//!   constant-time compare (see the sibling crate).
//!
//! ## How the guarantee is constructed
//!
//! ```text
//!   for each byte of the buffer:
//!       core::ptr::write_volatile(p, 0)   // compiler MUST emit this store
//!   core::sync::atomic::compiler_fence(SeqCst)   // no reordering past here
//! ```
//!
//! * `write_volatile` is the standard-library primitive for a store the
//!   compiler is forbidden from eliminating, even if no later read is visible
//!   to it.
//! * `compiler_fence(SeqCst)` is a pure optimiser barrier (no CPU instruction)
//!   that tells LLVM that nothing on one side of it may move to the other
//!   side. We use it so the zero-stores can't be reordered past the `Drop`
//!   boundary and out of the buffer's lifetime.
//!
//! We deliberately use `compiler_fence` (compiler-only), not `fence`
//! (hardware SeqCst). The defence here is against the *compiler*, not other
//! cores — a foreign core that can read process memory concurrently is
//! outside the threat model.
//!
//! ## The `Zeroize` trait and `Zeroizing<T>` wrapper
//!
//! `Zeroize` is implemented for byte slices, fixed-size byte arrays, byte
//! vectors, `String`, and all the fixed-width unsigned integers. `Zeroizing<T>`
//! is a thin newtype whose `Drop` calls `.zeroize()` on the inner value. That
//! means:
//!
//! ```ignore
//! {
//!     let key = Zeroizing::new([0u8; 32].map(|_| 0x42));
//!     // ... use `key` ...
//! } // <-- drop runs here, key bytes are wiped before the stack slot is reused
//! ```
//!
//! A container that owns a secret should hold it as `Zeroizing<…>` so that
//! every normal exit path — including panics — wipes the secret automatically.

#![deny(unsafe_op_in_unsafe_fn)]

use core::ptr;
use core::sync::atomic::{compiler_fence, Ordering};

// === Section 1. The low-level primitive ======================================

/// Overwrite every byte of `slice` with 0 using volatile stores.
///
/// The volatile annotation keeps the compiler from optimising the clear
/// away when it proves no later read will observe the zero. The trailing
/// compiler fence keeps the store from being reordered past the end of
/// the caller's lifetime.
///
/// This is the primitive everything else in the crate is built on.
pub fn zeroize_bytes(slice: &mut [u8]) {
    let ptr = slice.as_mut_ptr();
    let len = slice.len();
    // SAFETY: `ptr` points to `len` valid, properly aligned, mutably
    // borrowed bytes for the lifetime of the borrow. A byte is always
    // aligned for any address. The loop writes within the borrow's range,
    // and no other reference to these bytes exists while the borrow is live.
    unsafe {
        for i in 0..len {
            ptr::write_volatile(ptr.add(i), 0u8);
        }
    }
    compiler_fence(Ordering::SeqCst);
}

// === Section 2. The Zeroize trait ===========================================

/// Types that can be safely and observably scrubbed in place.
///
/// Implementors are required to end the operation with a compiler fence
/// (or to delegate to something that does, such as `zeroize_bytes`).
pub trait Zeroize {
    /// Overwrite the value with a zero/default representation the caller
    /// has no way to distinguish from a freshly zero-initialised value.
    fn zeroize(&mut self);
}

impl Zeroize for [u8] {
    fn zeroize(&mut self) {
        zeroize_bytes(self);
    }
}

impl<const N: usize> Zeroize for [u8; N] {
    fn zeroize(&mut self) {
        zeroize_bytes(&mut self[..]);
    }
}

impl Zeroize for Vec<u8> {
    /// Wipe the buffer, then clear the logical length to 0.
    ///
    /// We deliberately zero the entire **allocated capacity**, not just the
    /// live-element prefix. A `Vec<u8>` grows by copying its bytes into a
    /// larger allocation and leaving the old allocation to the allocator; by
    /// the time we zeroize, there may still be stale secret material in the
    /// unused tail of the current allocation. Walking over `capacity()` bytes
    /// scrubs that tail too.
    fn zeroize(&mut self) {
        let cap = self.capacity();
        let ptr = self.as_mut_ptr();
        // SAFETY: `ptr` is the Vec's heap allocation, valid for `cap` bytes.
        // For any `u8`, every byte pattern is a valid value and every address
        // is aligned, so writing `0u8` to each capacity byte is sound. We
        // never read uninitialised bytes: we only write.
        unsafe {
            for i in 0..cap {
                ptr::write_volatile(ptr.add(i), 0u8);
            }
        }
        compiler_fence(Ordering::SeqCst);
        self.clear();
    }
}

impl Zeroize for String {
    /// Wipe the string's byte buffer, then truncate to empty.
    ///
    /// Same reasoning as `Vec<u8>`: we scrub the full capacity, not just
    /// the UTF-8 length.
    fn zeroize(&mut self) {
        // SAFETY: we never observe the bytes after zeroing; we immediately
        // call `clear()` (which sets len = 0) before handing the `String`
        // back to safe code. Between the zeroing and the clear the string's
        // bytes are still a valid UTF-8 encoding (all-zero bytes are a
        // sequence of U+0000 code points), so no invariant is temporarily
        // broken.
        let bytes = unsafe { self.as_mut_vec() };
        bytes.zeroize();
    }
}

macro_rules! zeroize_int {
    ($($t:ty),*) => {
        $(
            impl Zeroize for $t {
                fn zeroize(&mut self) {
                    // SAFETY: `self` is a valid, properly aligned pointer to
                    // a single initialised `$t`. Every byte pattern (and the
                    // all-zero pattern in particular) is a valid bit pattern
                    // for a primitive integer.
                    unsafe {
                        ptr::write_volatile(self as *mut $t, 0 as $t);
                    }
                    compiler_fence(Ordering::SeqCst);
                }
            }
        )*
    };
}

zeroize_int!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);

impl<T: Zeroize> Zeroize for Option<T> {
    /// Zeroize the inner value if present, then set the option to `None`.
    ///
    /// Setting to `None` is a separate step because, in Rust, an `Option<T>`
    /// in the `Some(…)` variant and an `Option<T>` in the `None` variant
    /// are *different* bit patterns in general. We zero the payload through
    /// its own `Zeroize` impl (so the wipe is a volatile store the compiler
    /// cannot elide) and then change the variant discriminant to `None`.
    fn zeroize(&mut self) {
        if let Some(inner) = self.as_mut() {
            inner.zeroize();
        }
        *self = None;
    }
}

// === Section 3. The `Zeroizing<T>` RAII wrapper =============================

/// An owning wrapper whose `Drop` zeroizes the contained value.
///
/// Use this for any local holding a secret. It is deliberately the only
/// public RAII helper in this crate — one wrapper, one job.
///
/// ## Panic safety
///
/// `Drop::drop` runs during unwinding, so even a panic inside the protected
/// block triggers the wipe.
pub struct Zeroizing<T: Zeroize> {
    inner: T,
}

impl<T: Zeroize> Zeroizing<T> {
    /// Take ownership of `value` and wipe it on drop.
    pub fn new(value: T) -> Self {
        Self { inner: value }
    }

    /// Consume the wrapper and return the inner value *without* zeroizing.
    ///
    /// This is the escape hatch for cases where the secret must outlive the
    /// wrapper (e.g. handing it to a callee that takes `Vec<u8>` by value).
    /// The caller then owns the wipe-on-drop responsibility.
    pub fn into_inner(mut self) -> T
    where
        T: Default,
    {
        // Swap `inner` with a zero-ish default before skipping Drop. We
        // *don't* zeroize, because the caller is explicitly opting out.
        let out = core::mem::take(&mut self.inner);
        core::mem::forget(self);
        out
    }
}

impl<T: Zeroize> Drop for Zeroizing<T> {
    fn drop(&mut self) {
        self.inner.zeroize();
    }
}

impl<T: Zeroize> core::ops::Deref for Zeroizing<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.inner
    }
}

impl<T: Zeroize> core::ops::DerefMut for Zeroizing<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

// Do NOT implement `Debug`, `Display`, `Clone` by default. Printing or
// duplicating secret material is exactly the kind of mistake this wrapper
// exists to prevent.

// === Section 4. Tests =======================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_slice_is_zeroed() {
        let mut buf = [0xAAu8; 32];
        zeroize_bytes(&mut buf);
        assert_eq!(buf, [0u8; 32]);
    }

    #[test]
    fn empty_slice_is_a_noop() {
        let mut empty: [u8; 0] = [];
        zeroize_bytes(&mut empty);
        // Just confirm no panic, no UB; there's nothing to assert.
    }

    #[test]
    fn fixed_array_trait_impl() {
        let mut arr: [u8; 16] = [0xFF; 16];
        arr.zeroize();
        assert_eq!(arr, [0u8; 16]);
    }

    #[test]
    fn byte_slice_trait_impl() {
        let mut owned = [0x55u8; 64];
        owned[..].zeroize();
        assert!(owned.iter().all(|&b| b == 0));
    }

    #[test]
    fn vec_zeroize_clears_length_and_bytes() {
        let mut v: Vec<u8> = vec![0x11; 10];
        v.reserve_exact(22); // force capacity >= 32
        let cap_before = v.capacity();
        // Re-view the underlying allocation to check the capacity bytes too.
        let ptr = v.as_ptr();
        v.zeroize();
        assert_eq!(v.len(), 0);
        // SAFETY: the Vec still owns its allocation with capacity cap_before.
        // We read back through the raw pointer and expect all zeros. The
        // capacity is unchanged by `zeroize` (we only clear length).
        unsafe {
            for i in 0..cap_before {
                assert_eq!(
                    ptr::read_volatile(ptr.add(i)),
                    0,
                    "byte {} not zeroed",
                    i
                );
            }
        }
    }

    #[test]
    fn string_zeroize_clears_length_and_bytes() {
        let mut s = String::from("hunter2");
        s.reserve(32);
        let cap = s.capacity();
        let ptr = s.as_ptr();
        s.zeroize();
        assert!(s.is_empty());
        // SAFETY: the String still owns its allocation with the same
        // capacity. We read the raw bytes and expect zeros.
        unsafe {
            for i in 0..cap {
                assert_eq!(ptr::read_volatile(ptr.add(i)), 0, "byte {} not zeroed", i);
            }
        }
    }

    #[test]
    fn integer_zeroize() {
        let mut x: u64 = 0xDEAD_BEEF_CAFE_F00D;
        x.zeroize();
        assert_eq!(x, 0);

        let mut y: i32 = -12345;
        y.zeroize();
        assert_eq!(y, 0);

        let mut z: u128 = u128::MAX;
        z.zeroize();
        assert_eq!(z, 0);
    }

    #[test]
    fn option_zeroize_sets_none_and_wipes_payload() {
        let mut opt: Option<[u8; 8]> = Some([0xAA; 8]);
        opt.zeroize();
        assert_eq!(opt, None);

        let mut none: Option<u64> = None;
        none.zeroize();
        assert_eq!(none, None);
    }

    #[test]
    fn zeroizing_wraps_and_derefs() {
        let key = Zeroizing::new([0x42u8; 32]);
        // Deref read-through.
        assert_eq!(key[0], 0x42);
        assert_eq!(key.len(), 32);
        // Deref_mut: we can mutate the inner value.
        let mut key2 = Zeroizing::new([0u8; 4]);
        key2[0] = 1;
        key2[1] = 2;
        assert_eq!(&key2[..], &[1, 2, 0, 0]);
    }

    /// A small helper whose `zeroize()` delegates to a borrowed byte slice.
    /// This lets a test own the observable buffer separately from the
    /// `Zeroizing<…>` wrapper, so we can read the buffer's contents
    /// *after* the wrapper has dropped without any use-after-free.
    struct BorrowedZeroizer<'a> {
        buf: &'a mut [u8],
    }

    impl<'a> Zeroize for BorrowedZeroizer<'a> {
        fn zeroize(&mut self) {
            self.buf.zeroize();
        }
    }

    #[test]
    fn zeroizing_drop_wipes_observable_buffer() {
        // The caller-owned buffer outlives the wrapper, so inspecting it
        // post-Drop is sound.
        let mut owned = [0xCDu8; 48];
        {
            let _guard = Zeroizing::new(BorrowedZeroizer {
                buf: &mut owned[..],
            });
            // _guard drops at end of scope, triggering the wipe via
            // BorrowedZeroizer::zeroize → [u8]::zeroize → zeroize_bytes.
        }
        assert!(owned.iter().all(|&b| b == 0));
    }

    #[test]
    fn zeroizing_into_inner_opts_out_of_wipe() {
        let key = Zeroizing::new([0x77u8; 16]);
        let taken = key.into_inner();
        // The caller now holds the live bytes — `into_inner` deliberately
        // skipped the zeroize.
        assert_eq!(taken, [0x77u8; 16]);
    }

    #[test]
    fn zeroizing_runs_on_panic_unwind() {
        use std::panic::{catch_unwind, AssertUnwindSafe};

        // Stash a caller-owned buffer whose &mut borrow is handed to a
        // Zeroizing guard inside a panicking block. After the unwind the
        // &mut borrow is gone and `owned` is usable again; we assert the
        // wipe ran.
        let mut owned = [0xABu8; 24];
        let result = catch_unwind(AssertUnwindSafe(|| {
            let _guard = Zeroizing::new(BorrowedZeroizer {
                buf: &mut owned[..],
            });
            panic!("simulated mid-operation failure");
        }));
        assert!(result.is_err(), "panic did not propagate");
        assert!(
            owned.iter().all(|&b| b == 0),
            "Zeroizing::drop did not run during unwind"
        );
    }

    #[test]
    fn vec_capacity_bytes_are_scrubbed_beyond_len() {
        // Build a Vec where len < capacity, poison the capacity tail via a
        // detour through raw pointers, then zeroize and verify the tail is
        // also zero.
        let mut v: Vec<u8> = Vec::with_capacity(64);
        v.extend_from_slice(&[0xFF; 16]);
        assert!(v.capacity() >= 64);

        // Poison the tail directly.
        let ptr = v.as_mut_ptr();
        let cap = v.capacity();
        // SAFETY: writing 0xAA into the uninitialised capacity tail. Every
        // byte pattern is a valid `u8`, so this leaves the buffer in a
        // valid state even though we won't read it through the Vec's
        // logical `len`.
        unsafe {
            for i in 16..cap {
                ptr::write_volatile(ptr.add(i), 0xAAu8);
            }
        }

        v.zeroize();
        // After zeroize, the whole capacity window must be zero.
        unsafe {
            for i in 0..cap {
                assert_eq!(ptr::read_volatile(ptr.add(i)), 0, "byte {} not scrubbed", i);
            }
        }
    }
}
