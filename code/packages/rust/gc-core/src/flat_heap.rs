//! # `flat_heap` — a real-memory mark-and-sweep heap for native consumers
//!
//! `gc-core`'s primary collector ([`crate::gc_core::GcCore`] over the
//! `garbage-collector` crate) models the heap as a `HashMap<usize, Box<dyn
//! HeapObject>>` with *synthetic* addresses.  That is exactly right for the
//! **interpreters** (`vm-core`, `jit-core`): a running Rust interpreter always
//! knows the static type of every slot, so a boxed trait object per heap value
//! costs nothing conceptually and buys reflection-free tracing.
//!
//! It is the **wrong** model for **native-AOT** output.  There the heap must be
//! *flat*: `alloc(n)` returns a **real machine pointer** to `n` contiguous bytes
//! that compiled code reads and writes directly at byte offsets (the IIR
//! `field_load`/`field_store` ops), with no map indirection and no `Box<dyn>`.
//! A McCarthy-Lisp cons cell, a boxed integer, a closure record — all are just
//! `alloc(16)` returning a pointer the generated machine code dereferences.
//!
//! This module is that flat representation, lifted into `gc-core` as a
//! first-class **generic** algorithm (see `AOT00-T1-precise-gc.md` §3.1 and
//! `LANG16-gc-core.md`).  It is the collector the native-AOT / LLVM / WASM
//! columns link through the C ABI (`gc-core-capi`), and it supersedes the
//! Twig-specific `twig-aot/runtime/twig_gc.c` — same flat model (32-byte header,
//! 16-byte-aligned payload, intrusive free list, mark/sweep), but now one
//! generic Rust collector every native consumer shares rather than a hand-written
//! C fork.
//!
//! ## Memory layout
//!
//! Every allocation is one contiguous block: a fixed **32-byte header**
//! immediately followed by the caller's payload.
//!
//! ```text
//!   block ─┐
//!          ▼
//!   ┌──────────────── 32-byte FlatHeader ───────────────┬─ payload (n bytes) ─┐
//!   │ next(8) │ size(8) │ marked(1) │ kind(2) │ pad(13) │  … caller's bytes …  │
//!   └─────────────────────────────────────────────────┴──────────────────────┘
//!   ^block                                              ^block + 32  (returned)
//! ```
//!
//! The block is allocated 16-byte aligned and the header is exactly 32 bytes, so
//! the **payload is always 16-byte aligned** — its low 4 bits are zero, which the
//! NaN-box tag scheme (low-3-bits tag on dynamic values) requires: a real heap
//! pointer never collides with a tagged immediate.  The caller-visible pointer is
//! `block + 32`; the reverse map is `header = payload - 32`.
//!
//! ## What this PR delivers (T1 rung 0 — the flat core)
//!
//! A precise, *explicit-root* mark-and-sweep:
//! - [`FlatHeap::alloc`] — real-pointer allocation, header + zeroed payload.
//! - [`FlatHeap::collect`] — mark from a caller-supplied root list, then sweep.
//!   Roots and interior fields are traced **conservatively** for now (every
//!   aligned word that looks like an in-heap pointer, raw and tag-stripped, is
//!   followed) — identical semantics to `twig_gc.c` minus the C-stack scan.
//!   Precise roots (stack maps) and precise interior tracing (`HeapKind` field
//!   maps) are the next rungs; the `kind` id is already stored per object so they
//!   slot in without a layout change.
//! - live-byte / collection-count accounting, wired into [`GcProfile`] so this
//!   collector participates in `gc-core`'s adaptive policy like any other.
//!
//! The C ABI and the conservative *C-stack* scan that makes `collect()` safe to
//! call with no explicit roots (the drop-in for `twig_gc.c`'s argument-less
//! `__twig_gc_collect`) live in the `gc-core-capi` crate and a follow-up PR
//! respectively.

