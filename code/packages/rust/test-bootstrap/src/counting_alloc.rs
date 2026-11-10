//! # Counting allocator: measuring the heap instead of trusting the closure
//!
//! The first `AllocCounter` (VER07 §8, audit finding LH-003a) was
//! *cooperative*: it counted only the allocations a benchmark closure
//! remembered to report. A closure that never reported anything allocated
//! "zero bytes", so every `max_alloc_bytes = 0` budget passed vacuously. This
//! module replaces that with a real measurement: a [`GlobalAlloc`] wrapper
//! that sees every heap request the process makes.
//!
//! ## How it works
//!
//! ```text
//!   Vec::with_capacity(1) ──► CountingAllocator::alloc ──► System.alloc
//!                                    │
//!                                    └─► thread-local tally += (1 allocation, 1 byte)
//! ```
//!
//! [`CountingAllocator`] only *delegates* to [`std::alloc::System`]; the
//! memory it hands out is exactly what the system allocator would have
//! handed out. The extra work is bumping four plain integers.
//!
//! ## Why thread-local tallies (and no measurement lock)
//!
//! * A benchmark closure runs on one thread, so a per-thread tally sees all
//!   of its allocations and none of anybody else's. Another test thread
//!   allocating at the same moment cannot disturb a measurement, so there is
//!   nothing to serialise.
//! * The tally is a `const`-initialised `Cell` of a `Copy` type with no
//!   destructor. Such a thread-local needs no lazy registration, so reading
//!   or bumping it inside the allocator never allocates (which would recurse
//!   forever) and never panics during thread teardown.
//!
//! ## What it cannot see (documented limits)
//!
//! * Allocations made by *other threads* the closure spawns are not counted.
//! * Memory that does not go through Rust's global allocator (`mmap` by a C
//!   library, the stack) is not counted.
//! * `realloc` is counted conservatively as one free of the old block plus
//!   one allocation of the new size, even when the system grows in place.
//!
//! ## Installing it
//!
//! A process has exactly one global allocator, and only the final binary may
//! choose it, so a library cannot install it for you. A test or bench binary
//! opts in with one line:
//!
//! ```ignore
//! coding_adventures_test_bootstrap::install_counting_allocator!();
//! ```
//!
//! If a binary forgets, [`is_installed`] says so and
//! [`crate::bench::AllocCounter`] reports `NotLive`, which leaves a
//! `max_alloc_bytes` budget listed as *unchecked* instead of falsely passed.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

/// A tally of heap activity on one thread. All fields only ever grow.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AllocStats {
    /// Number of successful allocations (including the allocating half of a
    /// `realloc`).
    pub allocations: u64,
    /// Number of frees (including the freeing half of a `realloc`).
    pub deallocations: u64,
    /// Total bytes requested by successful allocations.
    pub bytes_allocated: u64,
    /// Total bytes released by frees.
    pub bytes_deallocated: u64,
}

impl AllocStats {
    /// The empty tally.
    pub const ZERO: AllocStats = AllocStats {
        allocations: 0,
        deallocations: 0,
        bytes_allocated: 0,
        bytes_deallocated: 0,
    };

    /// Activity between an `earlier` reading and this one. Saturates so a
    /// reading taken on a different thread can never underflow.
    pub fn since(self, earlier: AllocStats) -> AllocStats {
        AllocStats {
            allocations: self.allocations.saturating_sub(earlier.allocations),
            deallocations: self.deallocations.saturating_sub(earlier.deallocations),
            bytes_allocated: self.bytes_allocated.saturating_sub(earlier.bytes_allocated),
            bytes_deallocated: self
                .bytes_deallocated
                .saturating_sub(earlier.bytes_deallocated),
        }
    }
}

thread_local! {
    // `const` initialiser + `Copy` payload: no lazy init, no destructor, so
    // touching this from inside the allocator can never allocate.
    static TALLY: Cell<AllocStats> = const { Cell::new(AllocStats::ZERO) };
}

fn bump(f: impl FnOnce(&mut AllocStats)) {
    // `try_with` instead of `with`: never panic from inside the allocator.
    let _ = TALLY.try_with(|cell| {
        let mut stats = cell.get();
        f(&mut stats);
        cell.set(stats);
    });
}

fn record_alloc(bytes: usize) {
    bump(|s| {
        s.allocations += 1;
        s.bytes_allocated += bytes as u64;
    });
}

fn record_dealloc(bytes: usize) {
    bump(|s| {
        s.deallocations += 1;
        s.bytes_deallocated += bytes as u64;
    });
}

/// The calling thread's cumulative tally since the thread began. Stays at
/// zero forever unless [`CountingAllocator`] is the global allocator.
pub fn thread_stats() -> AllocStats {
    TALLY.try_with(Cell::get).unwrap_or_default()
}

/// Is [`CountingAllocator`] really this process's global allocator?
///
/// Proved by experiment, not by trusting a flag: allocate one box and see
/// whether the tally noticed.
pub fn is_installed() -> bool {
    let before = thread_stats();
    let probe = std::hint::black_box(Box::new(0u64));
    let after = thread_stats();
    drop(probe);
    // Make the probe invisible: put the tally back exactly as we found it, so
    // asking "is it installed?" never shows up in a measurement window.
    let _ = TALLY.try_with(|cell| cell.set(before));
    after.allocations > before.allocations
}

/// Run `f` and report the heap activity it caused on this thread, or `None`
/// for the activity when the counting allocator is not installed (a zero
/// would be a lie; see the module docs).
pub fn measure<R>(f: impl FnOnce() -> R) -> (R, Option<AllocStats>) {
    let before = thread_stats();
    let value = f();
    let after = thread_stats();
    let stats = if is_installed() {
        Some(after.since(before))
    } else {
        None
    };
    (value, stats)
}

/// A [`GlobalAlloc`] that forwards every request to [`System`] and tallies it.
///
/// It adds no behaviour of its own: sizes, alignments and returned pointers
/// are exactly the system allocator's.
#[derive(Debug, Default, Clone, Copy)]
pub struct CountingAllocator;

// SAFETY: every method forwards its arguments unchanged to `System`, which
// upholds the `GlobalAlloc` contract; the only extra work is updating a
// thread-local `Cell` of plain integers, which neither allocates nor unwinds,
// so it cannot violate the contract's "must not panic / must not re-enter the
// allocator" requirements.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: caller upholds `GlobalAlloc::alloc`'s contract for `layout`.
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            record_alloc(layout.size());
        }
        ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: caller upholds `GlobalAlloc::alloc_zeroed`'s contract.
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() {
            record_alloc(layout.size());
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: caller guarantees `ptr` was allocated here with `layout`;
        // every allocation above came from `System` with the same layout.
        unsafe { System.dealloc(ptr, layout) };
        record_dealloc(layout.size());
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: caller upholds `GlobalAlloc::realloc`'s contract; `ptr`
        // came from `System` with `layout`.
        let new_ptr = unsafe { System.realloc(ptr, layout, new_size) };
        if !new_ptr.is_null() {
            // Conservative accounting: a free of the old block and an
            // allocation of the new size (a failed realloc changes nothing).
            record_dealloc(layout.size());
            record_alloc(new_size);
        }
        new_ptr
    }
}

/// Declare [`CountingAllocator`] as this binary's global allocator.
///
/// Use it once, at the top level of a test or bench binary that wants
/// `max_alloc_bytes` budgets to be checked against the real heap.
#[macro_export]
macro_rules! install_counting_allocator {
    () => {
        #[global_allocator]
        static COUNTING_ALLOCATOR: $crate::counting_alloc::CountingAllocator =
            $crate::counting_alloc::CountingAllocator;
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    // The allocator itself is installed once for the whole unit-test binary
    // in lib.rs.

    #[test]
    fn installed_in_this_test_binary() {
        assert!(is_installed());
    }

    #[test]
    fn a_vec_allocation_is_counted_exactly() {
        let (v, stats) = measure(|| std::hint::black_box(Vec::<u8>::with_capacity(1)));
        let stats = stats.expect("installed");
        assert_eq!(stats.allocations, 1);
        assert_eq!(stats.bytes_allocated, 1);
        assert_eq!(stats.deallocations, 0);
        drop(v);
    }

    #[test]
    fn work_without_the_heap_counts_nothing() {
        let (sum, stats) = measure(|| (0..1000u64).map(std::hint::black_box).sum::<u64>());
        assert_eq!(sum, 499_500);
        assert_eq!(stats, Some(AllocStats::ZERO));
    }

    #[test]
    fn frees_zeroed_allocations_and_reallocs_are_counted() {
        let (_, stats) = measure(|| {
            let zeroed = std::hint::black_box(vec![0u8; 64]); // alloc_zeroed
            let mut grow = Vec::<u8>::with_capacity(8);
            grow.extend_from_slice(&[1; 8]);
            grow.reserve_exact(100); // realloc: 8 -> 108
            std::hint::black_box(&grow);
            drop(zeroed);
            drop(grow);
        });
        let s = stats.expect("installed");
        // 64 (zeroed) + 8 (initial) + 108 (realloc) bytes allocated.
        assert_eq!(s.bytes_allocated, 64 + 8 + 108);
        assert_eq!(s.allocations, 3);
        // realloc frees 8, then both vecs free 64 and 108.
        assert_eq!(s.bytes_deallocated, 8 + 64 + 108);
        assert_eq!(s.deallocations, 3);
    }

    #[test]
    fn tallies_are_per_thread() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        let go = Arc::new(AtomicBool::new(false));
        let child_go = Arc::clone(&go);
        let child = std::thread::spawn(move || {
            while !child_go.load(Ordering::Acquire) {
                std::hint::spin_loop();
            }
            drop(std::hint::black_box(vec![0u8; 4096]));
            thread_stats().bytes_allocated
        });
        // Everything the spawn itself allocated is behind us; from here the
        // only 4096-byte allocation happens on the child thread.
        let before = thread_stats();
        go.store(true, Ordering::Release);
        let child_bytes = child.join().expect("join");
        assert!(child_bytes >= 4096, "the child saw its own allocation");
        assert_eq!(thread_stats().since(before).bytes_allocated, 0);
    }

    #[test]
    fn since_saturates() {
        let big = AllocStats {
            allocations: 5,
            deallocations: 5,
            bytes_allocated: 5,
            bytes_deallocated: 5,
        };
        assert_eq!(AllocStats::ZERO.since(big), AllocStats::ZERO);
    }

    #[test]
    fn counting_allocator_is_a_transparent_wrapper() {
        // Direct calls behave like System.
        let a = CountingAllocator;
        let layout = Layout::from_size_align(32, 8).unwrap();
        // SAFETY: layout is non-zero-sized; the pointer is freed with the
        // layout it was last allocated with.
        unsafe {
            let p = a.alloc(layout);
            assert!(!p.is_null());
            p.write_bytes(7, 32);
            let q = a.realloc(p, layout, 64);
            assert!(!q.is_null());
            assert_eq!(*q, 7);
            a.dealloc(q, Layout::from_size_align(64, 8).unwrap());
        }
    }
}