use crate::profile::{GcCycleStats, GcProfile};
use std::alloc::{alloc_zeroed, dealloc, Layout};
use std::ptr;

/// Bytes of header prepended to every payload.  Chosen as 32 (not the natural
/// ~24 the fields need) so that a 16-byte-aligned block yields a 16-byte-aligned
/// payload — see the module-level layout note.
const HEADER_SIZE: usize = 32;

/// Allocation alignment.  16 bytes guarantees the low 4 payload-address bits are
/// zero (the NaN-box tag invariant).
const ALIGN: usize = 16;

/// Per-object header, placed immediately before the payload.
///
/// `#[repr(C)]` + explicit padding pins `size_of::<FlatHeader>() == 32` on every
/// target (asserted below), so `payload = (header as *mut FlatHeader).add(1)` and
/// `header = (payload as *mut FlatHeader).sub(1)` are exact inverses.
#[repr(C)]
struct FlatHeader {
    /// Intrusive singly-linked list threading **every** live block; the sweep
    /// walks it.  Head is [`FlatHeap::all`].
    next: *mut FlatHeader,
    /// Payload size in bytes (excludes the header).  Drives the conservative
    /// interior scan and reconstructs the `Layout` for `dealloc`.
    size: usize,
    /// Mark bit: set during mark, cleared by sweep on survivors.
    marked: bool,
    /// `HeapKind` id for *precise* interior tracing (a later rung).  `0` means
    /// "opaque / trace conservatively"; unused by this PR beyond being carried.
    kind: u16,
    /// Explicit tail padding to reach exactly 32 bytes.
    _pad: [u8; 12],
}

// Compile-time proof that the header is exactly 32 bytes — if a field ever
// changes size this fails to build rather than silently misaligning payloads.
const _: () = assert!(std::mem::size_of::<FlatHeader>() == HEADER_SIZE);

/// A flat, real-memory mark-and-sweep heap.
///
/// Owns every block it hands out and frees them on `collect` (unreachable ones)
/// or on `Drop` (all remaining).  Single-threaded by design — the native AOT
/// runtime is single-threaded (matching `twig_gc.c`), so no locking lives here;
/// the C ABI layer serialises access.
pub struct FlatHeap {
    /// Head of the all-blocks list.
    all: *mut FlatHeader,
    /// Sum of live payload bytes (updated on alloc and after each sweep).
    live_bytes: usize,
    /// Total collections run since creation.
    collection_count: u64,
    /// Adaptive-policy profile shared with the rest of `gc-core`.
    profile: GcProfile,
}

// SAFETY: `FlatHeap` holds raw pointers (`*mut FlatHeader`), which make it
// `!Send` by default.  The native AOT runtime is single-threaded and the C ABI
// wrapper (`gc-core-capi`) owns the single instance behind a mutex, so the heap
// is never touched from two threads at once.  Marking it `Send` lets that
// wrapper store it in a `static`.
unsafe impl Send for FlatHeap {}

impl Default for FlatHeap {
    fn default() -> Self {
        Self::new()
    }
}

impl FlatHeap {
    /// An empty heap.
    pub fn new() -> Self {
        FlatHeap {
            all: ptr::null_mut(),
            live_bytes: 0,
            collection_count: 0,
            profile: GcProfile::default(),
        }
    }

    /// Allocate `n` zeroed bytes and return a **real pointer** to the payload.
    ///
    /// `kind` is the object's [`crate::kind::HeapKind`] id (`0` = opaque); it is
    /// stored for later precise tracing and otherwise inert here.  Returns a null
    /// pointer if `n == 0`, on `header + n` overflow, or on allocator failure —
    /// the same fail-to-null contract the C ABI expects.
    ///
    /// The returned pointer is 16-byte aligned (payload alignment invariant).
    pub fn alloc(&mut self, n: usize, kind: u16) -> *mut u8 {
        if n == 0 {
            return ptr::null_mut();
        }
        // Overflow guard: a caller-controlled `n` near usize::MAX must not wrap
        // `HEADER_SIZE + n` to a small value (which would under-allocate while
        // `size` still records the huge value → out-of-bounds interior scan).
        let total = match HEADER_SIZE.checked_add(n) {
            Some(t) => t,
            None => return ptr::null_mut(),
        };
        let layout = match Layout::from_size_align(total, ALIGN) {
            Ok(l) => l,
            Err(_) => return ptr::null_mut(),
        };
        // SAFETY: `layout` has non-zero size (total >= HEADER_SIZE > 0) and a
        // valid power-of-two alignment.  `alloc_zeroed` zero-initialises the
        // whole block, so the payload starts as all-zero (matching `calloc`).
        let block = unsafe { alloc_zeroed(layout) } as *mut FlatHeader;
        if block.is_null() {
            return ptr::null_mut();
        }
        // SAFETY: `block` points to `total >= 32` freshly-allocated bytes, so the
        // header fields are in bounds; the list-prepend keeps the invariant that
        // every handed-out block is reachable from `self.all`.
        unsafe {
            (*block).next = self.all;
            (*block).size = n;
            (*block).marked = false;
            (*block).kind = kind;
        }
        self.all = block;
        self.live_bytes += n;
        self.profile.record_allocation(n);
        // Payload is the byte just past the 32-byte header.
        // SAFETY: `size_of::<FlatHeader>() == 32`, so `.add(1)` lands exactly on
        // the payload within the same allocation.
        unsafe { block.add(1) as *mut u8 }
    }

    /// Current live payload bytes (post-last-sweep for reclaimed blocks, plus any
    /// allocated since).
    pub fn live_bytes(&self) -> usize {
        self.live_bytes
    }

    /// Total collections run.
    pub fn collection_count(&self) -> u64 {
        self.collection_count
    }

    /// Read-only view of the adaptive profile.
    pub fn profile(&self) -> &GcProfile {
        &self.profile
    }

    /// Number of live blocks (walks the list — O(n); for tests/introspection).
    pub fn object_count(&self) -> usize {
        let mut n = 0;
        let mut h = self.all;
        // SAFETY: `all` and every `next` either point at a live block we own or
        // are null (list terminator).
        while !h.is_null() {
            n += 1;
            h = unsafe { (*h).next };
        }
        n
    }

    /// Run one mark-and-sweep cycle rooted at `roots` (payload addresses as
    /// `usize`).  Returns the cycle's [`GcCycleStats`] (also folded into the
    /// profile).  Any block not reachable from `roots` is freed.
    ///
    /// Tracing is **conservative**: each root word, and each aligned word inside a
    /// marked object's payload, is treated as a *candidate* pointer — both the raw
    /// value and the value with its low 3 tag bits cleared are looked up against
    /// the live set (so both a raw `alloc` pointer and a NaN-box-tagged heap
    /// reference are followed).  A false positive retains a dead object for one
    /// extra cycle; it never frees a live one.
    pub fn collect(&mut self, roots: &[usize]) -> GcCycleStats {
        let before = self.object_count();

        // ── Mark ──────────────────────────────────────────────────────────────
        // Iterative worklist (no recursion → no stack blow-up on deep lists).
        let mut work: Vec<*mut FlatHeader> = Vec::new();
        for &r in roots {
            self.mark_word(r, &mut work);
        }
        while let Some(h) = work.pop() {
            self.scan_payload(h, &mut work);
        }

        // ── Sweep ─────────────────────────────────────────────────────────────
        let (freed, survived, live) = self.sweep();
        self.live_bytes = live;

        let stats = GcCycleStats {
            freed,
            survived,
            pause_ns: 0,
            heap_size_before: before,
            heap_size_after: survived,
        };
        self.profile.record_cycle(&stats);
        self.collection_count += 1;
        stats
    }

    /// Collect using a **raw memory region** as the conservative root set.
    ///
    /// Every aligned word in `[base, base + len)` is treated as a *candidate* root
    /// (raw and low-3-bit-tag-stripped, exactly like [`Self::collect`]'s explicit
    /// roots); every object reachable from one survives, the rest are freed.
    ///
    /// This is the primitive a **native runtime** needs. Where [`Self::collect`]
    /// takes a tidy slice of `usize` roots the caller already gathered, real
    /// compiled code keeps its live references in *memory the collector must scan
    /// itself*: a block of spilled callee-saved registers, or the machine call
    /// stack from the current stack pointer up to the thread's stack base. Point
    /// this method at that memory and it roots from it. The argument-less
    /// `__twig_gc_collect` / `__twig_gc_safepoint` the native backend emits are
    /// built on exactly this — they discover `(base, len)` for the live stack (a
    /// platform-specific register-spill + stack-base step) and hand it here. This
    /// method is that platform-independent, unit-testable core; the stack-range
    /// discovery is layered on top separately.
    ///
    /// A false positive (a plain integer in the region whose bit pattern lands in a
    /// live block) retains a dead object for one cycle — never frees a live one.
    /// That imprecision is the defining, intended property of a conservative scan.
    ///
    /// # Safety
    ///
    /// `[base, base + len)` must be readable for the duration of the call (`base`
    /// may be null iff `len == 0`). No alignment of `base`/`len` is required —
    /// scanning starts at `base` and a sub-8-byte tail is ignored.
    pub unsafe fn collect_region(&mut self, base: *const u8, len: usize) -> GcCycleStats {
        let before = self.object_count();

        let mut work: Vec<*mut FlatHeader> = Vec::new();
        // SAFETY: caller guarantees `[base, base+len)` is readable.
        self.mark_region(base, len, &mut work);
        while let Some(h) = work.pop() {
            self.scan_payload(h, &mut work);
        }

        let (freed, survived, live) = self.sweep();
        self.live_bytes = live;

        let stats = GcCycleStats {
            freed,
            survived,
            pause_ns: 0,
            heap_size_before: before,
            heap_size_after: survived,
        };
        self.profile.record_cycle(&stats);
        self.collection_count += 1;
        stats
    }

    /// Scan `[base, base + len)` as an array of aligned candidate root words,
    /// enqueuing every live block a word points into. The region-oriented sibling
    /// of [`Self::scan_payload`] (which scans one object's payload); both defer to
    /// [`Self::mark_word`] for the raw-plus-tag-stripped lookup.
    ///
    /// # Safety
    ///
    /// `[base, base + len)` must be readable.
    unsafe fn mark_region(&self, base: *const u8, len: usize, work: &mut Vec<*mut FlatHeader>) {
        let mut off = 0usize;
        while off + 8 <= len {
            // SAFETY: `off + 8 <= len`, so the 8-byte read stays inside the region;
            // `read_unaligned` tolerates any sub-alignment of `base`.
            let word = ptr::read_unaligned(base.add(off) as *const usize);
            self.mark_word(word, work);
            off += 8;
        }
    }

    /// Mark the block a candidate `word` points into, if any, and enqueue it.
    /// Checks both the raw word and the tag-stripped word (NaN-box compat).
    fn mark_word(&self, word: usize, work: &mut Vec<*mut FlatHeader>) {
        // Raw candidate.
        let h = self.find_header(word);
        if !h.is_null() {
            // SAFETY: `find_header` only returns headers of live blocks we own.
            unsafe {
                if !(*h).marked {
                    (*h).marked = true;
                    work.push(h);
                }
            }
        }
        // Tag-stripped candidate (low 3 bits are a NaN-box tag on dyn values).
        let stripped = word & !0x7usize;
        if stripped != word && stripped != 0 {
            let h2 = self.find_header(stripped);
            if !h2.is_null() {
                // SAFETY: as above.
                unsafe {
                    if !(*h2).marked {
                        (*h2).marked = true;
                        work.push(h2);
                    }
                }
            }
        }
    }

    /// Conservatively scan `h`'s payload for further candidate pointers.
    fn scan_payload(&self, h: *mut FlatHeader, work: &mut Vec<*mut FlatHeader>) {
        // SAFETY: `h` is a live block; its payload is `size` bytes at `h + 32`.
        let (base, size) = unsafe {
            let payload = (h.add(1)) as *const u8;
            (payload, (*h).size)
        };
        let mut off = 0usize;
        while off + 8 <= size {
            // SAFETY: `off + 8 <= size`, so the 8-byte read stays inside the
            // payload.  `read_unaligned` tolerates any payload sub-alignment.
            let word = unsafe { ptr::read_unaligned(base.add(off) as *const usize) };
            self.mark_word(word, work);
            off += 8;
        }
    }

    /// Return the header of the live block whose payload contains `addr`, or null.
    /// Linear scan of the all-blocks list (matching `twig_gc.c`'s V1; a sorted
    /// interval index is a later optimisation).
    fn find_header(&self, addr: usize) -> *mut FlatHeader {
        if addr == 0 {
            return ptr::null_mut();
        }
        let mut h = self.all;
        // SAFETY: list walk over blocks we own / null.
        unsafe {
            while !h.is_null() {
                let payload = h.add(1) as usize;
                let end = payload + (*h).size;
                if addr >= payload && addr < end {
                    return h;
                }
                h = (*h).next;
            }
        }
        ptr::null_mut()
    }

    /// Free every unmarked block; clear marks on survivors.  Returns
    /// `(freed, survived, live_bytes)`.
    fn sweep(&mut self) -> (usize, usize, usize) {
        let mut freed = 0usize;
        let mut survived = 0usize;
        let mut live = 0usize;
        // `cursor` points at the link that currently references the block under
        // inspection (`&mut self.all` first, then `&mut (*prev).next`), so we can
        // unlink in place without a previous-node special case.
        let mut cursor: *mut *mut FlatHeader = &mut self.all;
        // SAFETY: every dereferenced pointer is a live block we own; `dealloc`
        // uses the exact `Layout` `alloc_zeroed` was given (same size + align).
        unsafe {
            while !(*cursor).is_null() {
                let h = *cursor;
                if (*h).marked {
                    (*h).marked = false;
                    survived += 1;
                    live += (*h).size;
                    cursor = &mut (*h).next;
                } else {
                    *cursor = (*h).next;
                    let layout =
                        Layout::from_size_align_unchecked(HEADER_SIZE + (*h).size, ALIGN);
                    dealloc(h as *mut u8, layout);
                    freed += 1;
                }
            }
        }
        (freed, survived, live)
    }
}

impl Drop for FlatHeap {
    /// Free every block still on the list — no leak when the heap itself is
    /// dropped (e.g. a test's `FlatHeap` going out of scope).
    fn drop(&mut self) {
        let mut h = self.all;
        // SAFETY: list walk freeing blocks we own with their exact layouts.
        unsafe {
            while !h.is_null() {
                let next = (*h).next;
                let layout = Layout::from_size_align_unchecked(HEADER_SIZE + (*h).size, ALIGN);
                dealloc(h as *mut u8, layout);
                h = next;
            }
        }
        self.all = ptr::null_mut();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `alloc` returns distinct, non-null, 16-byte-aligned pointers and tracks
    /// live bytes.
    #[test]
    fn alloc_returns_aligned_distinct_pointers() {
        let mut heap = FlatHeap::new();
        let a = heap.alloc(16, 0);
        let b = heap.alloc(16, 0);
        assert!(!a.is_null() && !b.is_null());
        assert_ne!(a, b, "distinct allocations must not alias");
        assert_eq!(a as usize % ALIGN, 0, "payload must be 16-byte aligned");
        assert_eq!(b as usize % ALIGN, 0);
        assert_eq!(heap.live_bytes(), 32);
        assert_eq!(heap.object_count(), 2);
    }

    /// The payload is writable and reads back what was written (real memory, not
    /// a synthetic handle).
    #[test]
    fn payload_roundtrips_values() {
        let mut heap = FlatHeap::new();
        let p = heap.alloc(16, 0) as *mut i64;
        unsafe {
            *p = 0x0102_0304_0506_0708;
            *p.add(1) = -42;
            assert_eq!(*p, 0x0102_0304_0506_0708);
            assert_eq!(*p.add(1), -42);
        }
    }

    /// `alloc_zeroed` contract: a fresh payload is all zero.
    #[test]
    fn fresh_payload_is_zeroed() {
        let mut heap = FlatHeap::new();
        let p = heap.alloc(24, 0);
        unsafe {
            for i in 0..24 {
                assert_eq!(*p.add(i), 0, "byte {i} not zeroed");
            }
        }
    }

    /// `alloc(0)` and overflow fail to null without panicking.
    #[test]
    fn degenerate_allocs_fail_to_null() {
        let mut heap = FlatHeap::new();
        assert!(heap.alloc(0, 0).is_null());
        assert!(heap.alloc(usize::MAX, 0).is_null());
        assert_eq!(heap.live_bytes(), 0);
    }

    /// A rooted object survives; an unrooted one is reclaimed.
    #[test]
    fn collect_frees_unrooted_keeps_rooted() {
        let mut heap = FlatHeap::new();
        let keep = heap.alloc(16, 0) as usize;
        let _drop = heap.alloc(16, 0); // no root → garbage
        assert_eq!(heap.object_count(), 2);
        assert_eq!(heap.live_bytes(), 32);

        let stats = heap.collect(&[keep]);
        assert_eq!(stats.freed, 1);
        assert_eq!(stats.survived, 1);
        assert_eq!(heap.object_count(), 1);
        assert_eq!(heap.live_bytes(), 16);
        assert_eq!(heap.collection_count(), 1);
        // The survivor's memory is still valid (rooted pointer unchanged).
        assert!(!heap.find_header(keep).is_null());
    }

    /// Interior pointers are followed: rooting a parent keeps a child it points
    /// at, transitively (a 3-deep chain).
    #[test]
    fn collect_traces_interior_pointers_transitively() {
        let mut heap = FlatHeap::new();
        let c = heap.alloc(16, 0) as usize;
        let b = heap.alloc(16, 0) as usize;
        let a = heap.alloc(16, 0) as usize;
        // a.field0 -> b, b.field0 -> c   (a raw cons-style chain)
        unsafe {
            *(a as *mut usize) = b;
            *(b as *mut usize) = c;
        }
        let _garbage = heap.alloc(16, 0); // unreachable
        assert_eq!(heap.object_count(), 4);

        let stats = heap.collect(&[a]); // root only the head
        assert_eq!(stats.survived, 3, "a→b→c must all survive");
        assert_eq!(stats.freed, 1, "only the unreachable block is freed");
        assert!(!heap.find_header(a).is_null());
        assert!(!heap.find_header(b).is_null());
        assert!(!heap.find_header(c).is_null());
    }

    /// A tagged (low-3-bits) heap reference is followed after stripping the tag.
    #[test]
    fn collect_follows_tagged_reference() {
        let mut heap = FlatHeap::new();
        let obj = heap.alloc(16, 0) as usize;
        let tagged = obj | 0x7; // NaN-box HEAP tag in the low 3 bits
        let stats = heap.collect(&[tagged]);
        assert_eq!(stats.freed, 0, "tagged pointer must keep its object alive");
        assert_eq!(stats.survived, 1);
    }

    /// Collecting with no roots frees everything.
    #[test]
    fn collect_no_roots_frees_all() {
        let mut heap = FlatHeap::new();
        heap.alloc(16, 0);
        heap.alloc(32, 0);
        let stats = heap.collect(&[]);
        assert_eq!(stats.freed, 2);
        assert_eq!(heap.object_count(), 0);
        assert_eq!(heap.live_bytes(), 0);
    }

    /// Repeated collections are stable and keep counting.
    #[test]
    fn repeated_collections_count_up() {
        let mut heap = FlatHeap::new();
        let keep = heap.alloc(16, 0) as usize;
        heap.collect(&[keep]);
        heap.collect(&[keep]);
        heap.collect(&[keep]);
        assert_eq!(heap.collection_count(), 3);
        assert_eq!(heap.object_count(), 1);
        assert_eq!(heap.profile().total_collections, 3);
    }

    /// `collect_region`: an object whose payload pointer appears anywhere in the
    /// scanned region survives; an object with no candidate pointer in the region
    /// is reclaimed. This is the exact behaviour a register-block / stack scan
    /// relies on.
    #[test]
    fn collect_region_roots_from_a_memory_region() {
        let mut heap = FlatHeap::new();
        let keep = heap.alloc(16, 0) as usize;
        let _garbage = heap.alloc(16, 0); // no pointer to it in the region below
        assert_eq!(heap.object_count(), 2);

        // A synthetic "register block" / stack slice: some plain integers plus the
        // live pointer, exactly as a real stack region interleaves data and refs.
        let region: [usize; 4] = [0xdead_beef, keep, 0, 42];
        let stats = unsafe {
            heap.collect_region(region.as_ptr() as *const u8, std::mem::size_of_val(&region))
        };

        assert_eq!(stats.survived, 1, "the region-rooted object must survive");
        assert_eq!(stats.freed, 1, "the object with no candidate in the region is freed");
        assert!(!heap.find_header(keep).is_null());
        assert_eq!(heap.live_bytes(), 16);
    }

    /// `collect_region` follows a low-3-bit-tagged (NaN-box) reference found in the
    /// region, not just a raw pointer.
    #[test]
    fn collect_region_follows_tagged_reference_in_region() {
        let mut heap = FlatHeap::new();
        let obj = heap.alloc(16, 0) as usize;
        let region: [usize; 2] = [obj | 0x7, 0]; // tagged in the low 3 bits
        let stats =
            unsafe { heap.collect_region(region.as_ptr() as *const u8, std::mem::size_of_val(&region)) };
        assert_eq!(stats.freed, 0);
        assert_eq!(stats.survived, 1);
    }

    /// An empty region (null base, zero length) roots nothing → everything is freed,
    /// matching `collect(&[])`.
    #[test]
    fn collect_region_empty_frees_all() {
        let mut heap = FlatHeap::new();
        heap.alloc(16, 0);
        heap.alloc(32, 0);
        let stats = unsafe { heap.collect_region(std::ptr::null(), 0) };
        assert_eq!(stats.freed, 2);
        assert_eq!(heap.object_count(), 0);
    }

    /// Interior pointers are still followed transitively from a region root: rooting
    /// the head of an `a → b → c` chain via the region keeps all three.
    #[test]
    fn collect_region_traces_interior_transitively() {
        let mut heap = FlatHeap::new();
        let c = heap.alloc(16, 0) as usize;
        let b = heap.alloc(16, 0) as usize;
        let a = heap.alloc(16, 0) as usize;
        unsafe {
            *(a as *mut usize) = b;
            *(b as *mut usize) = c;
        }
        let _garbage = heap.alloc(16, 0);
        let region: [usize; 1] = [a];
        let stats =
            unsafe { heap.collect_region(region.as_ptr() as *const u8, std::mem::size_of_val(&region)) };
        assert_eq!(stats.survived, 3);
        assert_eq!(stats.freed, 1);
    }
}
