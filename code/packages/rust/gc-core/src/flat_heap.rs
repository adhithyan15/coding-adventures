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
use std::alloc::{alloc, alloc_zeroed, dealloc, Layout};
use std::collections::{HashMap, HashSet};
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
    /// Which generation the object lives in: [`GEN_YOUNG`] (freshly allocated) or
    /// [`GEN_OLD`] (survived enough collections to be tenured — see [`Self::age`]).
    /// A generational *minor* GC scans only the young generation plus old→young
    /// pointers.
    generation: u8,
    /// **Tenuring age**: how many collections this object has *survived while
    /// young*. Incremented on each survival (in [`FlatHeap::sweep`]); when it
    /// reaches the heap's [`FlatHeap::tenure_age`] threshold the object is promoted
    /// to [`GEN_OLD`]. Born `0` (via `alloc_zeroed`), so with the default threshold
    /// of `1` an object tenures on its first survival — exactly the immediate
    /// tenuring the generational rung shipped with. A larger threshold keeps
    /// short-lived-but-not-instantly-dead objects in the young generation longer,
    /// reducing *premature* tenuring (objects that die in their 2nd/3rd cycle would
    /// otherwise pollute the old generation and be reclaimed only by a full GC).
    /// Saturates rather than wrapping. Old objects never age further.
    age: u8,
    /// **Pin bit** for the moving/compacting collector (AOT00-T3). Set during a
    /// mobility classification when the object is reachable through a *conservative*
    /// root or edge (a `collect_region` span, or any `kind == 0` object whose maybe-
    /// pointer words can't be safely rewritten). A pinned object must NOT be
    /// relocated — a stale conservative pointer to its old address would dangle. It
    /// is a per-classification transient (born `0` via `alloc_zeroed`, recomputed
    /// each pass). This PR only *computes* it (the movable/pinned predicate); the
    /// forwarding word + actual relocation are a later rung (which reuses `next` as
    /// the forwarding slot during stop-the-world, per spec §3.1, so no further
    /// header growth). Kept 1 byte so the header stays exactly 32 bytes.
    pinned: bool,
    /// **Provenance** flag for the moving/compacting collector (AOT00-T3 PR-3c). `false`
    /// (the `alloc_zeroed` default) for a normal per-object [`alloc`](FlatHeap::alloc)'d
    /// block; `true` for a block that lives inside an [`Arena`] (a moved object's copy).
    /// It is **load-bearing for safety**: an arena-backed block is a *slice* of one big
    /// arena allocation, so it must NEVER be handed to `dealloc` individually — the whole
    /// arena is freed together (on heap drop, or when a future compaction re-moves its
    /// live objects). Every `dealloc` site (`sweep`, `Drop`) skips arena-backed blocks.
    arena_backed: bool,
    /// Explicit tail padding to reach exactly 32 bytes.
    _pad: [u8; 8],
}

/// Generation tag for a freshly-allocated object: the **young** generation, where
/// most objects die (the generational hypothesis). New allocations start here.
pub const GEN_YOUNG: u8 = 0;

/// Generation tag for an object that has **survived** a collection and been
/// promoted (tenured) to the **old** generation. A minor GC skips old objects
/// except where an old→young pointer (recorded by a write barrier) demands it.
pub const GEN_OLD: u8 = 1;

// Compile-time proof that the header is exactly 32 bytes — if a field ever
// changes size this fails to build rather than silently misaligning payloads.
const _: () = assert!(std::mem::size_of::<FlatHeader>() == HEADER_SIZE);

/// The **reference layout** of one registered kind — where an object of that kind holds
/// its garbage-collected references, so the collector can trace it *precisely* (follow only
/// the reference words) instead of *conservatively* (treat every payload word as a
/// maybe-pointer).
///
/// A layout has two parts, visited in order by [`FlatHeap::for_each_ref_slot`]:
///
/// - **`fixed`** — reference fields at statically-known byte offsets, shared by every
///   instance of the kind. This is the **record** case (a cons cell `{car, cdr}`, a
///   `Point{x, y}`, a closure header): the reference offsets don't depend on the instance.
/// - **`tail_from`** — if `Some(start)`, every aligned 8-byte word in `[start, size)` of the
///   instance's payload is a reference. This is the **variable-length array** case (a JS
///   `Array`, a Ruby `Array`, a Python `list`, a vector, a hash's backing store), where the
///   number of reference slots is a property of the *instance* (its `size`), not the kind, so
///   a fixed offset list cannot describe it. `None` ⇒ a pure record (the pre-T5 behaviour).
///
/// `fixed` before `tail_from` composes: a boxed array object can carry a reference header
/// field *and* a variable reference tail. A pure record is `tail_from == None`, which
/// reproduces the original fixed-offset tracer byte-for-byte — so this type is a **strict
/// generalization** of the old `Box<[usize]>` field map. See
/// `code/specs/AOT00-T5-variable-length-ref-arrays.md`.
struct KindLayout {
    /// Reference fields at statically-known offsets (the record case). As registered.
    fixed: Box<[usize]>,
    /// If `Some(start)`, every aligned 8-byte word in `[start, size)` is a reference (the
    /// variable-length array tail). `None` ⇒ pure record. Populated by a later rung
    /// (`register_ref_array_kind`); today every registration sets `None`.
    tail_from: Option<usize>,
}

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
    /// Soft ceiling on `live_bytes` that triggers a collection. Consulted by
    /// [`Self::should_collect`]; re-tuned by [`Self::adapt_threshold`] after each
    /// cycle. See [`INITIAL_THRESHOLD`] / [`MAX_THRESHOLD`].
    collect_threshold: usize,
    /// Adaptive-policy profile shared with the rest of `gc-core`.
    profile: GcProfile,
    /// **Remembered set** for generational collection: payload addresses of
    /// **old** objects that may hold a pointer into the **young** generation.
    /// Populated by [`Self::write_barrier`] on every old-object store; a
    /// [`Self::collect_minor`] scans exactly these old parents (plus the roots)
    /// so it can reclaim young garbage without scanning the whole old generation.
    /// Cleared by every **full** [`Self::collect`] (which may free old objects,
    /// invalidating entries) and rebuilt lazily by the barrier afterwards.
    remembered: HashSet<usize>,
    /// Per-kind **reference layouts** for *precise* interior tracing, indexed by
    /// `kind_id - 1` (see [`Self::register_kind`]). Entry *k* is the [`KindLayout`] of
    /// kind `k + 1`. When an object carries a registered kind, [`Self::scan_payload`]
    /// follows *only* that layout's reference words instead of scanning every payload
    /// word conservatively — so a look-alike-pointer integer sitting in a non-reference
    /// field no longer pins a phantom child. Kind id `0` is reserved for "opaque / trace
    /// conservatively" and never appears here.
    field_maps: Vec<KindLayout>,
    /// **Tenuring threshold**: a young object is promoted to [`GEN_OLD`] once its
    /// [`FlatHeader::age`] (collections survived) reaches this value. Default
    /// [`DEFAULT_TENURE_AGE`] = `1` reproduces immediate tenuring (survive one
    /// collection → old); a larger value (via [`Self::set_tenure_age`]) keeps
    /// objects young longer to reduce premature tenuring. Never `0` (clamped by the
    /// setter) so an object always tenures after a bounded number of survivals.
    tenure_age: u8,
    /// **To-space arenas** retained by the heap (AOT00-T3 PR-3c). A compacting
    /// collection evacuates movable survivors into a fresh [`Arena`] and moves it here;
    /// the arena's objects are [`FlatHeader::arena_backed`] and live until the whole
    /// arena is dropped (on heap teardown, or — a future optimisation — when a later
    /// compaction re-moves its survivors). Dropped *after* the malloc'd all-list is
    /// freed in [`Drop`], so an arena-backed block is never individually deallocated.
    // Populated + consulted by `collect_compacting` (PR-3c-2); its Drop-time ownership is
    // the safety contract that lets the all-list skip arena-backed blocks.
    #[allow(dead_code)]
    arenas: Vec<Arena>,
    /// **Incremental-mark grey set** (AOT00-T4): objects marked but not yet scanned,
    /// carried *between* [`Self::incremental_step`] slices. In the tri-colour model
    /// white = `!marked`, black = `marked` and off this list, **grey = `marked` and on
    /// this list**. A stop-the-world collector keeps its mark worklist as a function-local
    /// `Vec` (drained before returning); an interruptible collector must persist it here so
    /// a slice can stop mid-drain and the next slice resume. Empty ⇔ no unscanned greys.
    mark_worklist: Vec<*mut FlatHeader>,
    /// True from [`Self::incremental_start`] until [`Self::incremental_finish`] — i.e. while
    /// an incremental mark phase is live. Gates alloc-black (`alloc` sets `marked` to this),
    /// the (future) incremental write barrier's shading, and a debug guard against starting a
    /// second incremental cycle or a full `collect*` mid-phase.
    mark_in_progress: bool,
    /// The **root snapshot** for the current incremental cycle: an incremental mark is rooted
    /// **once**, at `incremental_start`, so later steps never re-read a mutated stack (the
    /// soundness pivot — anything made reachable *after* start is caught by the write barrier,
    /// PR-2, not by re-scanning). Precise slot addresses (`mark_roots`) and conservative spans
    /// (`mark_regions`), exactly the pair [`Self::collect_mixed`] consumes. Cleared at finish.
    mark_roots: Vec<usize>,
    mark_regions: Vec<(*const u8, usize)>,
    /// Persistent state of an **in-progress incremental sweep** (AOT00-T4 §4): the sweep
    /// cursor + running tallies, carried across [`Self::incremental_sweep_step`] calls so the
    /// sweep pause is bounded exactly like the mark. `None` unless a stepped sweep is under
    /// way. [`Self::incremental_finish`] drains any remainder and consumes it.
    sweep_state: Option<SweepState>,
}

/// The resumable state of an incremental sweep — the sweep-phase analogue of the mark's
/// [`FlatHeap::mark_worklist`] (AOT00-T4 §4). The monolithic [`FlatHeap::sweep`] keeps this on
/// its stack; a *stepped* sweep hoists it here so one slice can stop mid-list and the next
/// resume from the same link.
struct SweepState {
    /// The last **kept** block — the resume point. The next slice recomputes its live cursor
    /// as `&mut (*resume_after).next` (or `&mut self.all` when null, i.e. nothing kept yet).
    ///
    /// Crucially this is a pointer *into a malloc'd block* (or null), **not** a `&mut self.all`
    /// re-derived pointer: persisting the latter across slices is Undefined Behaviour, because
    /// each `&mut self` at the next `incremental_sweep_step`/`incremental_finish` call performs a
    /// function-entry retag that invalidates any tag previously derived from `self` (caught by
    /// Miri's Stacked Borrows). A block pointer survives those retags, so each slice re-derives
    /// the `&mut self.all`/`&mut (*resume_after).next` cursor *freshly* under its own `&mut self`.
    resume_after: *mut FlatHeader,
    /// Objects freed / survived / live-payload-bytes accumulated so far this sweep.
    freed: usize,
    survived: usize,
    live: usize,
    /// Heap object count / live bytes captured at sweep start, for the cycle's `GcCycleStats`.
    before: usize,
    prev_live: usize,
}

/// The outcome of visiting one block in a full sweep (see [`FlatHeap::sweep_free_or_keep`]):
/// a live block is **kept** (carrying its live payload bytes) or a dead block is **freed**.
enum SweepVisit {
    Kept(usize),
    Freed,
}

/// Default [`FlatHeap::tenure_age`]: **1** — a survivor tenures on its first
/// surviving collection, i.e. the immediate-tenuring behaviour the generational
/// collector shipped with. Chosen as the default so this rung is purely additive
/// (existing consumers and tests are unchanged); aging is opt-in via
/// [`FlatHeap::set_tenure_age`] and can become the default in a later tuning pass.
pub const DEFAULT_TENURE_AGE: u8 = 1;

/// Debug-assert message for the four stop-the-world `collect*` entries: none may run
/// *between* an [`FlatHeap::incremental_start`] and its [`FlatHeap::incremental_finish`].
/// A full/minor collect mid-incremental-cycle would `sweep` blocks still referenced by the
/// persistent [`FlatHeap::mark_worklist`], leaving dangling worklist pointers a later
/// [`FlatHeap::incremental_step`] would pop (a use-after-free). One incremental cycle must
/// run to `finish` before any other collector — a caller-contract invariant, fenced here in
/// debug builds (AOT00-T4 §1 scope guard).
const INCREMENTAL_MIXING_MSG: &str =
    "stop-the-world collect during an incremental mark phase — drive incremental_step/finish \
     to completion before calling collect*";

/// Initial collection threshold, and the floor `adapt_threshold` never drops
/// below: **1 MiB**. Small enough that a real workload collects promptly, large
/// enough that a burst of tiny allocations does not thrash the collector. Matches
/// `twig_gc.c`'s `GC_INITIAL_THRESHOLD`.
pub const INITIAL_THRESHOLD: usize = 1024 * 1024;

/// Ceiling `adapt_threshold` never grows past: **256 MiB**. Bounds the worst-case
/// live-set overshoot between collections and stops a live-heavy program from
/// doubling the threshold up to `usize::MAX` (which would disable the GC and let
/// an untrusted program exhaust memory). Matches `twig_gc.c`'s `GC_MAX_THRESHOLD`.
pub const MAX_THRESHOLD: usize = 256 * 1024 * 1024;

/// A contiguous, 16-byte-aligned **bump arena** — the *to-space* a compacting
/// collection evacuates movable survivors into (AOT00-T3 PR-3). One `alloc`'d block
/// with a monotonically-advancing cursor; every [`Arena::bump`] rounds up to the
/// object alignment, so a copied [`FlatHeader`] (and its payload at `header + 32`)
/// lands 16-aligned exactly as [`FlatHeap::alloc`] guarantees. Owns its block and
/// frees it on drop.
// Consumed by `collect_compacting` in the next rung (PR-3b); exercised by tests now.
#[allow(dead_code)]
struct Arena {
    /// Base of the owned allocation (16-aligned), or null for a zero-capacity arena.
    base: *mut u8,
    /// Bytes handed out so far (always a multiple of [`ALIGN`]).
    top: usize,
    /// Total capacity in bytes.
    cap: usize,
}

#[allow(dead_code)] // methods land ahead of their `collect_compacting` consumer (PR-3b)
impl Arena {
    /// Allocate an arena of exactly `cap` bytes (rounded to a multiple of [`ALIGN`]).
    /// A `cap` of `0` yields an empty arena that owns nothing (a collection with no
    /// movable survivors). Returns `None` only on allocator failure or a `Layout`
    /// error (a `cap` so large it can't be aligned).
    fn with_capacity(cap: usize) -> Option<Arena> {
        if cap == 0 {
            return Some(Arena { base: ptr::null_mut(), top: 0, cap: 0 });
        }
        let layout = Layout::from_size_align(cap, ALIGN).ok()?;
        // SAFETY: `layout` has non-zero size and a valid power-of-two alignment.
        let base = unsafe { alloc(layout) };
        if base.is_null() {
            return None;
        }
        Some(Arena { base, top: 0, cap })
    }

    /// Reserve `n` bytes at the current 16-aligned cursor and return a pointer to
    /// them, advancing the cursor by `align_up(n, ALIGN)`. Returns `None` if the
    /// reservation would overrun the capacity (which never happens when the arena
    /// was sized to the exact evacuation total). The returned pointer is 16-aligned.
    fn bump(&mut self, n: usize) -> Option<*mut u8> {
        // Round the request up to the object alignment so the *next* object also
        // starts 16-aligned (the base and `top` are always 16-multiples).
        let n16 = n.checked_add(ALIGN - 1)? & !(ALIGN - 1);
        if self.top.checked_add(n16)? > self.cap {
            return None;
        }
        // SAFETY: `top + n16 <= cap`, so `base + top` is within the allocation.
        let p = unsafe { self.base.add(self.top) };
        self.top += n16;
        Some(p)
    }
}

impl Drop for Arena {
    fn drop(&mut self) {
        if !self.base.is_null() {
            // SAFETY: `base` came from `alloc` with this exact size + alignment.
            let layout = Layout::from_size_align(self.cap, ALIGN).expect("arena layout");
            unsafe { dealloc(self.base, layout) };
        }
    }
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
            collect_threshold: INITIAL_THRESHOLD,
            profile: GcProfile::default(),
            remembered: HashSet::new(),
            field_maps: Vec::new(),
            tenure_age: DEFAULT_TENURE_AGE,
            arenas: Vec::new(),
            mark_worklist: Vec::new(),
            mark_in_progress: false,
            mark_roots: Vec::new(),
            mark_regions: Vec::new(),
            sweep_state: None,
        }
    }

    /// Set the **tenuring threshold** — how many collections a young object must
    /// survive before it is promoted to the old generation. `1` (the default) is
    /// immediate tenuring; a larger value keeps objects young longer, so ones that
    /// die in their 2nd/3rd cycle are reclaimed by a cheap *minor* GC instead of
    /// polluting the old generation until a full GC. Clamped to a minimum of `1`
    /// (a `0` threshold is meaningless — an object would tenure before surviving
    /// anything) so tenuring always terminates.
    pub fn set_tenure_age(&mut self, threshold: u8) {
        self.tenure_age = threshold.max(1);
    }

    /// The current tenuring threshold (see [`Self::set_tenure_age`]).
    pub fn tenure_age(&self) -> u8 {
        self.tenure_age
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
            // **Alloc-black during an incremental mark** (AOT00-T4 §5): an object born
            // *while* a mark is in progress is marked (black), so the running cycle — whose
            // reachable snapshot was fixed at `incremental_start` — never sweeps it. It is
            // reclaimed next cycle if it dies. Outside a mark this is the usual `false`.
            (*block).marked = self.mark_in_progress;
            (*block).kind = kind;
            (*block).generation = GEN_YOUNG; // new objects are born young
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

    /// The current collection threshold in bytes (see [`INITIAL_THRESHOLD`]).
    pub fn collect_threshold(&self) -> usize {
        self.collect_threshold
    }

    /// Register a **reference-field map** for one class of object and return the
    /// `kind` id to allocate it with. Objects allocated with that id are traced
    /// **precisely**: [`Self::scan_payload`] follows only the byte offsets in
    /// `field_offsets` (the object's `ref`-typed fields) instead of scanning every
    /// payload word conservatively.
    ///
    /// This is what makes the flat collector serve **typed-object languages**
    /// (Ruby/Python/JS objects, records, tuples): a frontend registers each
    /// object layout's ref-field offsets once at startup, then a look-alike
    /// pointer sitting in an integer field can no longer keep a dead object alive.
    ///
    /// Ids are assigned from **1**; id `0` stays reserved for "opaque / trace
    /// conservatively", so `alloc(n, 0)` (the default) keeps the exact conservative
    /// behaviour. Offsets are taken as-is; an offset whose 8-byte word would run
    /// past an object's payload is skipped at trace time (a malformed map can never
    /// cause an out-of-bounds read).
    ///
    /// # Panics
    ///
    /// Panics only if more than `u16::MAX - 1` kinds are registered — far beyond
    /// any real program.
    pub fn register_kind(&mut self, field_offsets: &[usize]) -> u16 {
        let id = self.field_maps.len() + 1;
        assert!(
            id <= u16::MAX as usize,
            "flat-heap kind registry overflow: more than 65534 kinds registered"
        );
        // A pure record: fixed reference offsets, no variable-length tail. The tail case
        // (`tail_from == Some`) is `register_ref_array_kind`; a `None` tail keeps the tracer
        // behaviour byte-for-byte identical to the pre-`KindLayout` field map.
        self.field_maps.push(KindLayout { fixed: field_offsets.into(), tail_from: None });
        id as u16
    }

    /// Register a **variable-length reference-array** kind and return the `kind` id to allocate
    /// it with. An object of this kind is traced precisely as `fixed` reference fields (at
    /// statically-known offsets, exactly like [`Self::register_kind`]) **followed by a tail
    /// region**: every aligned 8-byte word in `[tail_from, size)` of the *instance's* payload is
    /// a reference. Because the tail's extent follows the instance's own `size`, one kind
    /// describes arrays of *every* length — the thing a fixed offset list cannot express.
    ///
    /// This is what makes the flat collector trace (and, crucially, **relocate**) the dominant
    /// heap object of a real language runtime — a JS `Array`, a Ruby `Array`, a Python `list`, a
    /// Scheme vector, a hash's backing store — *precisely* instead of conservatively. A
    /// conservatively-traced array pins itself and every element it references, so under the
    /// compacting collector nothing moves; a precise array and its elements are movable. See
    /// `code/specs/AOT00-T5-variable-length-ref-arrays.md`.
    ///
    /// **Layout contract (the frontend's responsibility).** Every word in `[tail_from, size)`
    /// must hold a *reference* — a base pointer (payload start, low-3 NaN-box tag permitted) or
    /// null — never an inline non-pointer datum. A packed array of unboxed values must either box
    /// them, choose a `tail_from` that excludes the non-reference region, or stay `kind 0`
    /// (conservative), which is always safe. This mirrors the record-field contract of
    /// [`Self::register_kind`]; a violation is caught in debug builds by the interior-pointer
    /// assertion in the compaction fixup.
    ///
    /// `tail_from` is **rounded up to a multiple of 8** so the tail scan stays 8-aligned (an
    /// element straddling the boundary would otherwise be split). A `tail_from >= size` yields an
    /// empty tail — a well-formed "no elements yet" array — and is safe. `fixed` offsets should
    /// lie before `tail_from`; they are traced regardless (each under the same bounds guard).
    ///
    /// # Panics
    ///
    /// Panics only if more than `u16::MAX - 1` kinds are registered — far beyond any real
    /// program.
    pub fn register_ref_array_kind(&mut self, fixed: &[usize], tail_from: usize) -> u16 {
        let id = self.field_maps.len() + 1;
        assert!(
            id <= u16::MAX as usize,
            "flat-heap kind registry overflow: more than 65534 kinds registered"
        );
        // Round the tail start up to the next 8-byte boundary so every tail word is aligned.
        // `checked_add` guards the (absurd) near-`usize::MAX` argument: on overflow the tail is
        // unreachable anyway (`>= size` for any real object), so an empty `Some(usize::MAX)` (via
        // saturation) is the safe, still-precise choice — never a wrapped small offset that would
        // trace non-reference words.
        let tail = tail_from.checked_add(7).map_or(usize::MAX, |n| n & !7usize);
        self.field_maps.push(KindLayout { fixed: fixed.into(), tail_from: Some(tail) });
        id as u16
    }

    /// Number of registered precise kinds (0 = none; ids run 1..=len).
    pub fn registered_kinds(&self) -> usize {
        self.field_maps.len()
    }

    /// Whether the live set has reached the threshold — i.e. a collection is due.
    ///
    /// This is the *policy* half of paced collection: it answers "should I collect
    /// now?" from live-byte accounting alone, with no knowledge of *where* the
    /// roots are. The *mechanism* half — actually finding the roots and running a
    /// cycle — lives with the caller (for native AOT, `gc-core-capi`'s stack scan),
    /// because only the caller knows how to enumerate its roots. A safepoint or an
    /// allocation consults this and, when true, drives a collect.
    pub fn should_collect(&self) -> bool {
        self.live_bytes >= self.collect_threshold
    }

    /// Re-tune the threshold after a cycle, given the live bytes *before* it.
    ///
    /// The heuristic (ported verbatim from `twig_gc.c`): if **more than half** the
    /// pre-cycle live set survived, the program is holding a large working set —
    /// **double** the threshold (capped at [`MAX_THRESHOLD`]) so we collect less
    /// often and waste less time on low-yield sweeps. Otherwise most of the heap
    /// was garbage — **halve** it (floored at [`INITIAL_THRESHOLD`]) so we collect
    /// sooner and keep the footprint tight.
    ///
    /// ```text
    ///   survived > prev/2  →  threshold = min(threshold*2, MAX)   (grow: retain-heavy)
    ///   survived ≤ prev/2  →  threshold = max(threshold/2, INITIAL) (shrink: garbage-heavy)
    /// ```
    ///
    /// The cap is load-bearing for safety, not just tuning: without it a live-heavy
    /// program could double the threshold toward `usize::MAX`, making
    /// `should_collect` never fire — the GC effectively off, an unbounded
    /// memory-exhaustion vector for untrusted input.
    fn adapt_threshold(&mut self, prev_live: usize) {
        if self.live_bytes > prev_live / 2 {
            self.collect_threshold = (self.collect_threshold * 2).min(MAX_THRESHOLD);
        } else {
            self.collect_threshold = (self.collect_threshold / 2).max(INITIAL_THRESHOLD);
        }
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

    /// Live object counts split by generation, as `(young, old)`. Walks the
    /// all-blocks list — O(n), for tests and profiling. A healthy generational
    /// workload keeps `young` churning while `old` stays comparatively stable.
    pub fn object_count_by_generation(&self) -> (usize, usize) {
        let mut young = 0usize;
        let mut old = 0usize;
        let mut h = self.all;
        // SAFETY: list walk over blocks we own / null terminator.
        while !h.is_null() {
            unsafe {
                if (*h).generation == GEN_OLD {
                    old += 1;
                } else {
                    young += 1;
                }
                h = (*h).next;
            }
        }
        (young, old)
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
        debug_assert!(!self.mark_in_progress, "{}", INCREMENTAL_MIXING_MSG);
        let before = self.object_count();
        let prev_live = self.live_bytes;

        // ── Mark ──────────────────────────────────────────────────────────────
        // Iterative worklist (no recursion → no stack blow-up on deep lists).
        let mut work: Vec<*mut FlatHeader> = Vec::new();
        for &r in roots {
            self.mark_word(r, &mut work, false);
        }
        while let Some(h) = work.pop() {
            self.scan_payload(h, &mut work, false);
        }

        // ── Sweep ─────────────────────────────────────────────────────────────
        let (freed, survived, live, _promoted) = self.sweep(false);
        self.live_bytes = live;
        self.adapt_threshold(prev_live);
        // A full collect may have freed old objects, so a remembered-set entry
        // could now dangle. Under aging, young objects can also *survive* a full
        // collect (they are not all tenured), so real old→young edges may still
        // exist — clearing outright would drop them and a later minor GC would
        // free a live young child (UAF). Instead **prune** only the dead entries;
        // surviving old sources (barrier-recorded and promotion-recorded) are kept.
        self.rebuild_remembered();

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
        debug_assert!(!self.mark_in_progress, "{}", INCREMENTAL_MIXING_MSG);
        let before = self.object_count();
        let prev_live = self.live_bytes;

        let mut work: Vec<*mut FlatHeader> = Vec::new();
        // SAFETY: caller guarantees `[base, base+len)` is readable.
        self.mark_region(base, len, &mut work, false);
        while let Some(h) = work.pop() {
            self.scan_payload(h, &mut work, false);
        }

        let (freed, survived, live, _promoted) = self.sweep(false);
        self.live_bytes = live;
        self.adapt_threshold(prev_live);
        // Full collect: prune only the dead entries from the remembered set (see
        // [`Self::collect`]); aging lets young objects survive, so surviving
        // old→young edges must be kept, not cleared.
        self.rebuild_remembered();

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

    /// **Generational write barrier.** Call this whenever the mutator stores a
    /// heap reference `child` into a field of heap object `parent` (both given as
    /// payload addresses). If `parent` is **old**, it is recorded in the
    /// remembered set so a later [`Self::collect_minor`] scans it for the young
    /// objects it may now reference — the pointers a young-only cycle would
    /// otherwise never see.
    ///
    /// It is **O(1)**: the object's header is always exactly [`HEADER_SIZE`] bytes
    /// before its payload, so the generation is read directly at
    /// `parent - HEADER_SIZE` with no heap search. Only `parent`'s generation is
    /// inspected — `child` is never dereferenced (it may legitimately be null or a
    /// tagged non-pointer immediate). Recording an old parent that did not
    /// actually store a young child is a harmless over-approximation: the minor
    /// scan simply finds no young child there.
    ///
    /// # Safety
    ///
    /// `parent` must be the payload address of a **live GC object** on this heap
    /// (the store target always is). The barrier reads one byte at
    /// `parent - HEADER_SIZE`; a non-heap `parent` would read foreign memory. A
    /// `parent < HEADER_SIZE` (null / tiny) is ignored.
    pub unsafe fn write_barrier(&mut self, parent: usize, child: usize) {
        // (a) Generational remembered-set barrier (unchanged): record an old parent so a
        //     later minor GC scans it for the young objects it may now reference.
        if parent >= HEADER_SIZE {
            // SAFETY: caller guarantees `parent` is a live GC payload, so its header is the
            // 32 bytes immediately before it (the flat-heap layout invariant).
            let parent_gen =
                unsafe { (*((parent - HEADER_SIZE) as *const FlatHeader)).generation };
            if parent_gen == GEN_OLD {
                self.remembered.insert(parent);
            }
        }

        // (b) Incremental **Dijkstra insertion barrier** (AOT00-T4 §5): while an incremental
        //     mark is in progress, shade the stored `child` GREY. This preserves the strong
        //     tri-colour invariant "no black → white": without it, storing a white child into
        //     an already-scanned (black) parent and dropping the child's other in-edge would
        //     strand the child white, and the sweep would free it while it is still live (a
        //     use-after-free). Shading the child (marking + enqueuing) guarantees it is
        //     rescanned, hence retained. `child` may be a raw or NaN-box-tagged heap pointer
        //     (low 3 bits), so both the raw and tag-stripped forms are shaded — the same
        //     candidate discipline as `mark_word`/`push_candidates`. Outside a mark this whole
        //     block is a single predictable-branch no-op (generational path unchanged).
        if self.mark_in_progress {
            // Compute the header pointers first (a `Copy` `*mut`), *then* shade — so the
            // `&self` `find_header` borrow ends before the `&mut self` `shade_grey`.
            let h0 = self.find_header(child);
            self.shade_grey(h0);
            let stripped = child & !0x7usize;
            if stripped != child && stripped != 0 {
                let h1 = self.find_header(stripped);
                self.shade_grey(h1);
            }
        }
    }

    /// Shade a candidate block **grey** for the incremental mark: if `h` is a non-null,
    /// still-**white** (`!marked`) block, mark it and push it onto the grey worklist so a
    /// later [`Self::incremental_step`] scans it. A null `h`, or an already-marked
    /// (grey/black) block, is a no-op — shading is idempotent. Erring toward grey (shading a
    /// child that later proves dead) only retains it one extra cycle (floating garbage), never
    /// a use-after-free.
    ///
    /// # Safety
    /// `h` is null or a live block owned by this heap (as returned by [`Self::find_header`]).
    unsafe fn shade_grey(&mut self, h: *mut FlatHeader) {
        if !h.is_null() && !(*h).marked {
            (*h).marked = true;
            self.mark_worklist.push(h);
        }
    }

    /// Run a **minor** (young-generation-only) collection rooted at `roots`.
    ///
    /// The payoff of the generational split: instead of scanning the whole heap,
    /// a minor cycle traces only (a) the roots and (b) the **remembered set** —
    /// the old objects a [`Self::write_barrier`] flagged as holding old→young
    /// pointers — and reclaims only **young** garbage. Old objects are never
    /// scanned or freed. Young survivors are tenured to old. This is what makes
    /// GC cost track the churny young generation, not the whole live set — the
    /// win for high-allocation-rate languages.
    ///
    /// Correctness rests on the barrier contract: *every* old→young store must
    /// have called [`Self::write_barrier`]. A missed old→young pointer whose only
    /// path to a young object is through that old parent would let the young
    /// object be wrongly freed. The GC upholds its half; the mutator/codegen must
    /// uphold the barrier.
    pub fn collect_minor(&mut self, roots: &[usize]) -> GcCycleStats {
        debug_assert!(!self.mark_in_progress, "{}", INCREMENTAL_MIXING_MSG);
        let before = self.object_count();
        let mut work: Vec<*mut FlatHeader> = Vec::new();
        // Roots — mark only the young objects they reach (old are live).
        for &r in roots {
            self.mark_word(r, &mut work, true);
        }
        self.minor_finish(before, work)
    }

    /// A **minor** collection rooted at a **raw memory region** — the stack-scan
    /// analogue of [`Self::collect_minor`], mirroring [`Self::collect_region`].
    /// `gc-core-capi`'s argument-less `__gc_collect_minor` discovers the live
    /// stack `(base, len)` and hands it here.
    ///
    /// # Safety
    ///
    /// `[base, base + len)` must be readable (or `base` null with `len == 0`),
    /// exactly as for [`Self::collect_region`].
    pub unsafe fn collect_minor_region(&mut self, base: *const u8, len: usize) -> GcCycleStats {
        debug_assert!(!self.mark_in_progress, "{}", INCREMENTAL_MIXING_MSG);
        let before = self.object_count();
        let mut work: Vec<*mut FlatHeader> = Vec::new();
        // SAFETY: caller guarantees `[base, base+len)` is readable.
        self.mark_region(base, len, &mut work, true);
        self.minor_finish(before, work)
    }

    /// Shared tail of a minor cycle: scan the remembered old→young parents, drain
    /// the young worklist, sweep the young generation, and build the stats. Both
    /// [`Self::collect_minor`] and [`Self::collect_minor_region`] call this after
    /// their (slice- vs region-sourced) root mark.
    fn minor_finish(&mut self, before: usize, mut work: Vec<*mut FlatHeader>) -> GcCycleStats {
        // Remembered old parents — scan each for the young children it holds.
        // Snapshot the addresses first so the immutable scan can't alias the set.
        // Each entry is a live old object (a full collect clears the set; a minor
        // never frees old), so its header is at `addr - HEADER_SIZE`.
        let remembered: Vec<usize> = self.remembered.iter().copied().collect();
        for parent in remembered {
            let h = (parent - HEADER_SIZE) as *mut FlatHeader;
            self.scan_payload(h, &mut work, true);
        }
        // Drain: `work` holds only young objects; scan them for more young refs.
        while let Some(h) = work.pop() {
            self.scan_payload(h, &mut work, true);
        }

        // Sweep the young generation only; age/promote survivors.
        let (freed, survived, live, promoted) = self.sweep(true);
        self.live_bytes = live;
        // The remembered set is intentionally *kept*: a minor cycle frees no old
        // object, so no entry dangles. Entries whose young child was promoted are
        // now old→old — stale but harmless (the next minor scan finds no young
        // child) and cleared by the next full collect.
        // Promotion barrier: a young object aged to tenure this minor may now hold
        // an old→young pointer (its child stayed young). Record such promotions so
        // the *next* minor GC visits them — else it would free the live young child.
        self.record_promoted_old_to_young(&promoted);

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

    /// Number of old objects currently in the remembered set (test/introspection).
    pub fn remembered_len(&self) -> usize {
        self.remembered.len()
    }

    /// Collect **precisely** from an enumerated set of exact root-slot addresses.
    ///
    /// This is the payoff of the stack-map rung (precision ladder rung "roots";
    /// see `AOT00-T1-precise-gc.md` §4 and §6.1). Where [`Self::collect_region`]
    /// scans an entire span of stack memory *conservatively* — treating **every**
    /// aligned word as a candidate pointer, so a plain integer whose bit pattern
    /// happens to land inside a live block pins that block for a cycle —
    /// `collect_precise` is told *exactly which slots hold references*. Each
    /// `usize` in `root_slots` is the **address of a slot** (a stack location or a
    /// register-spill cell) that a stack map named as live at the current
    /// safepoint. The collector reads the one word at each address and roots from
    /// it; nothing else in the frame is looked at.
    ///
    /// The result: **no false roots**. An integer sitting one slot over from a
    /// real reference — the classic source of "floating garbage" in a
    /// conservative collector — is never even read, so the object it look-alikes
    /// can be reclaimed. Root-scan cost also drops from O(stack depth) to
    /// O(live roots).
    ///
    /// Interior tracing of the objects thus rooted is unchanged: a registered
    /// `kind` is followed precisely through its ref-field offsets, an unregistered
    /// object conservatively (see [`Self::scan_payload`]). Sweep is in place — no
    /// relocation — so this rung is strictly *additive* precision over
    /// [`Self::collect`] and cannot regress liveness.
    ///
    /// The stack walk that *produces* `root_slots` — unwinding the frame-pointer
    /// chain, matching each frame's return address to its [`StackMapTable`] record
    /// (§4.2), and computing `frame_base + slot_offset` for every named slot (see
    /// [`frame_root_slots`]) — is the platform-specific half, layered on top in
    /// `gc-core-capi` exactly as the conservative C-stack scan layers on
    /// [`Self::collect_region`]. This method is the platform-independent,
    /// unit-testable precise-mark core.
    ///
    /// A frame the walker could **not** map (a C runtime frame, or an
    /// un-migrated backend) is handled by the caller falling back to
    /// [`Self::collect_region`] over that frame's span — precision is lost only
    /// there, never correctness.
    ///
    /// # Safety
    ///
    /// Every address in `root_slots` must be readable for the duration of the
    /// call (each names a live stack/register-spill slot). The 8-byte read at each
    /// is `read_unaligned`, so no alignment is required. Passing an address that
    /// is not a valid, readable slot is undefined behaviour — the same contract
    /// [`Self::collect_region`] places on its `[base, base + len)` span.
    pub unsafe fn collect_precise(&mut self, root_slots: &[usize]) -> GcCycleStats {
        debug_assert!(!self.mark_in_progress, "{}", INCREMENTAL_MIXING_MSG);
        let before = self.object_count();
        let prev_live = self.live_bytes;

        // ── Mark ──────────────────────────────────────────────────────────────
        // Read exactly the named slots — no whole-region scan. Each slot's word is
        // a candidate root, run through the same raw-plus-tag-stripped lookup as
        // every other root so NaN-boxed references resolve identically.
        let mut work: Vec<*mut FlatHeader> = Vec::new();
        for &slot in root_slots {
            // SAFETY: caller guarantees each `slot` address is readable; the word
            // there is a candidate root. `read_unaligned` tolerates sub-alignment.
            let word = unsafe { ptr::read_unaligned(slot as *const usize) };
            self.mark_word(word, &mut work, false);
        }
        while let Some(h) = work.pop() {
            self.scan_payload(h, &mut work, false);
        }

        // ── Sweep ─────────────────────────────────────────────────────────────
        let (freed, survived, live, _promoted) = self.sweep(false);
        self.live_bytes = live;
        self.adapt_threshold(prev_live);
        // Full collect: prune only the dead entries from the remembered set (see
        // [`Self::collect`]); aging lets young objects survive, so surviving
        // old→young edges must be kept, not cleared.
        self.rebuild_remembered();

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

    /// Collect from **both** exact root slots *and* raw conservative regions in a
    /// single cycle — the primitive a precise **stack walk** needs when only some
    /// frames are stack-mapped.
    ///
    /// [`Self::collect_precise`] roots from named slots only; [`Self::collect_region`]
    /// roots from one raw span only. But a real native stack walk sees a *mix*: the
    /// frames a migrated backend emitted stack maps for contribute exact
    /// `root_slots` (via [`frame_root_slots`]), while the frames it could **not**
    /// map — a C-runtime frame, the collector's own frames, a not-yet-migrated
    /// backend — must be scanned **conservatively**, each contributing its whole
    /// span as one `(base, len)` region. Those two roots must be marked in the *same*
    /// mark phase and reclaimed by the *same* sweep, because the heap has one live
    /// set; you cannot precise-collect then region-collect (the first sweep would
    /// free everything the second's roots keep). This method is that single cycle:
    /// mark every `root_slots` word (exactly as `collect_precise`) **and** every
    /// candidate word in every region (exactly as `collect_region`), then sweep once.
    ///
    /// It is the strict generalisation of both siblings — `collect_precise(slots)`
    /// is `collect_mixed(slots, &[])`, and `collect_region(base, len)` is
    /// `collect_mixed(&[], &[(base, len)])`. Precision is per-frame: a mapped frame
    /// pins only its real references, an unmapped frame conservatively pins whatever
    /// its span look-alikes — so adding precise coverage to more backends strictly
    /// reduces floating garbage, and a frame with no map is never *less* safe than
    /// today's fully-conservative scan. Interior tracing, in-place sweep, remembered-
    /// set clearing and threshold adaptation are all identical to the two siblings.
    ///
    /// # Safety
    ///
    /// Every address in `root_slots` must be readable for the call (each names a
    /// live stack / register-spill slot; the word is read `unaligned`). Every
    /// `(base, len)` region must be readable for the call (`base` may be null iff
    /// `len == 0`; no alignment required). This is exactly the union of the
    /// [`Self::collect_precise`] and [`Self::collect_region`] contracts — the walker
    /// derived both from real frames it unwound.
    pub unsafe fn collect_mixed(
        &mut self,
        root_slots: &[usize],
        regions: &[(*const u8, usize)],
    ) -> GcCycleStats {
        debug_assert!(!self.mark_in_progress, "{}", INCREMENTAL_MIXING_MSG);
        let before = self.object_count();
        let prev_live = self.live_bytes;

        // ── Mark ──────────────────────────────────────────────────────────────
        let mut work: Vec<*mut FlatHeader> = Vec::new();
        // Exact slots first (precise frames): read the one word at each address.
        for &slot in root_slots {
            // SAFETY: caller guarantees each `slot` address is readable; the word
            // there is a candidate root. `read_unaligned` tolerates sub-alignment.
            let word = unsafe { ptr::read_unaligned(slot as *const usize) };
            self.mark_word(word, &mut work, false);
        }
        // Then whole regions (unmapped frames): every aligned word is a candidate.
        for &(base, len) in regions {
            // SAFETY: caller guarantees `[base, base+len)` is readable.
            self.mark_region(base, len, &mut work, false);
        }
        while let Some(h) = work.pop() {
            self.scan_payload(h, &mut work, false);
        }

        // ── Sweep ─────────────────────────────────────────────────────────────
        let (freed, survived, live, _promoted) = self.sweep(false);
        self.live_bytes = live;
        self.adapt_threshold(prev_live);
        // Full collect: prune only the dead entries from the remembered set (see
        // [`Self::collect`]); aging lets young objects survive, so surviving
        // old→young edges must be kept, not cleared.
        self.rebuild_remembered();

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

    // ── Incremental (bounded-pause) collector — tri-colour marking (AOT00-T4) ──
    //
    // The stop-the-world `collect_mixed` above marks the *entire* live set then sweeps in one
    // indivisible call. The three methods below decompose the **mark** into bounded slices so
    // the mutator sees short pauses instead of one long one, while a persistent grey worklist
    // ([`FlatHeap::mark_worklist`]) holds the tri-colour invariant across the gaps. This is
    // PR-1: the interruptible mark itself, driven by a cooperative single-shot driver that
    // does not mutate the heap *during* a mark (so no write barrier is needed yet — that is
    // PR-2). Colours: white = `!marked`, grey = `marked` ∧ on the worklist, black = `marked`
    // ∧ off the worklist. Invariant to preserve at `finish`: every reachable object is black,
    // so every white object is unreachable and safe to sweep.

    /// **Begin** an incremental collection (AOT00-T4 §4): colour every object white, snapshot
    /// the roots for the whole phase, and shade the root-reachable objects grey.
    ///
    /// Rooting **once**, here, is the soundness pivot: later [`Self::incremental_step`] slices
    /// never re-read a possibly-mutated stack. (Anything the mutator makes reachable *after*
    /// this point is the write barrier's job — PR-2; a PR-1 driver simply does not mutate the
    /// heap mid-mark.) After this call the worklist holds the grey frontier (the roots).
    ///
    /// # Safety
    /// Each `root_slots` address and each `regions` span must be readable (same contract as
    /// [`Self::collect_mixed`]). Must not be called while a mark is already in progress.
    pub unsafe fn incremental_start(
        &mut self,
        root_slots: &[usize],
        regions: &[(*const u8, usize)],
    ) {
        debug_assert!(!self.mark_in_progress, "incremental cycle already in progress");
        // Everything white: clear the mark bit on every live block.
        // SAFETY: list walk over blocks we own / null terminator.
        let mut h = self.all;
        while !h.is_null() {
            (*h).marked = false;
            h = (*h).next;
        }
        // Snapshot the roots for the entire phase; entering the phase makes `alloc` born-black.
        self.mark_roots = root_slots.to_vec();
        self.mark_regions = regions.to_vec();
        self.mark_in_progress = true;

        // Grey the roots: mark each root-reachable block and push it onto the worklist
        // (`mark_word`/`mark_region` set `marked` and enqueue only previously-white blocks).
        let mut work: Vec<*mut FlatHeader> = Vec::new();
        for &slot in root_slots {
            let word = ptr::read_unaligned(slot as *const usize);
            self.mark_word(word, &mut work, false);
        }
        for &(base, len) in regions {
            self.mark_region(base, len, &mut work, false);
        }
        self.mark_worklist = work;
    }

    /// **Advance** marking by up to `budget` objects (AOT00-T4 §4) — the sole bounded-pause
    /// primitive. Pops grey objects, scans each (greying its still-white children), turning it
    /// black. Returns `true` once the worklist empties (marking complete → call
    /// [`Self::incremental_finish`]), `false` if more work remains. `budget` bounds the work
    /// per slice: at most `budget` objects are scanned, so the pause scales with `budget`.
    ///
    /// # Safety
    /// Must be called only between [`Self::incremental_start`] and
    /// [`Self::incremental_finish`]. Every worklist pointer is a live block owned by this heap
    /// (nothing frees a block mid-cycle in PR-1's non-mutating driver).
    pub unsafe fn incremental_step(&mut self, budget: usize) -> bool {
        debug_assert!(self.mark_in_progress, "incremental_step outside a mark phase");
        // Take the worklist out so `scan_payload(&self, …)` can borrow `self` immutably while
        // we push newly-greyed children into the (now local) list.
        //
        // SOUND ONLY UNDER THE SINGLE-THREADED, NON-REENTRANT DRIVER: while the list is taken
        // out, `self.mark_worklist` is empty, and the reassignment below overwrites it. A
        // `write_barrier` shade (PR-2) that pushed to `self.mark_worklist` *during* this window
        // would be clobbered — the child lost, then swept (a UAF). That never happens because
        // the mutator (hence the barrier) runs strictly *between* steps, never inside one:
        // gc-core is single-threaded and `incremental_step` calls nothing that re-enters the
        // mutator. This `take`/reassign is the one construct that would have to change first if
        // this collector ever went reentrant or multi-threaded.
        let mut work = core::mem::take(&mut self.mark_worklist);
        let mut scanned = 0usize;
        while scanned < budget {
            let h = match work.pop() {
                Some(h) => h, // grey → (scan) → black
                None => break,
            };
            // Grey every still-white child: `scan_payload` follows the object's ref fields
            // (precise for a registered kind, conservative otherwise) via `mark_word`, which
            // marks + enqueues only white targets — exactly the tri-colour shade step.
            self.scan_payload(h, &mut work, false);
            scanned += 1;
        }
        let done = work.is_empty();
        self.mark_worklist = work;
        done
    }

    /// **Advance the sweep** by up to `budget` blocks (AOT00-T4 §4) — the sweep-phase analogue
    /// of [`Self::incremental_step`], so the *sweep* pause is bounded just like the mark. Frees
    /// each unreachable (white) block and ages each survivor, resuming from a persistent cursor
    /// ([`FlatHeap::sweep_state`]); returns `true` once the whole all-list has been swept.
    ///
    /// Optional: a caller that wants a fully bounded cycle drives
    /// `start → step* → sweep_step* → finish`; a caller happy with a monolithic sweep just
    /// calls `finish` directly (which sweeps whatever the stepped sweep left, including all of
    /// it). Marking must be complete before the first sweep step.
    ///
    /// # Budget & mutation. `budget` bounds *blocks visited* (freed or kept) per call, so the
    /// pause scales with `budget`. Objects allocated between sweep steps are born **black**
    /// (`mark_in_progress` is still set) and are prepended ahead of the cursor, so the running
    /// sweep never visits or frees them — they simply survive to the next cycle.
    ///
    /// # Safety
    /// Called only after [`Self::incremental_start`] with marking complete; single mutator; no
    /// other collection runs mid-cycle.
    pub unsafe fn incremental_sweep_step(&mut self, budget: usize) -> bool {
        debug_assert!(self.mark_in_progress, "incremental_sweep_step outside a mark phase");
        debug_assert!(self.mark_worklist.is_empty(), "sweep before marking is complete");
        // Lazily begin the sweep on the first call: resume at the list head, tallies zeroed.
        if self.sweep_state.is_none() {
            let before = self.object_count();
            let prev_live = self.live_bytes;
            self.sweep_state = Some(SweepState {
                resume_after: ptr::null_mut(),
                freed: 0,
                survived: 0,
                live: 0,
                before,
                prev_live,
            });
        }
        // Operate on the state out-of-band so `sweep_free_or_keep` (which borrows nothing of
        // `self`) and the raw-pointer cursor writes don't collide with the field borrow.
        let mut st = self.sweep_state.take().expect("just initialised");
        let tenure = self.tenure_age;
        // Promotions are subsumed by the finish-time `rebuild_remembered`, so a throwaway
        // scratch suffices (matching how `collect_mixed` discards `sweep`'s `promoted`).
        let mut promoted_scratch: Vec<*mut FlatHeader> = Vec::new();
        // Re-derive the live cursor *freshly* under this call's `&mut self` from the persisted
        // resume block — never carry a `self`-derived pointer across the call boundary.
        let mut cursor: *mut *mut FlatHeader = if st.resume_after.is_null() {
            &mut self.all
        } else {
            &mut (*st.resume_after).next
        };
        let mut visited = 0usize;
        while visited < budget && !(*cursor).is_null() {
            let h = *cursor;
            // Read the successor *before* `sweep_free_or_keep`, whose `Freed` arm deallocates
            // `h` — touching `(*h).next` afterward would be a use-after-free.
            let next = (*h).next;
            match Self::sweep_free_or_keep(h, tenure, &mut promoted_scratch) {
                SweepVisit::Kept(sz) => {
                    st.survived += 1;
                    st.live += sz;
                    st.resume_after = h;
                    cursor = &mut (*h).next;
                }
                SweepVisit::Freed => {
                    *cursor = next;
                    st.freed += 1;
                }
            }
            visited += 1;
        }
        let done = (*cursor).is_null();
        self.sweep_state = Some(st);
        done
    }

    /// Whether an incremental **sweep** is currently in progress (test/introspection).
    pub fn incremental_sweeping(&self) -> bool {
        self.sweep_state.is_some()
    }

    /// **Finish** the incremental cycle (AOT00-T4 §4): ensure the sweep is complete (draining
    /// any remainder — or doing the whole sweep monolithically if the caller never stepped it),
    /// rebuild the remembered set, adapt the threshold, and end the phase. Marking must already
    /// be complete (the worklist empty). Returns the cycle's [`GcCycleStats`].
    ///
    /// This is robust to either drive style: `start → step* → finish` (finish sweeps
    /// monolithically) **or** `start → step* → sweep_step* → finish` (finish just consumes the
    /// stepped tallies, draining any blocks a short final `sweep_step` left).
    ///
    /// # Safety
    /// Called only after [`Self::incremental_start`] with marking complete.
    pub unsafe fn incremental_finish(&mut self) -> GcCycleStats {
        debug_assert!(self.mark_in_progress, "incremental_finish outside a mark phase");
        debug_assert!(self.mark_worklist.is_empty(), "marking not complete — step to done first");

        let (freed, survived, before, prev_live) = if let Some(mut st) = self.sweep_state.take() {
            // A stepped sweep is under way. Drain any remaining blocks monolithically so finish
            // always leaves the all-list fully swept, whatever the caller's last `sweep_step`
            // budget was (or if they never reached the end).
            let tenure = self.tenure_age;
            // Promotions are recomputed by `rebuild_remembered` below, so a throwaway suffices.
            let mut promoted_scratch: Vec<*mut FlatHeader> = Vec::new();
            // Re-derive the cursor freshly under this `&mut self` from the persisted resume block.
            let mut cursor: *mut *mut FlatHeader = if st.resume_after.is_null() {
                &mut self.all
            } else {
                &mut (*st.resume_after).next
            };
            while !(*cursor).is_null() {
                let h = *cursor;
                // As in `incremental_sweep_step`: capture the successor before the helper's
                // `Freed` arm frees `h`.
                let next = (*h).next;
                match Self::sweep_free_or_keep(h, tenure, &mut promoted_scratch) {
                    SweepVisit::Kept(sz) => {
                        st.survived += 1;
                        st.live += sz;
                        st.resume_after = h;
                        cursor = &mut (*h).next;
                    }
                    SweepVisit::Freed => {
                        *cursor = next;
                        st.freed += 1;
                    }
                }
            }
            self.live_bytes = st.live;
            (st.freed, st.survived, st.before, st.prev_live)
        } else {
            // No stepped sweep — do the whole sweep now (identical to a full `collect_mixed`).
            let before = self.object_count();
            let prev_live = self.live_bytes;
            let (freed, survived, live, _promoted) = self.sweep(false);
            self.live_bytes = live;
            (freed, survived, before, prev_live)
        };

        self.adapt_threshold(prev_live);
        self.rebuild_remembered();

        // End the phase; drop the root snapshot, the (already-empty) worklist, and sweep state.
        self.mark_in_progress = false;
        self.mark_roots.clear();
        self.mark_regions.clear();
        self.mark_worklist.clear();
        self.sweep_state = None;

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

    /// Whether an incremental mark phase is currently in progress (test/introspection).
    pub fn incremental_in_progress(&self) -> bool {
        self.mark_in_progress
    }

    /// Size of the grey frontier — objects marked but not yet scanned (test/introspection).
    /// `0` while no mark is in progress or once marking is complete.
    pub fn incremental_grey_count(&self) -> usize {
        self.mark_worklist.len()
    }

    /// Scan `[base, base + len)` as an array of aligned candidate root words,
    /// enqueuing every live block a word points into. The region-oriented sibling
    /// of [`Self::scan_payload`] (which scans one object's payload); both defer to
    /// [`Self::mark_word`] for the raw-plus-tag-stripped lookup.
    ///
    /// # Safety
    ///
    /// `[base, base + len)` must be readable.
    unsafe fn mark_region(
        &self,
        base: *const u8,
        len: usize,
        work: &mut Vec<*mut FlatHeader>,
        young_only: bool,
    ) {
        let mut off = 0usize;
        while off + 8 <= len {
            // SAFETY: `off + 8 <= len`, so the 8-byte read stays inside the region;
            // `read_unaligned` tolerates any sub-alignment of `base`.
            let word = ptr::read_unaligned(base.add(off) as *const usize);
            self.mark_word(word, work, young_only);
            off += 8;
        }
    }

    /// Mark the block a candidate `word` points into, if any, and enqueue it.
    /// Checks both the raw word and the tag-stripped word (NaN-box compat).
    fn mark_word(&self, word: usize, work: &mut Vec<*mut FlatHeader>, young_only: bool) {
        // Raw candidate.
        self.mark_candidate(self.find_header(word), work, young_only);
        // Tag-stripped candidate (low 3 bits are a NaN-box tag on dyn values).
        let stripped = word & !0x7usize;
        if stripped != word && stripped != 0 {
            self.mark_candidate(self.find_header(stripped), work, young_only);
        }
    }

    /// Mark the candidate block `h` (if non-null) and enqueue it for scanning.
    ///
    /// In a **minor** cycle (`young_only`), old objects are never marked or
    /// traversed: they are assumed live (a minor GC does not sweep them) and any
    /// old→young pointers they hold are reached instead through the remembered
    /// set. Skipping them here also avoids leaving a stale mark bit on an old
    /// object that a later full collect would misread as reachable.
    fn mark_candidate(
        &self,
        h: *mut FlatHeader,
        work: &mut Vec<*mut FlatHeader>,
        young_only: bool,
    ) {
        if h.is_null() {
            return;
        }
        // SAFETY: `find_header` only returns headers of live blocks we own.
        unsafe {
            if young_only && (*h).generation != GEN_YOUNG {
                return;
            }
            if !(*h).marked {
                (*h).marked = true;
                work.push(h);
            }
        }
    }

    /// Scan `h`'s payload for further candidate pointers.
    ///
    /// If the object carries a **registered kind** (`kind != 0` with a field map
    /// from [`Self::register_kind`]), only that kind's `ref`-field offsets are
    /// followed — **precise** interior tracing, so a look-alike-pointer integer in
    /// a non-reference field pins nothing. Otherwise the whole payload is scanned
    /// **conservatively** (kind `0`, or a kind id with no map — a safe fallback
    /// that never under-traces).
    ///
    /// `young_only` is threaded to [`Self::mark_candidate`]: in a minor cycle,
    /// children that resolve to old objects are ignored (they are live and not
    /// swept), so only young children are followed.
    /// Invoke `f(slot)` for each **precise reference slot** of a *registered-kind* object `h` —
    /// first its [`KindLayout::fixed`] offsets (the record case), then, if the kind has a tail
    /// region, every aligned 8-byte word in `[tail_from, size)` (the variable-length array
    /// case). Returns `true` iff `h` had a registered kind (so precise tracing applied);
    /// returns `false` for `kind == 0` / an unregistered id, leaving the caller to trace
    /// **conservatively** if it needs to (the mark and remembered-set sites do; the compaction
    /// sites treat `kind == 0` as *"holds no precise reference"* and do nothing).
    ///
    /// This is the single place the [`KindLayout`] is walked, so the four consumers
    /// ([`Self::scan_payload`], [`Self::precise_children`], [`Self::fixup_ref_fields`],
    /// [`Self::points_to_live_young`]) cannot disagree about *which words are references* — the
    /// co-totality that keeps mark, relocate, and remembered-set in lockstep. Each `slot` is a
    /// `*mut usize` inside `h`'s payload whose 8-byte access is in bounds; the callback reads
    /// (and, for fixup, writes) the word.
    ///
    /// # Safety
    /// `h` is a live block owned by this heap. The wrap-safe bound `off <= size - 8` (never
    /// `off + 8 <= size`, which could overflow for a near-`usize::MAX` offset from a bad map)
    /// keeps every produced `slot` inside the payload.
    unsafe fn for_each_ref_slot(&self, h: *mut FlatHeader, mut f: impl FnMut(*mut usize)) -> bool {
        let kind = (*h).kind;
        if kind == 0 {
            return false;
        }
        let layout = match self.field_maps.get((kind - 1) as usize) {
            Some(l) => l,
            None => return false,
        };
        let base = h.add(1) as *mut u8;
        let size = (*h).size;
        // Fixed reference fields (the record case).
        for &off in layout.fixed.iter() {
            if size >= 8 && off <= size - 8 {
                f(base.add(off) as *mut usize);
            }
        }
        // Variable-length reference tail (the array case): every aligned word in
        // `[tail_from, size)`. Empty when `tail_from` is past `size - 8`. `None` today.
        if let Some(start) = layout.tail_from {
            let mut off = start;
            while size >= 8 && off <= size - 8 {
                f(base.add(off) as *mut usize);
                off += 8;
            }
        }
        true
    }

    fn scan_payload(&self, h: *mut FlatHeader, work: &mut Vec<*mut FlatHeader>, young_only: bool) {
        // Precise path: an object of a registered kind is traced through exactly its
        // reference slots (fixed offsets + any tail region). `kind` ids are 1-based
        // (`0` = conservative). `for_each_ref_slot` returns `true` when it handled `h`.
        // SAFETY: `h` is a live block we own; `for_each_ref_slot` bounds every slot access.
        let precise = unsafe {
            self.for_each_ref_slot(h, |slot| {
                // SAFETY: `slot` is an in-bounds 8-byte reference word; `read_unaligned`
                // tolerates sub-alignment.
                let word = ptr::read_unaligned(slot);
                self.mark_word(word, work, young_only);
            })
        };
        if precise {
            return;
        }

        // Conservative fallback (kind 0 / unregistered): scan every aligned word.
        // SAFETY: `h` is a live block; its payload is `size` bytes at `h + 32`.
        let (base, size) = unsafe { ((h.add(1)) as *const u8, (*h).size) };
        let mut off = 0usize;
        while off + 8 <= size {
            // SAFETY: `off + 8 <= size`, so the 8-byte read stays inside the
            // payload.  `read_unaligned` tolerates any payload sub-alignment.
            let word = unsafe { ptr::read_unaligned(base.add(off) as *const usize) };
            self.mark_word(word, work, young_only);
            off += 8;
        }
    }

    // ── Moving/compacting collector — mobility classification (AOT00-T3 PR-2) ──
    //
    // No relocation yet: these compute *which* objects a future copying collector
    // may relocate and which must stay put. Getting the pin/move decision right is
    // the use-after-free surface, so it is landed and tested on its own first.

    /// Append the live block(s) the candidate `word` points into (raw **and** its
    /// low-3-tag-stripped form, matching [`Self::mark_word`]) to `out`.
    fn push_candidates(&self, word: usize, out: &mut Vec<*mut FlatHeader>) {
        let h = self.find_header(word);
        if !h.is_null() {
            out.push(h);
        }
        let stripped = word & !0x7usize;
        if stripped != word && stripped != 0 {
            let h2 = self.find_header(stripped);
            if !h2.is_null() {
                out.push(h2);
            }
        }
    }

    /// Append `h`'s **precise** children — the blocks its *registered-kind reference
    /// fields* point at. A `kind == 0` object (no field map) contributes **nothing**
    /// here: its out-edges are conservative and are handled by the pinning wave.
    ///
    /// # Safety
    /// `h` is a live block owned by this heap.
    unsafe fn precise_children(&self, h: *mut FlatHeader, out: &mut Vec<*mut FlatHeader>) {
        // A `kind == 0` / unregistered object contributes no *precise* children (its out-edges
        // are conservative and handled by the pinning wave), so ignore the returned flag.
        self.for_each_ref_slot(h, |slot| {
            let word = ptr::read_unaligned(slot);
            self.push_candidates(word, out);
        });
    }

    /// Append every block any aligned word of `h`'s payload points at — the
    /// **conservative** children (every word is a maybe-pointer).
    ///
    /// # Safety
    /// `h` is a live block owned by this heap.
    unsafe fn conservative_children(&self, h: *mut FlatHeader, out: &mut Vec<*mut FlatHeader>) {
        let base = h.add(1) as *const u8;
        let size = (*h).size;
        let mut off = 0usize;
        while off + 8 <= size {
            let word = ptr::read_unaligned(base.add(off) as *const usize);
            self.push_candidates(word, out);
            off += 8;
        }
    }

    /// Classify every live object as **movable** or **pinned** for the moving
    /// collector (AOT00-T3 PR-2) — *without relocating anything*. Returns the set of
    /// movable objects' **payload addresses**.
    ///
    /// An object is **movable** iff it is
    /// - **precise-reachable**: reached from `root_slots` (the exact slot addresses a
    ///   precise stack map names, as [`Self::collect_mixed`]) following **only**
    ///   registered-kind reference edges — so every object on the path is one whose
    ///   pointers can be rewritten; **and**
    /// - **not pinned**: no `regions` (conservative) root reaches it, and it is not a
    ///   child of any `kind == 0` object — a maybe-pointer to it could not be safely
    ///   rewritten if it moved; **and**
    /// - a **registered kind** (`kind != 0`): so its *own* pointers can be updated
    ///   after it moves.
    ///
    /// Everything else is **pinned** (its header [`FlatHeader::pinned`] bit is set).
    /// This is the simple, always-sound model (spec §2): *any* conservative in-edge
    /// pins — when unsure, pin. Erring toward pinning is safe; a pinned object
    /// mis-classified as movable would be a use-after-free once relocation lands
    /// (a stale conservative pointer to its old address), so the predicate is a
    /// deliberate over-approximation of "cannot move".
    ///
    /// # Safety
    /// Each `root_slots` address and each `regions` span must be readable (same
    /// contract as [`Self::collect_mixed`]).
    pub unsafe fn classify_mobility(
        &mut self,
        root_slots: &[usize],
        regions: &[(*const u8, usize)],
    ) -> HashSet<usize> {
        // Pin bits are a per-classification transient: clear them first.
        {
            let mut h = self.all;
            while !h.is_null() {
                (*h).pinned = false;
                h = (*h).next;
            }
        }

        // ── Precise wave ───────────────────────────────────────────────────────
        // From each precise slot's word, follow ONLY registered-kind reference
        // edges. A kind==0 object reached is precise-reachable but its (conservative)
        // out-edges are left for the pinning wave.
        let mut precise: HashSet<usize> = HashSet::new();
        let mut work: Vec<*mut FlatHeader> = Vec::new();
        let mut tmp: Vec<*mut FlatHeader> = Vec::new();
        for &slot in root_slots {
            let word = ptr::read_unaligned(slot as *const usize);
            self.push_candidates(word, &mut tmp);
        }
        for h in tmp.drain(..) {
            if precise.insert(h as usize) {
                work.push(h);
            }
        }
        while let Some(h) = work.pop() {
            self.precise_children(h, &mut tmp);
            for c in tmp.drain(..) {
                if precise.insert(c as usize) {
                    work.push(c);
                }
            }
        }

        // ── Pinning (conservative) wave ────────────────────────────────────────
        // Seeds: every conservative-region candidate, plus every precise-reachable
        // kind==0 object (its conservative out-edges make its children unmovable, and
        // it is itself unmovable). Then trace conservatively — every reached object is
        // pinned, and every word it holds is a candidate that pins its target.
        let mut cwork: Vec<*mut FlatHeader> = Vec::new();
        for &(base, len) in regions {
            let mut off = 0usize;
            while off + 8 <= len {
                let word = ptr::read_unaligned(base.add(off) as *const usize);
                self.push_candidates(word, &mut cwork);
                off += 8;
            }
        }
        for &ph in &precise {
            let h = ph as *mut FlatHeader;
            if (*h).kind == 0 {
                cwork.push(h);
            }
        }
        while let Some(h) = cwork.pop() {
            if (*h).pinned {
                continue;
            }
            (*h).pinned = true;
            self.conservative_children(h, &mut tmp);
            for c in tmp.drain(..) {
                cwork.push(c);
            }
        }

        // ── Result: movable = precise-reachable, not pinned, registered kind ────
        let mut movable: HashSet<usize> = HashSet::new();
        for &ph in &precise {
            let h = ph as *mut FlatHeader;
            if !(*h).pinned && (*h).kind != 0 {
                movable.insert(h as usize + HEADER_SIZE); // payload address
            }
        }
        movable
    }

    /// **PR-3a scaffold** for the compacting collector: classify mobility, then copy every
    /// MOVABLE object (header + payload) verbatim into a fresh to-space [`Arena`], returning
    /// the arena and a **forwarding map** from each moved object's *old* payload address to
    /// its *new* payload address in the arena.
    ///
    /// This is steps 1–2 of the moving cycle (spec §4) — the mark and the copy — and nothing
    /// else. It does **not** fix up any pointer and does **not** free anything: the from-space
    /// originals are untouched and the arena copies still hold stale (old-address) pointers.
    /// It exists so the arena / copy / forwarding-map mechanics land and are reviewed in
    /// isolation, before the pointer fixup (PR-3b) and from-space reclamation (PR-3c) wire it
    /// into a live collection. Pinned objects are never copied. Because the arena is returned
    /// (and normally dropped by the caller), this leaves the heap unchanged — an observable
    /// dry run.
    ///
    /// # Safety
    /// Each `root_slots` address and each `regions` span must be readable (same contract as
    /// [`Self::collect_mixed`]).
    // The full `collect_compacting` (fixup + reclaim) consumes this in PR-3b/c.
    #[allow(dead_code)]
    unsafe fn plan_compaction(
        &mut self,
        root_slots: &[usize],
        regions: &[(*const u8, usize)],
    ) -> (Arena, HashMap<usize, usize>) {
        let movable = self.classify_mobility(root_slots, regions);

        // Size the arena to the exact evacuation total: Σ align16(HEADER_SIZE + size).
        // Saturating throughout (every summand fits — each object already occupies live
        // memory — but saturating makes the no-wrap invariant explicit and fails safe: a
        // saturated `total` just makes `Arena::with_capacity` fail below, panicking rather
        // than under-sizing. The real bounds guard is `bump`'s capacity check regardless.
        let mut total = 0usize;
        for &payload in &movable {
            let h = (payload - HEADER_SIZE) as *mut FlatHeader;
            let obj = HEADER_SIZE.saturating_add((*h).size);
            let obj16 = obj.saturating_add(ALIGN - 1) & !(ALIGN - 1);
            total = total.saturating_add(obj16);
        }

        let mut arena = Arena::with_capacity(total).expect("arena allocation");
        let mut forward: HashMap<usize, usize> = HashMap::new();
        for &payload in &movable {
            let h = (payload - HEADER_SIZE) as *mut FlatHeader;
            let obj = HEADER_SIZE + (*h).size;
            let dst = arena.bump(obj).expect("arena sized to the evacuation total");
            // Copy header + payload verbatim. Source (a malloc'd from-space block) and
            // destination (the fresh arena) never overlap.
            ptr::copy_nonoverlapping(h as *const u8, dst, obj);
            // The copy lives in the arena — mark its provenance so no `dealloc` site ever
            // frees it individually (its storage belongs to the whole arena).
            (*(dst as *mut FlatHeader)).arena_backed = true;
            let new_payload = dst as usize + HEADER_SIZE;
            forward.insert(payload, new_payload);
        }
        (arena, forward)
    }

    /// If `word` is a pointer at a **moved** object's *base* (old payload address) —
    /// either raw, or carrying a low-3 NaN-box tag — return the forwarded pointer (new
    /// base, tag reattached); otherwise `None`.
    ///
    /// Only **base** pointers are handled, deliberately: a moved object is referenced
    /// *only* by base pointers. Precise reference fields hold base pointers, and any
    /// object reachable *conservatively* (a `regions` root, or a `kind == 0` object)
    /// was scanned every-word during classification, which **pins** its targets — so a
    /// moved object has no interior-pointer and no conservative in-edge. An interior or
    /// integer look-alike therefore never legitimately names a moved object, and is
    /// never rewritten (avoiding corrupting a non-pointer word).
    fn forwarded(&self, word: usize, forward: &HashMap<usize, usize>) -> Option<usize> {
        if let Some(&nw) = forward.get(&word) {
            return Some(nw); // untagged base pointer
        }
        let tag = word & 0x7;
        if tag != 0 {
            let base = word & !0x7usize;
            if let Some(&nw) = forward.get(&base) {
                return Some(nw | tag); // tagged base pointer — reattach the tag
            }
        }
        None
    }

    /// Rewrite `h`'s **registered-kind reference fields** that point at a moved object
    /// to the forwarded (new) address. `h` must be a registered kind (`kind != 0`) with
    /// a field map — the only objects that hold pointers to moved objects are the moved
    /// objects themselves (their arena copies), and those are registered kinds. Words
    /// that don't name a moved object are left untouched; tag bits are preserved by
    /// [`Self::forwarded`].
    ///
    /// # Safety
    /// `h` is a live block owned by this heap (or its arena copy).
    unsafe fn fixup_ref_fields(&self, h: *mut FlatHeader, forward: &HashMap<usize, usize>) {
        // A `kind == 0` / unregistered object provably holds no pointer to a moved object (its
        // targets were pinned by the conservative pinning wave), so `for_each_ref_slot` visits
        // nothing for it — exactly the old early-return. Rewrite each *precise* reference word
        // (fixed field or array-tail element) that names a moved object.
        self.for_each_ref_slot(h, |slot| {
            let w = ptr::read_unaligned(slot);
            // PR-3b reviewer follow-up: a *precise* reference word holds a reference —
            // a **base** pointer (payload start, low-3 NaN-box tag permitted) or null —
            // never an *interior* pointer. `forwarded` only rewrites base keys, so an
            // interior pointer in a ref word would silently escape relocation and dangle
            // once its target's from-space block is freed. This catches a frontend that
            // declared a ref slot holding an interior/derived pointer (a genuine
            // non-pointer datum belongs in a non-ref field, scanned conservatively, not at
            // a registered ref offset or in a ref tail). Compiled out of release builds;
            // exercised under tests + Miri. `find_header` is a live read (from-space is
            // still intact during fixup, before any sweep).
            #[cfg(debug_assertions)]
            {
                let cand = w & !0x7usize;
                let bh = self.find_header(cand);
                debug_assert!(
                    bh.is_null() || bh as usize + HEADER_SIZE == cand,
                    "precise ref slot holds an interior pointer \
                     (0x{cand:x}); precise ref slots must hold base/tagged-base pointers",
                );
            }
            if let Some(nw) = self.forwarded(w, forward) {
                ptr::write_unaligned(slot, nw);
            }
        });
    }

    /// **AOT00-T3 PR-3b — evacuate + pointer fixup (the moving cycle's steps 1–3).**
    /// Marks/classifies, copies every movable object into a fresh to-space [`Arena`]
    /// (via [`Self::plan_compaction`]), then rewrites every pointer that named a moved
    /// object to its new arena address:
    /// - **roots** — each precise `root_slot` whose word names a moved object is updated
    ///   in place (tag preserved);
    /// - **interior** — each moved object's *arena copy* has its registered-kind
    ///   reference fields rewritten (pinned and `kind == 0` objects provably hold no
    ///   pointer to a moved object — see [`Self::forwarded`] — so they are skipped).
    ///
    /// Returns `(arena, forward)`. **The caller MUST keep the arena alive for as long as
    /// any rewritten pointer is dereferenced** — the moved objects now live *in the
    /// arena*, and the from-space originals are intentionally left untouched (not freed)
    /// and orphaned. Reclaiming the from-space blocks and re-threading the heap's
    /// all-list over the arena (so the heap itself uses the compacted copies) is the
    /// integration step PR-3c; this PR lands and reviews the fixup math in isolation. The
    /// remembered set is likewise left pointing at the still-valid from-space addresses;
    /// remapping it belongs with the reclamation step.
    ///
    /// # Safety
    /// Each `root_slots` address and each `regions` span must be readable (same contract
    /// as [`Self::collect_mixed`]).
    unsafe fn evacuate_and_fixup(
        &mut self,
        root_slots: &[usize],
        regions: &[(*const u8, usize)],
    ) -> (Arena, HashMap<usize, usize>) {
        let (arena, forward) = self.plan_compaction(root_slots, regions);

        // (a) Roots: write the forwarded address back into each precise slot.
        for &slot in root_slots {
            let w = ptr::read_unaligned(slot as *const usize);
            if let Some(nw) = self.forwarded(w, &forward) {
                ptr::write_unaligned(slot as *mut usize, nw);
            }
        }

        // (b) Interior: fix up each MOVED object's arena copy. (Only moved objects hold
        //     pointers to moved objects; pinned / kind==0 objects were conservatively
        //     scanned in classification, which pinned their targets.)
        for &new_payload in forward.values() {
            self.fixup_ref_fields((new_payload - HEADER_SIZE) as *mut FlatHeader, &forward);
        }

        (arena, forward)
    }

    /// **AOT00-T3 PR-3c-2 — the full moving cycle (`collect_compacting`).** Runs one
    /// complete relocating collection: classify + evacuate + fix up (steps 1–3, via
    /// [`Self::evacuate_and_fixup`]), then **reclaim** from-space and **integrate** the
    /// to-space arena into the heap (step 4). After it returns the heap is self-consistent
    /// and owns everything: the moved objects live in an arena on `self.arenas`, the
    /// pinned survivors stay in place on `self.all`, and the from-space originals and all
    /// unreachable objects are freed. This is the shipping entry the compacting collector
    /// exposes; the C ABI (spec §5) wraps it in a later rung.
    ///
    /// # Step 4 in detail — the UAF surface, and why each step is safe
    ///
    /// After [`Self::evacuate_and_fixup`], the crucial invariant established by
    /// [`Self::classify_mobility`] is:
    ///
    /// > **A reachable object survives *in place* iff its [`FlatHeader::pinned`] bit is
    /// > set; every reachable-but-not-moved object is pinned.**
    ///
    /// Proof: a reachable object is either (a) reached conservatively (a `regions` root, or
    /// through a `kind == 0` object) — the pinning wave sets its pin bit; or (b) reached
    /// only precisely. A precise `kind == 0` object is *seeded* into the pinning wave (its
    /// conservative out-edges make it unmovable) → pinned. A precise `kind != 0` object is
    /// pinned iff a conservative edge also reached it; if not, it is exactly a **movable**
    /// object and was moved. So `reachable ∧ ¬moved ≡ pinned`, and `movable ⇒ ¬pinned`.
    ///
    /// Therefore the pin bit is a ready-made *keep-in-place* predicate:
    /// 1. **Mark survivors-in-place**: set `marked = pinned` on every from-space block.
    ///    Unpinned = (unreachable) ∪ (moved originals, since `movable ⇒ ¬pinned`).
    /// 2. **Sweep** (`sweep(false)`) frees every unmarked (unpinned) block — reclaiming the
    ///    dead *and* the now-orphaned from-space originals of moved objects (their bytes
    ///    live in the arena; every pointer that named them was rewritten in step 3, so no
    ///    live reference dangles) — and keeps + ages the pinned survivors, re-threading
    ///    `self.all` over just them. From-space originals are malloc'd (`arena_backed ==
    ///    false`), so `sweep` frees them normally; no arena slice is touched here.
    /// 3. **Integrate the arena**: the moved objects' arena copies are not yet on any list
    ///    (their `next` fields are stale bytes from the `copy_nonoverlapping`). Re-thread
    ///    them into one fresh chain and prepend it to `self.all`, and age/tenure each just
    ///    as `sweep` ages an in-place survivor (so a moved young object still progresses
    ///    toward tenuring instead of being immortally young). Then hand the arena to
    ///    `self.arenas` so its storage outlives the collection — and is freed exactly once,
    ///    when the arena drops, never by an individual `dealloc` (its blocks are
    ///    [`FlatHeader::arena_backed`]).
    /// 4. **Rebuild the remembered set** over the *post-integration* `self.all`: this both
    ///    remaps any moved old→young parent to its new address and re-derives every
    ///    old→young edge (the promotion barrier), exactly as a full [`Self::collect_mixed`]
    ///    does — a moved parent is found at its new arena address, its rewritten ref fields
    ///    resolve to the survivors' current addresses.
    ///
    /// With **no movable survivors** (`forward` empty) this degenerates to marking the
    /// pinned/reachable set and sweeping — i.e. exactly [`Self::collect_mixed`], the
    /// spec's "strict generalization" (§4). (Because a *pinned* `kind != 0` object is
    /// scanned conservatively during classification, the kept set can be a conservative
    /// superset of `collect_mixed`'s — never smaller: a live object is never freed.)
    ///
    /// # Safety
    /// Each `root_slots` address and each `regions` span must be readable (same contract as
    /// [`Self::collect_mixed`]).
    pub unsafe fn collect_compacting(
        &mut self,
        root_slots: &[usize],
        regions: &[(*const u8, usize)],
    ) -> GcCycleStats {
        debug_assert!(!self.mark_in_progress, "{}", INCREMENTAL_MIXING_MSG);
        let before = self.object_count();
        let prev_live = self.live_bytes;

        // Steps 1–3: classify, evacuate movable survivors into a fresh arena, and rewrite
        // every pointer that named a moved object (roots + moved copies' ref fields).
        let (arena, forward) = self.evacuate_and_fixup(root_slots, regions);

        // The moved objects' new headers (arena copies) and their total live bytes.
        // Captured before the sweep so the from-space walk below is undisturbed.
        let mut moved_new: Vec<*mut FlatHeader> = Vec::with_capacity(forward.len());
        let mut moved_bytes = 0usize;
        for &new_payload in forward.values() {
            let nh = (new_payload - HEADER_SIZE) as *mut FlatHeader;
            moved_bytes += (*nh).size;
            moved_new.push(nh);
        }

        // Step 4.1: mark survivors-in-place. `pinned` is the keep predicate (see doc);
        // unpinned blocks — the unreachable *and* the moved-from-space originals — will be
        // freed by the sweep.
        {
            let mut h = self.all;
            while !h.is_null() {
                (*h).marked = (*h).pinned;
                h = (*h).next;
            }
        }

        // Step 4.2: sweep from-space. Frees unpinned blocks (dead + moved originals), keeps
        // and ages the pinned survivors, re-threads `self.all` over them, clears marks.
        let (swept, survived_in_place, live_in_place, _promoted) = self.sweep(false);
        // A moved object's from-space original is swept too, but it did not *die* — its
        // contents live on in the arena. Report only the genuinely-dead (unreachable) count,
        // so `freed + survived == before` holds exactly as for a non-moving collect. Every
        // moved object contributes exactly one swept original, so this subtraction is exact.
        let freed = swept.saturating_sub(moved_new.len());

        // Step 4.3: integrate the arena copies. Re-thread them (their `next` bytes are
        // stale) into one chain, age/tenure each like an in-place survivor, and prepend to
        // the (pinned-survivor) all-list.
        for &nh in &moved_new {
            if (*nh).generation == GEN_YOUNG {
                (*nh).age = (*nh).age.saturating_add(1);
                if (*nh).age >= self.tenure_age {
                    (*nh).generation = GEN_OLD;
                }
            }
            (*nh).marked = false; // a fresh survivor carries no mark into the next cycle
        }
        for i in 0..moved_new.len() {
            let nh = moved_new[i];
            (*nh).next = if i + 1 < moved_new.len() { moved_new[i + 1] } else { self.all };
        }
        if let Some(&head) = moved_new.first() {
            self.all = head;
        }

        // Retain the arena so the moved objects' storage outlives the collection. Freed
        // exactly once, when the arena drops (its blocks are `arena_backed`, so `sweep` /
        // `Drop` never `dealloc` them individually).
        self.arenas.push(arena);

        self.live_bytes = live_in_place + moved_bytes;
        self.adapt_threshold(prev_live);
        // Rebuild over the integrated all-list: remaps moved old→young parents to their new
        // addresses and re-derives the promotion barrier, as a full collect does.
        self.rebuild_remembered();

        let survived = survived_in_place + moved_new.len();
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

    /// Free unmarked blocks and age/tenure survivors; returns
    /// `(freed, survived, live_bytes, promoted)` where `promoted` is the headers of
    /// objects tenured to [`GEN_OLD`] *this* sweep (for the caller to check for
    /// old→young pointers and add to the remembered set — the promotion barrier).
    ///
    /// When `young_only` (a **minor** cycle), **old** objects are left entirely
    /// alone — never freed, mark bit untouched — because a minor GC does not scan
    /// or reclaim the old generation; only **young** blocks are freed (if
    /// unmarked) or aged (if marked). A full cycle (`!young_only`) considers every
    /// block. A young survivor's age is bumped and it is tenured once it reaches
    /// [`Self::tenure_age`].
    fn sweep(&mut self, young_only: bool) -> (usize, usize, usize, Vec<*mut FlatHeader>) {
        let mut freed = 0usize;
        let mut survived = 0usize;
        let mut live = 0usize;
        let mut promoted: Vec<*mut FlatHeader> = Vec::new();
        // `cursor` points at the link that currently references the block under
        // inspection (`&mut self.all` first, then `&mut (*prev).next`), so we can
        // unlink in place without a previous-node special case.
        let mut cursor: *mut *mut FlatHeader = &mut self.all;
        // SAFETY: every dereferenced pointer is a live block we own; `dealloc`
        // uses the exact `Layout` `alloc_zeroed` was given (same size + align).
        unsafe {
            while !(*cursor).is_null() {
                let h = *cursor;
                // Minor cycle: an old object is untouched — still live, mark bit
                // (which a minor cycle never sets) left as-is. Advance past it.
                if young_only && (*h).generation == GEN_OLD {
                    survived += 1;
                    live += (*h).size;
                    cursor = &mut (*h).next;
                    continue;
                }
                // Capture the successor *before* `sweep_free_or_keep`, whose `Freed` arm
                // deallocates `h` — reading `(*h).next` after that is a use-after-free.
                let next = (*h).next;
                match Self::sweep_free_or_keep(h, self.tenure_age, &mut promoted) {
                    SweepVisit::Kept(sz) => {
                        survived += 1;
                        live += sz;
                        cursor = &mut (*h).next;
                    }
                    SweepVisit::Freed => {
                        // Unlink the dead block; the successor moves under the same cursor.
                        *cursor = next;
                        freed += 1;
                    }
                }
            }
        }
        (freed, survived, live, promoted)
    }

    /// Decide the fate of one block during a **full** sweep and enact it: a **marked** (live)
    /// block has its mark cleared, is aged, and tenured to [`GEN_OLD`] once its
    /// [`FlatHeader::age`] reaches [`Self::tenure_age`] (recording it in `promoted` for the
    /// promotion barrier); an **unmarked** (dead) block is **freed** here — unless it is
    /// [`FlatHeader::arena_backed`] (a slice of an arena, freed en masse when the arena drops,
    /// never individually — that would be UB). Returns [`SweepVisit::Kept`] (with the live
    /// payload bytes) or [`SweepVisit::Freed`]; the caller advances/unlinks the cursor. Shared
    /// verbatim by the monolithic [`Self::sweep`] and the stepped
    /// [`Self::incremental_sweep_step`] so their free/age logic can never drift apart.
    ///
    /// # Safety
    /// `h` is a live block owned by this heap; on `Freed` it is deallocated, so the caller must
    /// not touch `h` afterward (it reads `(*h).next` *before* calling to relink).
    unsafe fn sweep_free_or_keep(
        h: *mut FlatHeader,
        tenure_age: u8,
        promoted: &mut Vec<*mut FlatHeader>,
    ) -> SweepVisit {
        if (*h).marked {
            (*h).marked = false;
            // Tenure the survivor, but only once it has *aged* enough. A young object that
            // lives through a collection has its `age` bumped (saturating); when `age` reaches
            // `tenure_age` it is promoted to the old generation so a future minor GC can skip
            // it. With the default threshold of 1 this is immediate tenuring (age 0 → 1 ≥ 1).
            // Already-old objects never age. Keeping soon-to-die objects young a little longer
            // means a cheap minor GC reclaims them instead of a full GC.
            if (*h).generation == GEN_YOUNG {
                (*h).age = (*h).age.saturating_add(1);
                if (*h).age >= tenure_age {
                    (*h).generation = GEN_OLD;
                    // Record newly-promoted objects so the caller can add any that hold an
                    // old→young pointer to the remembered set (the promotion barrier). A store
                    // made into this object *while it was young* fired no write barrier, so
                    // under aging that edge would otherwise be invisible to the next minor GC.
                    promoted.push(h);
                }
            }
            SweepVisit::Kept((*h).size)
        } else {
            // Provenance: an **arena-backed** block is a slice of a big arena allocation — it
            // must NOT be `dealloc`'d individually (UB); its storage is reclaimed when the
            // whole arena drops. Only a normal per-object `alloc`'d block is freed here.
            if !(*h).arena_backed {
                let layout = Layout::from_size_align_unchecked(HEADER_SIZE + (*h).size, ALIGN);
                dealloc(h as *mut u8, layout);
            }
            SweepVisit::Freed
        }
    }

    /// Return `true` iff the object `h` holds a pointer to a live **young** block —
    /// i.e. it is an old→young source the remembered set must record. Traces `h`'s
    /// payload with the *same* discipline the collector's mark uses (a registered
    /// [`FlatHeader::kind`]'s reference-field offsets precisely; otherwise every
    /// aligned word conservatively; both the raw word and its low-3-tag-stripped
    /// form), so an edge is recorded exactly when a minor GC's scan would follow it.
    ///
    /// # Safety
    /// `h` must be a live block owned by this heap.
    unsafe fn points_to_live_young(&self, h: *mut FlatHeader) -> bool {
        let has_young = |word: usize| -> bool {
            for cand in [word, word & !0b111usize] {
                let child = self.find_header(cand);
                if !child.is_null() && (*child).generation == GEN_YOUNG {
                    return true;
                }
            }
            false
        };
        // Precise path: check each reference slot (fixed field or array-tail element). The
        // closure keeps scanning after a hit (the remembered-set rebuild is cold), which is
        // result-identical to the old early `return true`.
        let mut found = false;
        let precise = self.for_each_ref_slot(h, |slot| {
            if !found && has_young(ptr::read_unaligned(slot)) {
                found = true;
            }
        });
        if precise {
            return found;
        }
        // Conservative fallback (kind 0 / unregistered): scan every aligned word.
        let base = h.add(1) as *const u8;
        let size = (*h).size;
        let mut off = 0usize;
        while size >= 8 && off <= size - 8 {
            let w = ptr::read_unaligned(base.add(off) as *const usize);
            if has_young(w) {
                return true;
            }
            off += 8;
        }
        false
    }

    /// **Rebuild** the remembered set after a *full* collect: it must list exactly
    /// the surviving **old** objects that point into the **young** generation. A
    /// full collect can free old objects (dangling entries) and — under aging —
    /// leave young objects alive (real old→young edges), so neither the pre-collect
    /// entries nor a blanket clear is correct. Recompute from scratch by scanning
    /// the (small, post-sweep) live set. When nothing young survives (e.g. the
    /// default immediate-tenuring threshold), this yields the empty set, exactly as
    /// the previous `clear()` did.
    fn rebuild_remembered(&mut self) {
        self.remembered.clear();
        let mut h = self.all;
        // SAFETY: list walk over live blocks we own; `points_to_live_young` only
        // reads payloads of live blocks.
        unsafe {
            while !h.is_null() {
                if (*h).generation == GEN_OLD && self.points_to_live_young(h) {
                    self.remembered.insert(h as usize + HEADER_SIZE);
                }
                h = (*h).next;
            }
        }
    }

    /// The **promotion barrier** for a *minor* collect: after a minor sweep, add any
    /// just-`promoted` object that now points into the young generation to the
    /// remembered set. A minor keeps the existing remembered entries (it never frees
    /// old objects, so they cannot dangle); this covers the new old→young edges that
    /// aging creates when a parent tenures a cycle before its still-young child.
    fn record_promoted_old_to_young(&mut self, promoted: &[*mut FlatHeader]) {
        // SAFETY: every `h` came from this sweep's survivor set and is live.
        unsafe {
            for &h in promoted {
                if self.points_to_live_young(h) {
                    self.remembered.insert(h as usize + HEADER_SIZE);
                }
            }
        }
    }
}

impl Drop for FlatHeap {
    /// Free every block still on the list — no leak when the heap itself is
    /// dropped (e.g. a test's `FlatHeap` going out of scope). **Arena-backed** blocks
    /// are skipped here: they are slices of the arenas in `self.arenas`, which are freed
    /// as those `Arena`s drop *after* this `Drop::drop` returns (Rust drops the fields
    /// after the explicit impl). So each block's storage is released exactly once — a
    /// malloc'd block here, an arena block via its `Arena`.
    fn drop(&mut self) {
        let mut h = self.all;
        // SAFETY: list walk freeing malloc'd blocks we own with their exact layouts;
        // arena-backed blocks are not `dealloc`'d (their arena frees them).
        unsafe {
            while !h.is_null() {
                let next = (*h).next;
                if !(*h).arena_backed {
                    let layout = Layout::from_size_align_unchecked(HEADER_SIZE + (*h).size, ALIGN);
                    dealloc(h as *mut u8, layout);
                }
                h = next;
            }
        }
        self.all = ptr::null_mut();
        // `self.arenas` drops next (after this method), freeing the arena-backed blocks.
    }
}

// ─── Stack maps: the format + lookup half of precise roots ──────────────────────
//
// A stack map answers one question (AOT00-T1-precise-gc.md §4): *at this program
// counter, where are the live references?* The compiler backend emits one
// [`StackMapRecord`] per safepoint (every `safepoint` op and, crucially, every
// call site — a callee may allocate and thus collect, so the caller's live refs
// must be described there). At collection time the native stack walker
// (`gc-core-capi`) matches the current return address to a record, computes the
// exact slot addresses with [`frame_root_slots`], and hands them to
// [`FlatHeap::collect_precise`].
//
// This is the platform-independent *data structure + lookup*; the walk that reads
// a live machine stack is the platform-specific half in `gc-core-capi`, kept out
// of here for the same reason the C-stack scan is (it needs `asm!` + the frame
// layout), so this half stays pure and unit-testable.

/// One safepoint's live-reference description (AOT00-T1-precise-gc.md §4.1).
///
/// Records are keyed within a function by `pc_offset` (the return-address /
/// safepoint offset from the function's start) so the walker can binary-search by
/// return address. The reference locations are `slots` — **byte offsets from the
/// frame pointer** — plus `callee_saved_mask`, a bitmask of callee-saved registers
/// that hold references at this PC (spilled and scanned by the walker). `frame_size`
/// lets the walker step to the caller's frame.
///
/// `frame_size` and `callee_saved_mask` are consumed by the stack *walker*
/// (`gc-core-capi`, a later PR) — the frame-stepping and register-spill halves;
/// [`frame_root_slots`] and [`FlatHeap::collect_precise`] here use only `slots`.
/// They are carried now because they are part of the record the backends emit and
/// the walker reads, so the format is fixed once.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StackMapRecord {
    /// Offset of this safepoint / return address from the function's first byte.
    pub pc_offset: u32,
    /// Frame size in bytes — how far up to the caller's frame (walker use).
    pub frame_size: u32,
    /// Live reference slots at this PC, as **byte offsets from the frame pointer**
    /// (may be negative for slots below FP).
    pub slots: Vec<i32>,
    /// Bitmask of callee-saved registers holding references here (walker use).
    pub callee_saved_mask: u16,
}

impl StackMapRecord {
    /// A record naming `slots` (FP-relative byte offsets) at `pc_offset`, with no
    /// callee-saved reference registers and an unspecified frame size. The common
    /// shape a first-cut backend emits (all live refs spilled to the stack frame).
    pub fn new(pc_offset: u32, slots: Vec<i32>) -> Self {
        Self {
            pc_offset,
            frame_size: 0,
            slots,
            callee_saved_mask: 0,
        }
    }
}

/// A function's stack maps: its per-safepoint [`StackMapRecord`]s, sorted by
/// `pc_offset` so a return address resolves in O(log n) (§4.2).
///
/// The backend builds one table per compiled function; the walker looks up the
/// record for the current PC by binary search. Construction sorts, so records may
/// be supplied in any order.
#[derive(Debug, Clone, Default)]
pub struct StackMapTable {
    records: Vec<StackMapRecord>,
}

impl StackMapTable {
    /// Build a table from `records`, sorting them by `pc_offset` so [`lookup`] can
    /// binary-search. Any input order is accepted.
    ///
    /// Each `pc_offset` should be **unique** within a function — a backend emits at
    /// most one record per safepoint / return address. Duplicate `pc_offset`s are
    /// not rejected (this is trusted compiler output, like a `kind` field map), but
    /// [`lookup`] would then return an arbitrary one of the collisions; keep them
    /// distinct.
    ///
    /// [`lookup`]: Self::lookup
    pub fn from_records(mut records: Vec<StackMapRecord>) -> Self {
        records.sort_by_key(|r| r.pc_offset);
        Self { records }
    }

    /// The record whose `pc_offset` **exactly** equals `pc_offset`, or `None`.
    ///
    /// A backend emits a record at precisely each safepoint / call-return address,
    /// and the walker looks up the exact return address it unwound, so the match is
    /// exact — not a range/`<=` search. A PC with no record is an unmapped frame:
    /// the caller falls back to a conservative scan of that frame (§4.2), so a miss
    /// is safe, never a crash.
    pub fn lookup(&self, pc_offset: u32) -> Option<&StackMapRecord> {
        self.records
            .binary_search_by_key(&pc_offset, |r| r.pc_offset)
            .ok()
            .map(|i| &self.records[i])
    }

    /// Number of safepoint records in this table.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether this table has no records.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// The records, in `pc_offset` order (for inspection / re-emission).
    pub fn records(&self) -> &[StackMapRecord] {
        &self.records
    }
}

/// Turn a walked frame into the exact root-slot addresses its stack map names,
/// appending them to `out` for [`FlatHeap::collect_precise`].
///
/// `frame_base` is the frame pointer of a live frame the walker has unwound to;
/// `rec` is the [`StackMapRecord`] matched to that frame's return address. Each
/// named slot's address is `frame_base + slot_offset` (the offset is signed:
/// negative slots live below FP). This is the pure arithmetic bridge between the
/// stack-map format and the precise-mark core — the walker calls it per mapped
/// frame, accumulating one flat list of addresses, then makes a single
/// `collect_precise` call.
///
/// It computes addresses only; it does **not** dereference them. Whether a given
/// `frame_base + offset` is actually readable is the walker's responsibility (it
/// derived `frame_base` from a real frame), enforced at the `collect_precise`
/// safety boundary.
pub fn frame_root_slots(frame_base: usize, rec: &StackMapRecord, out: &mut Vec<usize>) {
    out.reserve(rec.slots.len());
    for &off in &rec.slots {
        // Signed-offset add via wrapping in the isize domain: a slot below FP has a
        // negative offset. Matches how a walker computes `[fp + off]`.
        let addr = (frame_base as isize).wrapping_add(off as isize) as usize;
        out.push(addr);
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
        assert_eq!(
            stats.freed, 1,
            "the object with no candidate in the region is freed"
        );
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
        let stats = unsafe {
            heap.collect_region(region.as_ptr() as *const u8, std::mem::size_of_val(&region))
        };
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
        let stats = unsafe {
            heap.collect_region(region.as_ptr() as *const u8, std::mem::size_of_val(&region))
        };
        assert_eq!(stats.survived, 3);
        assert_eq!(stats.freed, 1);
    }

    // ── Adaptive collection threshold (GC pacing) ──────────────────────────────

    /// A fresh heap starts at the initial threshold and is not yet due to collect.
    #[test]
    fn fresh_heap_starts_at_initial_threshold() {
        let heap = FlatHeap::new();
        assert_eq!(heap.collect_threshold(), INITIAL_THRESHOLD);
        assert!(!heap.should_collect());
    }

    /// `should_collect` flips true exactly when live bytes reach the threshold.
    #[test]
    fn should_collect_fires_at_threshold() {
        let mut heap = FlatHeap::new();
        heap.collect_threshold = 100;
        heap.live_bytes = 99;
        assert!(!heap.should_collect());
        heap.live_bytes = 100;
        assert!(heap.should_collect());
        heap.live_bytes = 101;
        assert!(heap.should_collect());
    }

    /// Retention-heavy cycle (> half survived) doubles the threshold.
    #[test]
    fn adapt_threshold_doubles_when_retention_high() {
        let mut heap = FlatHeap::new();
        heap.collect_threshold = 4 * INITIAL_THRESHOLD;
        heap.live_bytes = 100; // survived
        heap.adapt_threshold(150); // prev/2 = 75 < 100 → grow
        assert_eq!(heap.collect_threshold(), 8 * INITIAL_THRESHOLD);
    }

    /// Garbage-heavy cycle (≤ half survived) halves the threshold.
    #[test]
    fn adapt_threshold_halves_when_retention_low() {
        let mut heap = FlatHeap::new();
        heap.collect_threshold = 4 * INITIAL_THRESHOLD;
        heap.live_bytes = 10; // survived
        heap.adapt_threshold(100); // prev/2 = 50 ≥ 10 → shrink
        assert_eq!(heap.collect_threshold(), 2 * INITIAL_THRESHOLD);
    }

    /// Growth is capped at `MAX_THRESHOLD` — the safety cap that keeps the GC from
    /// being tuned off entirely.
    #[test]
    fn adapt_threshold_caps_at_max() {
        let mut heap = FlatHeap::new();
        heap.collect_threshold = MAX_THRESHOLD;
        heap.live_bytes = MAX_THRESHOLD; // fully retained → wants to grow
        heap.adapt_threshold(1);
        assert_eq!(heap.collect_threshold(), MAX_THRESHOLD);
    }

    /// Shrinking never drops below the initial floor.
    #[test]
    fn adapt_threshold_floors_at_initial() {
        let mut heap = FlatHeap::new();
        heap.collect_threshold = INITIAL_THRESHOLD;
        heap.live_bytes = 0; // nothing survived → wants to shrink
        heap.adapt_threshold(1000);
        assert_eq!(heap.collect_threshold(), INITIAL_THRESHOLD);
    }

    /// A real collect that reclaims everything (0 survivors) shrinks the threshold
    /// — end-to-end proof the cycle re-tunes pacing, not just the direct unit test.
    #[test]
    fn collect_retunes_threshold_end_to_end() {
        let mut heap = FlatHeap::new();
        heap.collect_threshold = 4 * INITIAL_THRESHOLD;
        heap.alloc(64, 0); // unrooted → freed by the collect below
        let _ = heap.collect(&[]); // 0 survive ≤ prev/2 → halve
        assert_eq!(heap.collect_threshold(), 2 * INITIAL_THRESHOLD);
    }

    // ── Precise interior tracing (HeapKind field maps) ─────────────────────────

    /// Kind ids are assigned from 1 (0 stays reserved for conservative tracing).
    #[test]
    fn register_kind_ids_are_one_based() {
        let mut heap = FlatHeap::new();
        assert_eq!(heap.registered_kinds(), 0);
        assert_eq!(heap.register_kind(&[0]), 1);
        assert_eq!(heap.register_kind(&[0, 8]), 2);
        assert_eq!(heap.registered_kinds(), 2);
    }

    /// The headline property: with a precise kind whose only ref field is at
    /// offset 0, a heap pointer stored in a **non-reference** field (offset 8) is
    /// NOT followed, so the pointee is reclaimed — no phantom retention.
    #[test]
    fn precise_tracing_reclaims_phantom_in_nonref_field() {
        let mut heap = FlatHeap::new();
        // Kind 1: a 16-byte object whose only ref field is at offset 0.
        let rec = heap.register_kind(&[0]);
        let target = heap.alloc(16, 0) as usize; // opaque pointee
        let container = heap.alloc(16, rec) as usize;
        unsafe {
            *(container as *mut usize) = 0; // field@0 (ref) = null
            *((container + 8) as *mut usize) = target; // field@8 (non-ref) = look-alike ptr
        }
        // Root only the container.
        let stats = heap.collect(&[container]);
        // Precise tracing follows offset 0 only → `target` is unreachable → freed.
        assert!(
            heap.find_header(target).is_null(),
            "precise tracing must reclaim a pointee referenced only via a non-ref field"
        );
        assert!(
            !heap.find_header(container).is_null(),
            "container is rooted"
        );
        assert_eq!(stats.freed, 1, "exactly the phantom pointee is freed");
    }

    /// Baseline contrast: the *same* memory layout under conservative tracing
    /// (kind 0) DOES retain the phantom — proving the precise path is what closes
    /// the gap, not something else.
    #[test]
    fn conservative_retains_phantom_baseline() {
        let mut heap = FlatHeap::new();
        let target = heap.alloc(16, 0) as usize;
        let container = heap.alloc(16, 0) as usize; // kind 0 → conservative
        unsafe {
            *(container as *mut usize) = 0;
            *((container + 8) as *mut usize) = target; // phantom in payload
        }
        let stats = heap.collect(&[container]);
        assert!(
            !heap.find_header(target).is_null(),
            "conservative tracing retains the look-alike pointer's pointee"
        );
        assert_eq!(
            stats.freed, 0,
            "nothing freed — both survive conservatively"
        );
    }

    /// A pointer in a real **ref** field is still followed under precise tracing.
    #[test]
    fn precise_tracing_follows_real_ref_field() {
        let mut heap = FlatHeap::new();
        let rec = heap.register_kind(&[0]); // ref field at offset 0
        let target = heap.alloc(16, 0) as usize;
        let container = heap.alloc(16, rec) as usize;
        unsafe {
            *(container as *mut usize) = target; // field@0 (ref) = target
            *((container + 8) as *mut usize) = 0;
        }
        let stats = heap.collect(&[container]);
        assert!(
            !heap.find_header(target).is_null(),
            "a real ref field must keep its pointee alive"
        );
        assert_eq!(stats.freed, 0);
    }

    /// An object carrying a kind id with no registered map falls back to a
    /// conservative full-payload scan — a safe default that never under-traces.
    #[test]
    fn unregistered_kind_falls_back_to_conservative() {
        let mut heap = FlatHeap::new();
        let target = heap.alloc(16, 0) as usize;
        // kind 99 was never registered → treated conservatively.
        let container = heap.alloc(16, 99) as usize;
        unsafe {
            *((container + 8) as *mut usize) = target;
        }
        let _ = heap.collect(&[container]);
        assert!(
            !heap.find_header(target).is_null(),
            "unregistered kind traces conservatively (safe fallback)"
        );
    }

    /// A field offset that would run past the payload is skipped, never read out
    /// of bounds — a malformed map cannot corrupt memory.
    #[test]
    fn precise_out_of_range_offset_is_skipped() {
        let mut heap = FlatHeap::new();
        // Ref field claimed at offset 16, but the object is only 16 bytes: the
        // 8-byte read at 16 would end at 24 > 16, so it must be skipped.
        let rec = heap.register_kind(&[16]);
        let target = heap.alloc(16, 0) as usize;
        let container = heap.alloc(16, rec) as usize;
        unsafe {
            *(container as *mut usize) = target; // in-bounds but NOT a mapped field
        }
        // No mapped offset is in range → nothing traced from container → target freed.
        let stats = heap.collect(&[container]);
        assert!(heap.find_header(target).is_null());
        assert_eq!(stats.freed, 1);
    }

    /// A pathological offset near `usize::MAX` must not wrap the bounds check and
    /// trigger an out-of-bounds read — it is simply skipped. (The guard is written
    /// `off <= size - 8`, not `off + 8 <= size`, precisely to avoid the wrap.)
    #[test]
    fn precise_huge_offset_does_not_overflow_the_guard() {
        let mut heap = FlatHeap::new();
        let rec = heap.register_kind(&[usize::MAX, usize::MAX - 3]);
        let container = heap.alloc(16, rec) as usize;
        unsafe {
            *(container as *mut usize) = 0;
        }
        // Must not read out of bounds; container is rooted so it survives cleanly.
        let _ = heap.collect(&[container]);
        assert!(!heap.find_header(container).is_null());
    }

    // ── Variable-length reference arrays (AOT00-T5 PR-2) ──────────────────────
    //
    // A `register_ref_array_kind` kind traces a tail region `[tail_from, size)` as references,
    // so ONE kind describes arrays of every length. These prove the tail is traced precisely
    // (survivors = exactly the referenced elements), is movable under compaction (vs a pinned
    // conservative twin), composes with a fixed header, feeds the generational barrier, and is
    // bounds-safe at the edges.

    /// **Precise array trace.** A length-3 reference array holds A, B, C plus a look-alike
    /// integer in a fourth slot. Precise tracing follows the three real references and the
    /// phantom integer's pointee is reclaimed — the whole point of a precise array.
    #[test]
    fn ref_array_traces_elements_precisely() {
        let mut heap = FlatHeap::new();
        let arr_kind = heap.register_ref_array_kind(&[], 0); // every word is a reference
        let a = heap.alloc(16, 0) as usize;
        let b = heap.alloc(16, 0) as usize;
        let c = heap.alloc(16, 0) as usize;
        let phantom = heap.alloc(16, 0) as usize;
        // A 4-slot array: [A, B, C, <look-alike int to phantom>].
        let arr = heap.alloc(32, arr_kind) as usize;
        unsafe {
            *(arr as *mut usize) = a;
            *((arr + 8) as *mut usize) = b;
            *((arr + 16) as *mut usize) = c;
            *((arr + 24) as *mut usize) = phantom; // stored as an element → IS a reference here
        }
        // Root only the array. With every slot a declared reference, all four pointees are
        // reachable — so nothing is freed. (Contrast: `phantom` is only reachable *through* the
        // array, proving the tail is scanned.)
        let stats = heap.collect(&[arr]);
        assert_eq!(stats.freed, 0, "all four elements reachable via the array's tail");
        for p in [a, b, c, phantom] {
            assert!(!heap.find_header(p).is_null(), "element retained via the ref tail");
        }
        // Now drop one element (null slot 1) and collect again: B becomes unreachable.
        unsafe { *((arr + 8) as *mut usize) = 0 };
        let stats2 = heap.collect(&[arr]);
        assert_eq!(stats2.freed, 1, "the dropped element is now reclaimed");
        assert!(heap.find_header(b).is_null(), "B freed after its only ref (the slot) was cleared");
    }

    /// **A ref array is movable; its conservative twin is pinned.** Under `collect_compacting` a
    /// precise array and every element it references relocate into the arena; the identical heap
    /// built with `kind 0` (conservative) pins them in place. This is the array-shaped analogue
    /// of the cons-cell relocation proof, and the whole reason precise arrays matter.
    #[test]
    fn ref_array_relocates_under_compaction_vs_pinned_conservative_twin() {
        // Precise: a 2-element array of two leaf objects.
        let mut heap = FlatHeap::new();
        let arr_kind = heap.register_ref_array_kind(&[], 0);
        let leaf = heap.register_kind(&[]); // a movable leaf (no ref fields)
        let e0 = heap.alloc(16, leaf) as usize;
        let e1 = heap.alloc(16, leaf) as usize;
        let arr = heap.alloc(16, arr_kind) as usize;
        unsafe {
            *(arr as *mut usize) = e0;
            *((arr + 8) as *mut usize) = e1;
            *((e1 + 8) as *mut usize) = 0xCAFE_usize; // sentinel in a non-ref word of e1
        }
        let root = arr;
        let slots = [&root as *const usize as usize];
        let stats = unsafe { heap.collect_compacting(&slots, &[]) };
        assert_eq!(stats.survived, 3, "the array + both elements survive");
        assert_eq!(stats.freed, 0);
        let narr = root; // root slot rewritten to the array's new address
        assert_ne!(narr, arr, "the array itself relocated");
        let ne0 = unsafe { *(narr as *const usize) };
        let ne1 = unsafe { *((narr + 8) as *const usize) };
        assert_ne!(ne0, e0, "element 0 relocated; the array's tail slot was fixed up");
        assert_ne!(ne1, e1, "element 1 relocated");
        assert_eq!(
            unsafe { *((ne1 + 8) as *const usize) },
            0xCAFE,
            "moved element's payload byte-preserved (a missed tail fixup would dangle)",
        );

        // Conservative twin: same shape, kind 0 everywhere → pinned, nothing moves.
        let mut twin = FlatHeap::new();
        let t0 = twin.alloc(16, 0) as usize;
        let t1 = twin.alloc(16, 0) as usize;
        let tarr = twin.alloc(16, 0) as usize;
        unsafe {
            *(tarr as *mut usize) = t0;
            *((tarr + 8) as *mut usize) = t1;
        }
        let troot = tarr;
        let tslots = [&troot as *const usize as usize];
        let tstats = unsafe { twin.collect_compacting(&tslots, &[]) };
        assert_eq!(tstats.survived, 3, "twin survivors match");
        assert_eq!(troot, tarr, "conservative array is PINNED — no relocation");
        drop(heap);
        drop(twin);
    }

    /// **Header + tail compose.** A `{header_ref, len, elems…}` object: a fixed reference field
    /// at offset 0, a non-reference length word at offset 8, then a reference tail from offset
    /// 16. The header and the elements are traced; the `len` word between them is not treated as
    /// a pointer.
    #[test]
    fn ref_array_fixed_header_and_tail_compose() {
        let mut heap = FlatHeap::new();
        // fixed ref at 0; tail from 16 (skips the len word at 8).
        let kind = heap.register_ref_array_kind(&[0], 16);
        let hdr = heap.alloc(16, 0) as usize;
        let e0 = heap.alloc(16, 0) as usize;
        let phantom = heap.alloc(16, 0) as usize;
        let obj = heap.alloc(24, kind) as usize;
        unsafe {
            *(obj as *mut usize) = hdr; // fixed ref field
            *((obj + 8) as *mut usize) = phantom; // the `len` word — a look-alike int, NOT a ref
            *((obj + 16) as *mut usize) = e0; // tail element (ref)
        }
        let stats = heap.collect(&[obj]);
        assert!(!heap.find_header(hdr).is_null(), "fixed header ref retained");
        assert!(!heap.find_header(e0).is_null(), "tail element retained");
        assert!(heap.find_header(phantom).is_null(), "the non-ref len word did not pin its look-alike");
        assert_eq!(stats.freed, 1, "exactly the phantom is reclaimed");
    }

    /// **The tail feeds the generational barrier.** An old array whose tail holds a young element
    /// is recorded as an old→young source, so a minor GC keeps the young element.
    #[test]
    fn ref_array_tail_records_old_to_young_edge() {
        let mut heap = FlatHeap::new();
        let arr_kind = heap.register_ref_array_kind(&[], 0);
        let arr = heap.alloc(16, arr_kind) as usize;
        // Age the array to old: survive a full collection rooted at itself.
        let _ = heap.collect(&[arr]);
        assert_eq!(heap.object_count_by_generation(), (0, 1), "array tenured to old");
        // Now store a fresh YOUNG element into the array's tail and fire the write barrier.
        let young = heap.alloc(16, 0) as usize;
        unsafe {
            *(arr as *mut usize) = young;
            heap.write_barrier(arr, young);
        }
        assert_eq!(heap.remembered_len(), 1, "old array with a young tail element is remembered");
        // A minor GC rooted only at the array keeps the young element (found via the tail).
        let stats = heap.collect_minor(&[arr]);
        assert_eq!(stats.freed, 0, "the young tail element survives the minor GC");
        assert!(!heap.find_header(young).is_null());
    }

    /// **Bounds edges are safe.** An empty array (`tail_from == size`) traces nothing; a
    /// `tail_from` past the payload is an empty tail; an unaligned `tail_from` is rounded up so
    /// the scan stays 8-aligned. None read out of bounds.
    #[test]
    fn ref_array_bound_edges_are_safe() {
        let mut heap = FlatHeap::new();
        // Empty array: 16-byte object, tail starts at 16 → no elements.
        let empty_kind = heap.register_ref_array_kind(&[], 16);
        let target = heap.alloc(16, 0) as usize;
        let empty = heap.alloc(16, empty_kind) as usize;
        unsafe { *(empty as *mut usize) = target }; // a word BEFORE the tail → not a reference
        let stats = heap.collect(&[empty]);
        assert!(heap.find_header(target).is_null(), "word before an empty tail is not a ref");
        assert!(!heap.find_header(empty).is_null());
        assert_eq!(stats.freed, 1);

        // Unaligned tail_from (7) is rounded up to 8; tail_from past size is empty. Neither reads
        // out of bounds (the collection completes and the rooted object survives).
        let mut heap2 = FlatHeap::new();
        let unaligned = heap2.register_ref_array_kind(&[], 7); // → rounds to 8
        let huge = heap2.register_ref_array_kind(&[], 4096); // past any small payload → empty
        let o1 = heap2.alloc(16, unaligned) as usize;
        let o2 = heap2.alloc(16, huge) as usize;
        unsafe {
            *(o1 as *mut usize) = 0;
            *((o1 + 8) as *mut usize) = 0;
        }
        let _ = heap2.collect(&[o1, o2]);
        assert!(!heap2.find_header(o1).is_null());
        assert!(!heap2.find_header(o2).is_null());
    }

    // ── Object-model stress differential (AOT00-T5) ───────────────────────────
    //
    // One heap graph exercising EVERY object-model feature at once — records (fixed ref
    // fields), reference arrays (a tail region), a header+tail object (fixed ref + non-ref
    // length word + ref tail), a cycle, opaque leaves, and look-alike-integer non-ref fields
    // — driven through both the non-moving collector and the compacting collector and checked
    // against a hand-computed oracle. This is the "does the whole thing hold together" test a
    // real language runtime (JS/Ruby/Python) needs: mixed layouts, cyclic references, and
    // integers that merely *look* like pointers, all in the same collection.

    /// Sentinels stored in non-reference words, checked byte-for-byte after relocation.
    const SENT_A: usize = 0xA0A0_A0A0;
    const SENT_D: usize = 0xD0D0_D0D0;
    const SENT_CYCLE: usize = 0xC1C1_C1C1;

    /// The graph's key object addresses (valid until a collection moves them).
    struct Graph {
        root: usize,
        arr: usize,
        rec2: usize,
        cycle: usize,
        leaf_a: usize,
        leaf_d: usize,
        phantom: usize,
    }

    /// Build the stress graph on `heap` and return the pre-collection addresses. Reachable from
    /// `root` (8 objects): root → {arr, rec2}; arr → [leaf_a, leaf_b, cycle]; rec2 → leaf_c
    /// (fixed), <look-alike int → phantom> (non-ref len word), leaf_d (tail); cycle → root
    /// (back-edge). Garbage (3 objects): `phantom` (named only by a non-ref look-alike int),
    /// an unreachable record, and an unreachable array.
    unsafe fn build_stress_graph(heap: &mut FlatHeap) -> Graph {
        let rec = heap.register_kind(&[0, 8]); // record: refs at 0 and 8
        let arr_kind = heap.register_ref_array_kind(&[], 0); // pure ref array
        let hdrarr = heap.register_ref_array_kind(&[0], 16); // fixed ref @0, len @8, tail @16
        let cyc = heap.register_kind(&[0]); // record with one ref
        let leaf = heap.register_kind(&[]); // opaque, no ref fields (movable)

        let leaf_a = heap.alloc(16, leaf) as usize;
        let leaf_b = heap.alloc(16, leaf) as usize;
        let leaf_c = heap.alloc(16, leaf) as usize;
        let leaf_d = heap.alloc(16, leaf) as usize;
        let phantom = heap.alloc(16, leaf) as usize; // garbage: only a look-alike int names it
        let cycle = heap.alloc(16, cyc) as usize;
        let arr = heap.alloc(24, arr_kind) as usize; // 3-slot ref array
        let rec2 = heap.alloc(24, hdrarr) as usize; // header + 1-slot tail
        let root = heap.alloc(16, rec) as usize;
        let _garbage_rec = heap.alloc(16, rec) as usize; // unreachable
        let _garbage_arr = heap.alloc(24, arr_kind) as usize; // unreachable

        // Wire the graph.
        *(root as *mut usize) = arr; // root.0 -> arr
        *((root + 8) as *mut usize) = rec2; // root.8 -> rec2
        *(arr as *mut usize) = leaf_a; // arr[0]
        *((arr + 8) as *mut usize) = leaf_b; // arr[1]
        *((arr + 16) as *mut usize) = cycle; // arr[2]
        *(rec2 as *mut usize) = leaf_c; // rec2.0 (fixed ref)
        *((rec2 + 8) as *mut usize) = phantom; // rec2.8 (NON-ref len word: a look-alike int)
        *((rec2 + 16) as *mut usize) = leaf_d; // rec2.16 (tail element)
        *(cycle as *mut usize) = root; // cycle.0 -> root (back-edge → a cycle)
        *((cycle + 8) as *mut usize) = SENT_CYCLE; // sentinel in cycle's non-ref word
        *((leaf_a + 8) as *mut usize) = SENT_A; // sentinel in a leaf non-ref word
        *((leaf_d + 8) as *mut usize) = SENT_D;

        Graph { root, arr, rec2, cycle, leaf_a, leaf_d, phantom }
    }

    /// **Non-moving collection over the mixed graph.** Rooting only `root`, exactly the 8
    /// reachable objects survive and the 3 garbage objects (including the look-alike-int
    /// `phantom`) are reclaimed — proving precise tracing across records, arrays, a header+tail,
    /// and a cycle in one pass, with a non-ref integer field pinning nothing.
    #[test]
    fn stress_graph_mark_sweep_matches_oracle() {
        let mut heap = FlatHeap::new();
        let g = unsafe { build_stress_graph(&mut heap) };
        assert_eq!(heap.object_count(), 11, "8 reachable + 3 garbage allocated");

        let stats = heap.collect(&[g.root]);

        assert_eq!(stats.freed, 3, "phantom + unreachable record + unreachable array reclaimed");
        assert_eq!(heap.object_count(), 8, "exactly the reachable set survives");
        // The phantom (named only by a non-ref look-alike integer) must be gone.
        assert!(heap.find_header(g.phantom).is_null(), "look-alike int did not pin the phantom");
        // Every reachable object is still present (the cycle did not cause over- or under-marking).
        for p in [g.root, g.arr, g.rec2, g.cycle, g.leaf_a, g.leaf_d] {
            assert!(!heap.find_header(p).is_null(), "reachable object retained");
        }
        // Sentinels intact (non-moving: addresses unchanged).
        assert_eq!(unsafe { *((g.leaf_a + 8) as *const usize) }, SENT_A);
        assert_eq!(unsafe { *((g.cycle + 8) as *const usize) }, SENT_CYCLE);
    }

    /// **Compacting collection over the mixed graph — the relocation stress test.** Every object
    /// is a registered kind reached only precisely, so all 8 survivors are movable: the whole
    /// graph evacuates into the arena, every edge (record field, array-tail slot, header+tail
    /// element, and the back-edge that closes the cycle) is fixed up to the new addresses, and
    /// the non-ref sentinels are byte-preserved. Walking the graph from the rewritten root after
    /// the move must reach every object at its *new* location — a missed fixup anywhere would
    /// dereference a freed from-space block (caught here and under Miri).
    #[test]
    fn stress_graph_compaction_relocates_whole_graph() {
        let mut heap = FlatHeap::new();
        let g = unsafe { build_stress_graph(&mut heap) };

        // Root through a slot so compaction can rewrite it in place.
        let root_holder = g.root;
        let slots = [&root_holder as *const usize as usize];
        let stats = unsafe { heap.collect_compacting(&slots, &[]) };

        assert_eq!(stats.freed, 3, "the 3 garbage objects reclaimed");
        assert_eq!(stats.survived, 8, "the 8 reachable objects survive (moved)");
        assert_eq!(heap.object_count(), 8);

        // Walk the graph from the REWRITTEN root; every hop must land on a moved object.
        let new_root = root_holder;
        assert_ne!(new_root, g.root, "root relocated");
        unsafe {
            let new_arr = *(new_root as *const usize);
            let new_rec2 = *((new_root + 8) as *const usize);
            assert_ne!(new_arr, g.arr, "arr relocated + root's ref fixed up");
            assert_ne!(new_rec2, g.rec2, "rec2 relocated + root's ref fixed up");

            // arr's tail: [leaf_a, leaf_b, cycle] all fixed up to new addresses.
            let new_leaf_a = *(new_arr as *const usize);
            let new_cycle = *((new_arr + 16) as *const usize);
            assert_ne!(new_leaf_a, g.leaf_a, "array element relocated + tail slot fixed up");
            assert_eq!(*((new_leaf_a + 8) as *const usize), SENT_A, "leaf sentinel preserved");

            // rec2: fixed ref (leaf_c) + tail element (leaf_d); the len word is a non-ref int.
            let new_leaf_d = *((new_rec2 + 16) as *const usize);
            assert_ne!(new_leaf_d, g.leaf_d, "header+tail element relocated + fixed up");
            assert_eq!(*((new_leaf_d + 8) as *const usize), SENT_D, "tail-element sentinel preserved");

            // The cycle: cycle.0 must point back at the NEW root address (back-edge fixed up).
            assert_ne!(new_cycle, g.cycle, "cycle node relocated");
            assert_eq!(*(new_cycle as *const usize), new_root, "cycle back-edge fixed up to new root");
            assert_eq!(*((new_cycle + 8) as *const usize), SENT_CYCLE, "cycle sentinel preserved");
        }

        // The phantom stays reclaimed — a non-ref look-alike int neither retained nor relocated it.
        assert!(heap.find_header(g.phantom).is_null());
        drop(heap); // frees the retained arena + any malloc survivors exactly once
    }

    // ── Generational split: young/old + promotion ─────────────────────────────

    /// Fresh allocations are born into the young generation.
    #[test]
    fn fresh_allocations_are_young() {
        let mut heap = FlatHeap::new();
        heap.alloc(16, 0);
        heap.alloc(16, 0);
        assert_eq!(heap.object_count_by_generation(), (2, 0));
    }

    /// An object that survives a collection is promoted (tenured) to the old
    /// generation; garbage is freed; a subsequently-allocated object is young
    /// again. This is the young → old lifecycle a minor GC will exploit.
    #[test]
    fn survivors_are_promoted_to_old() {
        let mut heap = FlatHeap::new();
        let keep = heap.alloc(16, 0) as usize;
        heap.alloc(16, 0); // unrooted garbage
        assert_eq!(heap.object_count_by_generation(), (2, 0));

        // Collect rooting only `keep`: the garbage is freed, `keep` is promoted.
        let stats = heap.collect(&[keep]);
        assert_eq!(stats.freed, 1);
        assert_eq!(
            heap.object_count_by_generation(),
            (0, 1),
            "the lone survivor is now old"
        );

        // A new allocation is young; the old survivor stays old.
        heap.alloc(16, 0);
        assert_eq!(heap.object_count_by_generation(), (1, 1));
    }

    /// A promoted object stays old across further collections (it is not demoted).
    #[test]
    fn old_objects_stay_old() {
        let mut heap = FlatHeap::new();
        let keep = heap.alloc(16, 0) as usize;
        let _ = heap.collect(&[keep]); // promote → old
        assert_eq!(heap.object_count_by_generation(), (0, 1));
        // Second collect, still rooting it: remains a single old object.
        let _ = heap.collect(&[keep]);
        assert_eq!(heap.object_count_by_generation(), (0, 1));
    }

    // ── Generational aging: tenure only after surviving `tenure_age` cycles ─────

    /// The default tenuring threshold is `1` — immediate tenuring, preserving the
    /// behaviour the generational rung shipped with (so this change is additive).
    #[test]
    fn default_tenure_age_is_one() {
        let heap = FlatHeap::new();
        assert_eq!(DEFAULT_TENURE_AGE, 1);
        assert_eq!(heap.tenure_age(), 1);
    }

    /// With a raised threshold a young survivor is **aged**, not tenured: it stays
    /// young for `tenure_age − 1` surviving collections and is promoted to old only
    /// on the `tenure_age`-th. This is the headline aging behaviour — it keeps
    /// objects that die in their 2nd/3rd cycle reclaimable by a cheap minor GC
    /// instead of polluting the old generation.
    #[test]
    fn raised_threshold_ages_before_tenuring() {
        let mut heap = FlatHeap::new();
        heap.set_tenure_age(3);
        assert_eq!(heap.tenure_age(), 3);

        let keep = heap.alloc(16, 0) as usize;
        // Survives collection 1 → age 1 (< 3): still young.
        let _ = heap.collect(&[keep]);
        assert_eq!(heap.object_count_by_generation(), (1, 0), "young after 1 survival");
        // Survives collection 2 → age 2 (< 3): still young.
        let _ = heap.collect(&[keep]);
        assert_eq!(heap.object_count_by_generation(), (1, 0), "young after 2 survivals");
        // Survives collection 3 → age 3 (≥ 3): tenured to old.
        let _ = heap.collect(&[keep]);
        assert_eq!(heap.object_count_by_generation(), (0, 1), "tenured on the 3rd survival");
        // Stays old thereafter (old objects never age or demote).
        let _ = heap.collect(&[keep]);
        assert_eq!(heap.object_count_by_generation(), (0, 1), "remains old");
    }

    /// Aging is driven by the shared sweep, so a **minor** GC ages a young survivor
    /// too: with threshold 2 the object stays young through one minor and tenures
    /// on the second.
    #[test]
    fn minor_gc_ages_young_survivor() {
        let mut heap = FlatHeap::new();
        heap.set_tenure_age(2);
        let keep = heap.alloc(16, 0) as usize;

        heap.collect_minor(&[keep]); // age 1 (< 2): young
        assert_eq!(heap.object_count_by_generation(), (1, 0), "young after 1 minor");
        heap.collect_minor(&[keep]); // age 2 (≥ 2): tenured
        assert_eq!(heap.object_count_by_generation(), (0, 1), "tenured after 2 minors");
    }

    /// **UAF regression (aging + generational barrier).** With a raised threshold a
    /// parent can tenure a cycle *before* a young child it points at — and the
    /// parent→child store happened while the parent was young, so no write barrier
    /// fired. A **promotion barrier** must record the newly-old parent so the next
    /// minor GC scans it and keeps the child; otherwise the minor GC frees a live
    /// object. This is the exact scenario a security review caught.
    #[test]
    fn aged_promotion_records_old_to_young_edge_for_minor_gc() {
        let mut heap = FlatHeap::new();
        heap.set_tenure_age(2);

        // `parent` survives one collection → age 1, still young.
        let parent = heap.alloc(16, 0) as usize;
        let _ = heap.collect(&[parent]);
        assert_eq!(heap.object_count_by_generation(), (1, 0), "parent young, age 1");

        // Allocate `child`, store it into `parent` (parent still YOUNG → the write
        // barrier is a no-op and records nothing).
        let child = heap.alloc(16, 0) as usize;
        unsafe { *(parent as *mut usize) = child; }
        unsafe { heap.write_barrier(parent, child); }
        assert_eq!(heap.remembered_len(), 0, "barrier no-op while parent is young");

        // Second full collect: `parent` ages 1→2 → tenured to OLD; `child` (age 1)
        // stays YOUNG. The promotion barrier must now remember `parent`.
        let _ = heap.collect(&[parent]);
        assert_eq!(heap.object_count_by_generation(), (1, 1), "parent old, child young");
        assert_eq!(heap.remembered_len(), 1, "promotion recorded the old→young source");

        // A minor GC rooting only `parent` must KEEP `child` (reached via the
        // remembered old parent), not free it.
        heap.collect_minor(&[parent]);
        assert!(!heap.find_header(child).is_null(), "live child must survive the minor GC");
        assert!(!heap.find_header(parent).is_null(), "parent survives too");
    }

    /// A `0` threshold is meaningless (an object would tenure before surviving
    /// anything); [`FlatHeap::set_tenure_age`] clamps it to `1`, so tenuring stays
    /// well-defined and terminates.
    #[test]
    fn set_tenure_age_clamps_zero_to_one() {
        let mut heap = FlatHeap::new();
        heap.set_tenure_age(0);
        assert_eq!(heap.tenure_age(), 1, "0 clamped to 1");
        let keep = heap.alloc(16, 0) as usize;
        let _ = heap.collect(&[keep]);
        assert_eq!(heap.object_count_by_generation(), (0, 1), "behaves as immediate tenuring");
    }

    // ── Generational minor GC: remembered set + write barrier ──────────────────

    /// A minor GC reclaims young garbage and promotes the young survivor.
    #[test]
    fn minor_gc_reclaims_young_garbage_and_promotes_survivor() {
        let mut heap = FlatHeap::new();
        let keep = heap.alloc(16, 0) as usize;
        heap.alloc(16, 0); // young garbage
        let stats = heap.collect_minor(&[keep]);
        assert_eq!(stats.freed, 1, "young garbage reclaimed");
        assert!(!heap.find_header(keep).is_null());
        assert_eq!(
            heap.object_count_by_generation(),
            (0, 1),
            "survivor tenured"
        );
    }

    /// A minor GC never frees old objects — even unreachable ones. Only a full
    /// collect reclaims the old generation.
    #[test]
    fn minor_gc_never_frees_old_objects() {
        let mut heap = FlatHeap::new();
        let obj = heap.alloc(16, 0) as usize;
        let _ = heap.collect(&[obj]); // tenure to old
        assert_eq!(heap.object_count_by_generation(), (0, 1));

        // `obj` is now unreachable, but a MINOR GC must not touch the old gen.
        let minor = heap.collect_minor(&[]);
        assert_eq!(minor.freed, 0, "minor GC leaves old garbage alone");
        assert!(!heap.find_header(obj).is_null());

        // A full collect reclaims it.
        let full = heap.collect(&[]);
        assert_eq!(full.freed, 1);
        assert!(heap.find_header(obj).is_null());
    }

    /// **The headline barrier proof.** A young object reachable *only* through an
    /// old object survives a minor GC — because the write barrier recorded the
    /// old parent in the remembered set, so the minor scan visits it.
    #[test]
    fn minor_gc_retains_young_reachable_only_via_remembered_old_parent() {
        let mut heap = FlatHeap::new();
        // Make `parent` old.
        let parent = heap.alloc(16, 0) as usize;
        let _ = heap.collect(&[parent]);
        assert_eq!(heap.object_count_by_generation(), (0, 1));

        // Store a fresh young child into parent's field 0, WITH the barrier.
        let child = heap.alloc(16, 0) as usize;
        unsafe {
            *(parent as *mut usize) = child; // old → young store
            heap.write_barrier(parent, child);
        }
        assert_eq!(heap.remembered_len(), 1, "old parent remembered");

        // Minor GC with no external roots: child is reachable only via old parent.
        let stats = heap.collect_minor(&[]);
        assert!(
            !heap.find_header(child).is_null(),
            "the remembered set keeps the old→young pointee alive"
        );
        assert_eq!(stats.freed, 0);
        assert_eq!(heap.object_count_by_generation(), (0, 2), "child tenured");
    }

    /// The remembered set is **load-bearing**: the identical old→young store
    /// *without* the barrier leaves the young child unreachable to the minor scan,
    /// so it is (correctly, given the missed barrier) reclaimed. This proves the
    /// barrier does real work — omitting it would be a use-after-free in a real
    /// program, which is exactly why the barrier contract is mandatory.
    #[test]
    fn minor_gc_without_barrier_frees_young_only_reachable_from_old() {
        let mut heap = FlatHeap::new();
        let parent = heap.alloc(16, 0) as usize;
        let _ = heap.collect(&[parent]); // parent → old
        let child = heap.alloc(16, 0) as usize;
        unsafe {
            *(parent as *mut usize) = child; // store WITHOUT calling write_barrier
        }
        assert_eq!(heap.remembered_len(), 0);

        let stats = heap.collect_minor(&[]);
        assert!(
            heap.find_header(child).is_null(),
            "no remembered entry → the minor scan never visits parent → child freed"
        );
        assert_eq!(stats.freed, 1);
    }

    /// The write barrier records a store only when the *parent* is old (an
    /// old→young pointer is the only kind a minor GC must chase).
    #[test]
    fn write_barrier_records_only_old_parents() {
        let mut heap = FlatHeap::new();
        // Young parent: not remembered.
        let yp = heap.alloc(16, 0) as usize;
        let c = heap.alloc(16, 0) as usize;
        unsafe { heap.write_barrier(yp, c) };
        assert_eq!(heap.remembered_len(), 0, "young parent isn't remembered");

        // Promote a parent to old, then a store records it.
        let op = heap.alloc(16, 0) as usize;
        let _ = heap.collect(&[op, yp, c]); // promote all; clears remembered
        unsafe { heap.write_barrier(op, c) };
        assert_eq!(heap.remembered_len(), 1, "old parent is remembered");
    }

    /// A full collect clears the remembered set (its entries could otherwise
    /// dangle if an old parent were freed).
    #[test]
    fn full_collect_clears_remembered_set() {
        let mut heap = FlatHeap::new();
        let p = heap.alloc(16, 0) as usize;
        let _ = heap.collect(&[p]); // p → old
        let c = heap.alloc(16, 0) as usize;
        unsafe {
            *(p as *mut usize) = c;
            heap.write_barrier(p, c);
        }
        assert_eq!(heap.remembered_len(), 1);
        let _ = heap.collect(&[p, c]); // full collect
        assert_eq!(
            heap.remembered_len(),
            0,
            "remembered set cleared by full GC"
        );
    }

    // ── Precise stack-map roots ─────────────────────────────────────────────

    /// `StackMapTable::from_records` sorts by `pc_offset`; `lookup` is an exact
    /// binary search (hit at each PC; miss between PCs, before the first, after the
    /// last); `len`/`is_empty`/`records` report the sorted contents.
    #[test]
    fn stack_map_table_lookup_is_exact_and_sorted() {
        // Supplied out of order on purpose.
        let table = StackMapTable::from_records(vec![
            StackMapRecord::new(40, vec![8]),
            StackMapRecord::new(8, vec![0]),
            StackMapRecord::new(24, vec![16, 24]),
        ]);
        assert_eq!(table.len(), 3);
        assert!(!table.is_empty());
        // Sorted by pc_offset.
        let pcs: Vec<u32> = table.records().iter().map(|r| r.pc_offset).collect();
        assert_eq!(pcs, vec![8, 24, 40]);
        // Exact hits return the right record.
        assert_eq!(table.lookup(8).unwrap().slots, vec![0]);
        assert_eq!(table.lookup(24).unwrap().slots, vec![16, 24]);
        assert_eq!(table.lookup(40).unwrap().slots, vec![8]);
        // Misses: between records, before the first, after the last.
        assert!(table.lookup(16).is_none());
        assert!(table.lookup(0).is_none());
        assert!(table.lookup(1000).is_none());
        // Empty table looks up to nothing.
        let empty = StackMapTable::default();
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);
        assert!(empty.lookup(0).is_none());
    }

    /// `frame_root_slots` computes `frame_base + offset` for each named slot,
    /// honoring negative (below-FP) offsets, and appends to the caller's vec.
    #[test]
    fn frame_root_slots_computes_signed_addresses() {
        let base: usize = 0x1_0000;
        let rec = StackMapRecord::new(0, vec![0, 8, -16]);
        let mut out = vec![0xdead_usize]; // pre-existing entry is preserved
        frame_root_slots(base, &rec, &mut out);
        assert_eq!(out, vec![0xdead, 0x1_0000, 0x1_0008, 0x1_0000 - 16]);
    }

    /// The precision win: an object named by a stack-map slot survives; an object
    /// whose pointer sits in an *un-named* slot of the very same frame is
    /// reclaimed — the false root a conservative whole-frame scan would have kept
    /// is gone. This is the whole point of precise roots.
    #[test]
    fn collect_precise_keeps_named_frees_unnamed_in_same_frame() {
        let mut heap = FlatHeap::new();
        let a = heap.alloc(16, 0) as usize; // referenced by a NAMED slot → live
        let b = heap.alloc(16, 0) as usize; // referenced only by an UNNAMED slot

        // A simulated stack frame: two adjacent slots, both physically holding heap
        // pointers. `frame[0]` is a live reference; `frame[1]` is dead (e.g. a
        // spilled integer that happens to equal b's address, or a stale ref).
        let frame: [usize; 2] = [a, b];
        // The stack map names ONLY slot 0 (byte offset 0). Slot 1 (offset 8) is not
        // a reference at this PC, so it is not listed.
        let rec = StackMapRecord::new(0, vec![0]);
        let frame_base = frame.as_ptr() as usize;
        let mut roots = Vec::new();
        frame_root_slots(frame_base, &rec, &mut roots);
        assert_eq!(roots, vec![frame_base]); // just the &frame[0] address

        let stats = unsafe { heap.collect_precise(&roots) };
        assert_eq!(stats.freed, 1, "the unnamed-slot object is reclaimed");
        assert_eq!(stats.survived, 1);
        assert!(!heap.find_header(a).is_null(), "named object survives");
        assert!(heap.find_header(b).is_null(), "unnamed object is gone");

        // Keep `frame` alive to the end so the addresses stayed valid.
        assert_eq!(frame[0], a);
    }

    /// Contrast proving the win is real: the *same* two-slot frame scanned
    /// **conservatively** (`collect_region`) retains BOTH objects — b is floating
    /// garbage kept alive by a false root. Precise marking (above) frees it.
    #[test]
    fn collect_region_conservatively_retains_what_precise_frees() {
        let mut heap = FlatHeap::new();
        let a = heap.alloc(16, 0) as usize;
        let b = heap.alloc(16, 0) as usize;
        let frame: [usize; 2] = [a, b];

        let base = frame.as_ptr() as *const u8;
        let len = std::mem::size_of_val(&frame);
        let stats = unsafe { heap.collect_region(base, len) };
        assert_eq!(
            stats.freed, 0,
            "conservative scan keeps both (b is a false root)"
        );
        assert_eq!(stats.survived, 2);
        assert!(!heap.find_header(a).is_null());
        assert!(
            !heap.find_header(b).is_null(),
            "b floats — the imprecision precise roots remove"
        );
        assert_eq!(frame[1], b);
    }

    /// Precise roots still trace interiors transitively: a named parent keeps a
    /// child it points at (raw interior pointer), while an unrelated object dies.
    #[test]
    fn collect_precise_traces_interiors_transitively() {
        let mut heap = FlatHeap::new();
        let child = heap.alloc(16, 0) as usize;
        let parent = heap.alloc(16, 0) as usize;
        unsafe { *(parent as *mut usize) = child }; // parent.field0 -> child
        let _garbage = heap.alloc(16, 0); // unreachable

        let frame: [usize; 1] = [parent];
        let rec = StackMapRecord::new(0, vec![0]);
        let mut roots = Vec::new();
        frame_root_slots(frame.as_ptr() as usize, &rec, &mut roots);

        let stats = unsafe { heap.collect_precise(&roots) };
        assert_eq!(stats.freed, 1, "only the unreachable object is freed");
        assert_eq!(
            stats.survived, 2,
            "parent + transitively-reached child survive"
        );
        assert!(!heap.find_header(parent).is_null());
        assert!(!heap.find_header(child).is_null());
        assert_eq!(frame[0], parent);
    }

    /// The headline for `collect_mixed`: one cycle over a **mapped** frame (exact
    /// slots) and an **unmapped** frame (conservative span). The mapped frame frees
    /// its unnamed-slot look-alike, while the unmapped frame conservatively retains
    /// everything its span names — exactly the per-frame precision a real stack walk
    /// gets when only some frames carry stack maps.
    #[test]
    fn collect_mixed_precise_frame_and_conservative_frame_in_one_cycle() {
        let mut heap = FlatHeap::new();
        let a = heap.alloc(16, 0) as usize; // named by the mapped frame → live
        let b = heap.alloc(16, 0) as usize; // only in an UNNAMED slot → must die
        let c = heap.alloc(16, 0) as usize; // in the unmapped frame's span → live
        let d = heap.alloc(16, 0) as usize; // referenced nowhere → must die

        // Mapped frame: two slots [a, b], stack map names only slot 0.
        let mapped: [usize; 2] = [a, b];
        let rec = StackMapRecord::new(0, vec![0]);
        let mut slots = Vec::new();
        frame_root_slots(mapped.as_ptr() as usize, &rec, &mut slots);

        // Unmapped frame: a raw span holding c (and a non-pointer word). No stack
        // map, so it is scanned conservatively as one region.
        let unmapped: [usize; 2] = [c, 0xdead_beef];
        let region = (
            unmapped.as_ptr() as *const u8,
            std::mem::size_of_val(&unmapped),
        );

        let stats = unsafe { heap.collect_mixed(&slots, &[region]) };
        assert_eq!(stats.freed, 2, "b (unnamed) and d (unreferenced) are reclaimed");
        assert_eq!(stats.survived, 2, "a (named) and c (span) survive");
        assert!(!heap.find_header(a).is_null(), "a survives (precise root)");
        assert!(heap.find_header(b).is_null(), "b freed (unnamed in mapped frame)");
        assert!(!heap.find_header(c).is_null(), "c survives (conservative span)");
        assert!(heap.find_header(d).is_null(), "d freed (rooted nowhere)");

        // Keep both frames alive so their addresses stayed valid through the scan.
        assert_eq!(mapped[0], a);
        assert_eq!(unmapped[0], c);
    }

    /// `collect_mixed(slots, &[])` is exactly `collect_precise(slots)`: with no
    /// regions it frees an unnamed-slot look-alike just as the precise collector does.
    #[test]
    fn collect_mixed_slots_only_equals_collect_precise() {
        let mut heap = FlatHeap::new();
        let a = heap.alloc(16, 0) as usize;
        let b = heap.alloc(16, 0) as usize;
        let frame: [usize; 2] = [a, b];
        let rec = StackMapRecord::new(0, vec![0]); // names only slot 0 (a)
        let mut slots = Vec::new();
        frame_root_slots(frame.as_ptr() as usize, &rec, &mut slots);

        let stats = unsafe { heap.collect_mixed(&slots, &[]) };
        assert_eq!(stats.freed, 1, "the unnamed-slot object is reclaimed");
        assert!(!heap.find_header(a).is_null());
        assert!(heap.find_header(b).is_null());
        assert_eq!(frame[0], a);
    }

    /// `collect_mixed(&[], &[(base, len)])` is exactly `collect_region(base, len)`:
    /// with no slots it conservatively retains everything the span look-alikes.
    #[test]
    fn collect_mixed_regions_only_equals_collect_region() {
        let mut heap = FlatHeap::new();
        let a = heap.alloc(16, 0) as usize;
        let b = heap.alloc(16, 0) as usize;
        let frame: [usize; 2] = [a, b]; // both physically in the span
        let region = (frame.as_ptr() as *const u8, std::mem::size_of_val(&frame));

        let stats = unsafe { heap.collect_mixed(&[], &[region]) };
        assert_eq!(stats.freed, 0, "conservative span retains both");
        assert_eq!(stats.survived, 2);
        assert!(!heap.find_header(a).is_null());
        assert!(!heap.find_header(b).is_null());
        assert_eq!(frame[1], b);
    }

    /// `collect_mixed(&[], &[])` roots from nothing and frees the whole heap — the
    /// degenerate case both siblings share.
    #[test]
    fn collect_mixed_empty_frees_all() {
        let mut heap = FlatHeap::new();
        let _a = heap.alloc(16, 0);
        let _b = heap.alloc(16, 0);
        let stats = unsafe { heap.collect_mixed(&[], &[]) };
        assert_eq!(stats.freed, 2);
        assert_eq!(stats.survived, 0);
        assert_eq!(heap.live_bytes(), 0);
    }

    /// Multiple regions all contribute roots in one cycle: two disjoint spans plus a
    /// precise slot, each keeping its own object; a fourth object rooted nowhere dies.
    #[test]
    fn collect_mixed_multiple_regions_all_root() {
        let mut heap = FlatHeap::new();
        let p = heap.alloc(16, 0) as usize; // precise slot
        let r1 = heap.alloc(16, 0) as usize; // region 1
        let r2 = heap.alloc(16, 0) as usize; // region 2
        let _dead = heap.alloc(16, 0); // rooted nowhere

        let pframe: [usize; 1] = [p];
        let rec = StackMapRecord::new(0, vec![0]);
        let mut slots = Vec::new();
        frame_root_slots(pframe.as_ptr() as usize, &rec, &mut slots);

        let span1: [usize; 1] = [r1];
        let span2: [usize; 1] = [r2];
        let regions = [
            (span1.as_ptr() as *const u8, std::mem::size_of_val(&span1)),
            (span2.as_ptr() as *const u8, std::mem::size_of_val(&span2)),
        ];

        let stats = unsafe { heap.collect_mixed(&slots, &regions) };
        assert_eq!(stats.freed, 1, "only the unrooted object dies");
        assert_eq!(stats.survived, 3);
        assert!(!heap.find_header(p).is_null());
        assert!(!heap.find_header(r1).is_null());
        assert!(!heap.find_header(r2).is_null());
        assert_eq!((pframe[0], span1[0], span2[0]), (p, r1, r2));
    }

    /// Degenerate regions in the slice are inert: a `(null, 0)` tuple and a
    /// sub-word (`len < 8`) region read nothing (per the documented contract), while
    /// a real slot root alongside them still keeps its object. Pins the "`base` may
    /// be null iff `len == 0`" guarantee against future edits.
    #[test]
    fn collect_mixed_tolerates_degenerate_regions() {
        let mut heap = FlatHeap::new();
        let live = heap.alloc(16, 0) as usize; // kept by a precise slot
        let _dead = heap.alloc(16, 0); // rooted nowhere

        let frame: [usize; 1] = [live];
        let rec = StackMapRecord::new(0, vec![0]);
        let mut slots = Vec::new();
        frame_root_slots(frame.as_ptr() as usize, &rec, &mut slots);

        // A null/empty region and a 4-byte region — both scan nothing.
        let tiny: [u8; 4] = [1, 2, 3, 4];
        let regions = [
            (std::ptr::null::<u8>(), 0usize),
            (tiny.as_ptr(), tiny.len()), // len < 8 → mark_region reads nothing
        ];

        let stats = unsafe { heap.collect_mixed(&slots, &regions) };
        assert_eq!(stats.freed, 1, "degenerate regions add no roots; dead object dies");
        assert!(!heap.find_header(live).is_null(), "the slot-rooted object survives");
        assert_eq!(frame[0], live);
    }

    /// Precise interior tracing composes with precise roots: a registered `kind`
    /// with one ref field follows only that field, so an integer look-alike in a
    /// non-ref field of a precisely-rooted object pins nothing.
    #[test]
    fn collect_precise_honors_registered_kind_field_map() {
        let mut heap = FlatHeap::new();
        // kind 1: a 16-byte object whose only ref field is at offset 0.
        let kind = heap.register_kind(&[0]);
        let target = heap.alloc(16, 0) as usize; // reached via the ref field → live
        let phantom = heap.alloc(16, 0) as usize; // look-alike in a NON-ref field
        let obj = heap.alloc(16, kind) as usize;
        unsafe {
            *(obj as *mut usize) = target; // field0 (ref) -> target
            *((obj + 8) as *mut usize) = phantom; // field1 (non-ref) -> phantom
        }

        let frame: [usize; 1] = [obj];
        let rec = StackMapRecord::new(0, vec![0]);
        let mut roots = Vec::new();
        frame_root_slots(frame.as_ptr() as usize, &rec, &mut roots);

        let stats = unsafe { heap.collect_precise(&roots) };
        assert!(!heap.find_header(obj).is_null(), "rooted object survives");
        assert!(
            !heap.find_header(target).is_null(),
            "ref-field target survives"
        );
        assert!(
            heap.find_header(phantom).is_null(),
            "non-ref-field look-alike is reclaimed (precise interior tracing)"
        );
        assert_eq!(stats.freed, 1);
        assert_eq!(frame[0], obj);
    }

    /// An empty root set frees everything (no roots named → nothing live).
    #[test]
    fn collect_precise_empty_roots_frees_all() {
        let mut heap = FlatHeap::new();
        let _a = heap.alloc(16, 0);
        let _b = heap.alloc(24, 0);
        assert_eq!(heap.object_count(), 2);
        let stats = unsafe { heap.collect_precise(&[]) };
        assert_eq!(stats.freed, 2);
        assert_eq!(stats.survived, 0);
        assert_eq!(heap.live_bytes(), 0);
    }

    /// Multiple mapped frames accumulate into one flat root list, then one precise
    /// collect — the shape the native walker produces (one `collect_precise` after
    /// walking the whole chain).
    #[test]
    fn collect_precise_across_multiple_frames() {
        let mut heap = FlatHeap::new();
        let x = heap.alloc(16, 0) as usize; // rooted from "frame 0"
        let y = heap.alloc(16, 0) as usize; // rooted from "frame 1"
        let _dead = heap.alloc(16, 0);

        let frame0: [usize; 1] = [x];
        let frame1: [usize; 2] = [0xabc, y]; // slot 0 unnamed, slot 1 (offset 8) named
        let mut roots = Vec::new();
        frame_root_slots(
            frame0.as_ptr() as usize,
            &StackMapRecord::new(0, vec![0]),
            &mut roots,
        );
        frame_root_slots(
            frame1.as_ptr() as usize,
            &StackMapRecord::new(0, vec![8]),
            &mut roots,
        );

        let stats = unsafe { heap.collect_precise(&roots) };
        assert_eq!(stats.survived, 2, "x and y across two frames");
        assert_eq!(stats.freed, 1);
        assert!(!heap.find_header(x).is_null());
        assert!(!heap.find_header(y).is_null());
        assert_eq!((frame0[0], frame1[1]), (x, y));
    }

    // ── Moving/compacting collector — mobility classification (AOT00-T3 PR-2) ──

    /// A registered-kind object reachable ONLY through a precise slot is **movable**.
    #[test]
    fn mobility_precise_only_registered_kind_is_movable() {
        let mut heap = FlatHeap::new();
        let k = heap.register_kind(&[0]); // kind 1: one ref field at offset 0
        let a = heap.alloc(16, k) as usize;

        // A precise root slot: the *address* of a word holding `a`.
        let slot_word = a;
        let slots = [&slot_word as *const usize as usize];
        let movable = unsafe { heap.classify_mobility(&slots, &[]) };

        assert!(movable.contains(&a), "precise-reachable registered-kind object is movable");
    }

    /// The SAME object gains a **conservative** in-edge (a region root) → it is now
    /// **pinned**, not movable, even though it is still precisely reachable. This is
    /// the load-bearing rule: any conservative in-edge wins (a maybe-pointer to its
    /// old address could not be rewritten if it moved).
    #[test]
    fn mobility_conservative_in_edge_pins_even_when_precise() {
        let mut heap = FlatHeap::new();
        let k = heap.register_kind(&[0]);
        let a = heap.alloc(16, k) as usize;

        let precise_word = a;
        let cons_word = a;
        let slots = [&precise_word as *const usize as usize];
        let region = [(&cons_word as *const usize as *const u8, 8usize)];

        // Control: precise-only → movable.
        assert!(unsafe { heap.classify_mobility(&slots, &[]) }.contains(&a));
        // With the conservative region → pinned (removed from the movable set).
        let movable = unsafe { heap.classify_mobility(&slots, &region) };
        assert!(
            !movable.contains(&a),
            "a conservative in-edge pins a precisely-reachable object",
        );
    }

    /// A `kind == 0` (conservatively-traced) object is **never** movable — its own
    /// pointers cannot be safely rewritten — even when reached only via a precise slot.
    #[test]
    fn mobility_kind_zero_object_is_never_movable() {
        let mut heap = FlatHeap::new();
        let a = heap.alloc(16, 0) as usize; // opaque / conservative kind
        let slot_word = a;
        let slots = [&slot_word as *const usize as usize];

        let movable = unsafe { heap.classify_mobility(&slots, &[]) };
        assert!(!movable.contains(&a), "kind==0 object is never movable");
    }

    /// Movability is transitive along a **precise** chain of registered-kind objects
    /// (parent → child both movable), but a `kind == 0` parent's child — reached only
    /// through the parent's conservative out-edge — is pinned (and the kind==0 parent
    /// itself is never movable).
    #[test]
    fn mobility_transitive_precise_chain_vs_conservative_parent() {
        let mut heap = FlatHeap::new();
        let k = heap.register_kind(&[0]);

        // Precise chain: P --(ref field 0)--> C, both registered kind.
        let p = heap.alloc(16, k) as usize;
        let c = heap.alloc(16, k) as usize;
        unsafe { *(p as *mut usize) = c };

        // A kind==0 parent Q --(conservative word)--> D (registered kind).
        let q = heap.alloc(16, 0) as usize;
        let d = heap.alloc(16, k) as usize;
        unsafe { *(q as *mut usize) = d };

        let root_p = p;
        let root_q = q;
        let slots = [
            &root_p as *const usize as usize,
            &root_q as *const usize as usize,
        ];
        let movable = unsafe { heap.classify_mobility(&slots, &[]) };

        assert!(movable.contains(&p), "precise parent movable");
        assert!(movable.contains(&c), "precise child (transitive) movable");
        assert!(!movable.contains(&q), "kind==0 parent never movable");
        assert!(
            !movable.contains(&d),
            "child reached only via a kind==0 (conservative) parent is pinned",
        );
    }

    /// Transitive conservative pinning across **two `kind == 0` hops** from a region
    /// root: a registered object reached via `region → q1(kind0) → q2(kind0) →
    /// registered` is pinned even though it is *also* precisely rooted. Exercises
    /// `conservative_children`'s multi-hop closure (the UAF-critical over-approximation).
    #[test]
    fn mobility_transitive_conservative_chain_pins_registered_leaf() {
        let mut heap = FlatHeap::new();
        let k = heap.register_kind(&[0]);

        let r = heap.alloc(16, k) as usize; // registered leaf — would be movable alone
        let q2 = heap.alloc(16, 0) as usize; // kind==0
        let q1 = heap.alloc(16, 0) as usize; // kind==0, the region's target
        unsafe {
            *(q1 as *mut usize) = q2; // q1 -> q2 (conservative)
            *(q2 as *mut usize) = r; //  q2 -> r  (conservative)
        }

        // r is ALSO precisely rooted, so only the conservative chain can pin it.
        let precise_word = r;
        let slots = [&precise_word as *const usize as usize];
        let region_word = q1;
        let region = [(&region_word as *const usize as *const u8, 8usize)];

        let movable = unsafe { heap.classify_mobility(&slots, &region) };
        assert!(
            !movable.contains(&r),
            "registered leaf reached via a 2-hop kind==0 conservative chain is pinned",
        );
    }

    /// Reclassifying is idempotent: the transient pin bits are cleared each call, so
    /// removing a conservative root restores an object to movable.
    #[test]
    fn mobility_pin_bits_are_transient_across_classifications() {
        let mut heap = FlatHeap::new();
        let k = heap.register_kind(&[0]);
        let a = heap.alloc(16, k) as usize;

        let word = a;
        let slots = [&word as *const usize as usize];
        let region = [(&word as *const usize as *const u8, 8usize)];

        // Pin it, then classify again without the region — it must be movable again.
        assert!(!unsafe { heap.classify_mobility(&slots, &region) }.contains(&a));
        assert!(
            unsafe { heap.classify_mobility(&slots, &[]) }.contains(&a),
            "pin bit is a per-classification transient, cleared each call",
        );
    }

    // ── Moving/compacting collector — arena + copy scaffold (AOT00-T3 PR-3a) ──

    /// A movable object is copied **byte-identically** (header + payload) into the arena
    /// at a fresh 16-aligned address, and recorded in the forwarding map. The from-space
    /// original is untouched (no fixup, no free — this is the dry-run scaffold).
    #[test]
    fn compaction_copies_movable_object_byte_identical() {
        let mut heap = FlatHeap::new();
        let k = heap.register_kind(&[0]);
        let a = heap.alloc(16, k) as usize;
        unsafe { *(a as *mut u64) = 0xDEAD_BEEF_1234_5678 }; // payload content

        let word = a;
        let slots = [&word as *const usize as usize];
        let (arena, forward) = unsafe { heap.plan_compaction(&slots, &[]) };

        let na = *forward.get(&a).expect("movable object is forwarded");
        assert_ne!(na, a, "copied to a fresh address");
        assert_eq!(na % ALIGN, 0, "arena payload is 16-aligned");
        assert!(
            na >= arena.base as usize && na < arena.base as usize + arena.cap,
            "new address lies inside the arena",
        );

        // The 16-byte PAYLOAD is copied byte-for-byte; the header's size/kind match; and
        // the copy's `arena_backed` provenance byte is the one intentional difference
        // (set to `true` on the copy, `false` on the malloc'd original).
        let old_pl = unsafe { std::slice::from_raw_parts(a as *const u8, 16) };
        let new_pl = unsafe { std::slice::from_raw_parts(na as *const u8, 16) };
        assert_eq!(old_pl, new_pl, "payload copied byte-for-byte");
        let oh = (a - HEADER_SIZE) as *const FlatHeader;
        let nh = (na - HEADER_SIZE) as *const FlatHeader;
        unsafe {
            assert_eq!((*oh).size, (*nh).size);
            assert_eq!((*oh).kind, (*nh).kind);
            assert!(!(*oh).arena_backed, "malloc'd original is not arena-backed");
            assert!((*nh).arena_backed, "arena copy is arena-backed");
        }
        drop(arena);
    }

    /// Pinned objects are **never** copied — the forwarding map excludes them and the
    /// arena is empty when nothing is movable.
    #[test]
    fn compaction_skips_pinned_and_empty_when_all_pinned() {
        let mut heap = FlatHeap::new();
        let k = heap.register_kind(&[0]);
        let a = heap.alloc(16, k) as usize;

        let word = a;
        let region = [(&word as *const usize as *const u8, 8usize)];
        let (arena, forward) = unsafe { heap.plan_compaction(&[], &region) };

        assert!(!forward.contains_key(&a), "a pinned object is not evacuated");
        assert!(forward.is_empty(), "nothing movable → empty forwarding map");
        assert_eq!(arena.cap, 0, "nothing movable → zero-capacity arena");
        drop(arena);
    }

    /// The forwarding map's keys are **exactly** the movable set, and every mapped address
    /// is distinct — one arena slot per moved object, no aliasing.
    #[test]
    fn compaction_forwarding_map_matches_movable_set() {
        let mut heap = FlatHeap::new();
        let k = heap.register_kind(&[0]);
        let p = heap.alloc(16, k) as usize;
        let c = heap.alloc(16, k) as usize;
        unsafe { *(p as *mut usize) = c }; // p --ref--> c (both movable)

        let root = p;
        let slots = [&root as *const usize as usize];

        let movable = unsafe { heap.classify_mobility(&slots, &[]) };
        let (arena, forward) = unsafe { heap.plan_compaction(&slots, &[]) };

        let keys: HashSet<usize> = forward.keys().copied().collect();
        assert_eq!(keys, movable, "exactly the movable objects are forwarded");
        let news: HashSet<usize> = forward.values().copied().collect();
        assert_eq!(news.len(), forward.len(), "each object gets a distinct arena address");
        drop(arena);
    }

    // ── Moving/compacting collector — evacuate + pointer fixup (AOT00-T3 PR-3b) ──

    /// The headline move differential: a precise-rooted registered object and its
    /// registered child both MOVE; the root slot is rewritten to the parent's new
    /// address, and the parent's (arena-copy) reference field is rewritten to the
    /// child's new address — so deref-through the rewritten root reaches the child at
    /// its NEW arena location. (Arena kept alive across the assertions.)
    #[test]
    fn evacuate_moves_precise_chain_and_rewrites_root_and_interior() {
        let mut heap = FlatHeap::new();
        let k = heap.register_kind(&[0]); // one ref field at offset 0
        let a = heap.alloc(16, k) as usize;
        let b = heap.alloc(16, k) as usize;
        unsafe { *(a as *mut usize) = b }; // a.field0 -> b

        let root = a; // a precise root slot holding A's address
        let slots = [&root as *const usize as usize];
        let (arena, forward) = unsafe { heap.evacuate_and_fixup(&slots, &[]) };

        let na = *forward.get(&a).expect("A moved");
        let nb = *forward.get(&b).expect("B moved (reached precisely via A)");
        assert_eq!(root, na, "root slot rewritten to A's new address");
        let a_field = unsafe { *(na as *const usize) };
        assert_eq!(a_field, nb, "moved A's ref field now points at moved B's new address");
        assert!(na >= arena.base as usize && na < arena.base as usize + arena.cap);
        assert!(nb >= arena.base as usize && nb < arena.base as usize + arena.cap);
        drop(arena);
    }

    /// A conservatively-rooted object PINS — even when *also* precisely rooted: it is
    /// not moved and its root slot is left unchanged (a conservative pointer to it can't
    /// be rewritten, so it must stay put).
    #[test]
    fn evacuate_conservative_in_edge_object_pins_and_root_unchanged() {
        let mut heap = FlatHeap::new();
        let k = heap.register_kind(&[0]);
        let a = heap.alloc(16, k) as usize;

        let precise = a;
        let cons = a;
        let slots = [&precise as *const usize as usize];
        let region = [(&cons as *const usize as *const u8, 8usize)];
        let (arena, forward) = unsafe { heap.evacuate_and_fixup(&slots, &region) };

        assert!(!forward.contains_key(&a), "a conservative in-edge pins the object");
        assert_eq!(precise, a, "a pinned object's precise root slot is left unchanged");
        assert_eq!(arena.cap, 0, "nothing moved");
        drop(arena);
    }

    /// A **tagged** interior pointer (low-3 NaN-box tag) is fixed up to the child's new
    /// address with the tag reattached.
    #[test]
    fn evacuate_fixes_tagged_interior_pointer_preserving_tag() {
        let mut heap = FlatHeap::new();
        let k = heap.register_kind(&[0]);
        let a = heap.alloc(16, k) as usize;
        let b = heap.alloc(16, k) as usize;
        let tag = 0x3usize;
        unsafe { *(a as *mut usize) = b | tag }; // a.field0 -> b, tagged

        let root = a;
        let slots = [&root as *const usize as usize];
        let (arena, forward) = unsafe { heap.evacuate_and_fixup(&slots, &[]) };

        let nb = *forward.get(&b).expect("tagged child still reached + moved");
        let na = *forward.get(&a).expect("A moved");
        let a_field = unsafe { *(na as *const usize) };
        assert_eq!(a_field, nb | tag, "tagged pointer fixed up, tag preserved");
        drop(arena);
    }

    // ── Moving collector — arena provenance safety (AOT00-T3 PR-3c-1) ──
    //
    // An arena-backed block is a SLICE of one big arena allocation, so it must never be
    // handed to `dealloc` individually. These integrate an arena copy into the heap and
    // exercise both `dealloc` sites (sweep, Drop). Run under Miri to catch a double-free
    // or a dealloc of an arena slice.

    /// Copy an object into a fresh arena (marked `arena_backed`), splice it onto the
    /// heap's all-list, and retain the arena — the shared setup for the two tests below.
    /// Returns the arena-copy header. (Standing in for the real integration PR-3c-2 does.)
    unsafe fn integrate_one_arena_copy(heap: &mut FlatHeap) -> *mut FlatHeader {
        let k = heap.register_kind(&[0]);
        let a = heap.alloc(16, k) as usize;
        let root = a;
        let slots = [&root as *const usize as usize];
        let (arena, forward) = heap.plan_compaction(&slots, &[]);
        let na = *forward.get(&a).expect("A moved");
        let new_h = (na - HEADER_SIZE) as *mut FlatHeader;
        assert!((*new_h).arena_backed, "arena copy is marked arena-backed");
        // Splice the arena copy onto the all-list (the from-space original stays too).
        (*new_h).next = heap.all;
        heap.all = new_h;
        heap.arenas.push(arena);
        new_h
    }

    /// **Drop** never `dealloc`s an arena-backed block: a live arena copy present on the
    /// all-list at teardown is freed only when its `Arena` drops (after `Drop::drop`), so
    /// its storage is released exactly once — no double-free, no dealloc of an arena slice.
    #[test]
    fn arena_backed_block_not_freed_by_drop() {
        let mut heap = FlatHeap::new();
        unsafe { integrate_one_arena_copy(&mut heap) };
        // all-list = [arena_copy, malloc'd original]. Dropping frees the malloc block and
        // skips the arena copy (freed as `arenas` drops). Miri catches any double-free.
        drop(heap);
    }

    /// **Sweep** never `dealloc`s an arena-backed block: a collection that finds the arena
    /// copy unreachable unlinks it from the all-list but does not free it (its arena will).
    #[test]
    fn arena_backed_block_not_freed_by_sweep() {
        let mut heap = FlatHeap::new();
        unsafe { integrate_one_arena_copy(&mut heap) };
        // Collect rooting nothing: both blocks are unreachable. The malloc original is
        // dealloc'd; the arena copy is only unlinked (arena-backed → not freed here).
        let _ = heap.collect(&[]);
        // Teardown then frees the retained arena. No double-free / no arena-slice dealloc.
        drop(heap);
    }

    // ── Moving collector — the full moving cycle `collect_compacting` (AOT00-T3 PR-3c-2) ──
    //
    // These run a COMPLETE relocating collection and then keep using the heap: deref the
    // rewritten roots, read moved payloads, and collect again. Run under Miri to catch the
    // UAF surface — a double-free of a from-space block, a dealloc of an arena slice as if
    // malloc'd, or a walk over a stale/corrupt `next` link after integration.

    /// **The headline executing differential.** A precise chain `a → b` (both movable) is
    /// evacuated; an unreachable `c` is reclaimed. Afterwards the heap holds exactly the two
    /// moved copies, the root is rewritten to `a`'s new arena address, dereferencing it
    /// reaches `b`'s new address, and a sentinel written into `b` before the collection is
    /// byte-preserved at the new location. Then a SECOND `collect_compacting` (rooting the
    /// already-moved `a`) re-moves the arena copies — exercising relocation of an
    /// arena-backed object and the re-threaded all-list — and the sentinel still survives.
    #[test]
    fn collect_compacting_moves_chain_reclaims_garbage_and_preserves_values() {
        let mut heap = FlatHeap::new();
        let k = heap.register_kind(&[0]); // one ref field at offset 0
        let a = heap.alloc(16, k) as usize;
        let b = heap.alloc(16, k) as usize;
        let _c = heap.alloc(16, k) as usize; // unreachable garbage
        unsafe {
            *(a as *mut usize) = b; // a.field0 -> b
            *(b as *mut usize) = 0; // b.field0 -> null (leaf)
            *((b + 8) as *mut usize) = 0xDEAD_BEEF_usize; // sentinel in b's non-ref word
        }

        let root = a;
        let slots = [&root as *const usize as usize];
        let stats = unsafe { heap.collect_compacting(&slots, &[]) };

        assert_eq!(stats.freed, 1, "the unreachable c was reclaimed");
        assert_eq!(stats.survived, 2, "a and b survived (moved)");
        assert_eq!(heap.object_count(), 2, "heap holds exactly the two moved copies");
        assert_eq!(heap.live_bytes(), 32, "live bytes = 2 × 16");
        assert_eq!(heap.remembered_len(), 0, "all survivors young → empty remembered set");

        let na = root; // root slot was rewritten in place to a's new address
        assert_ne!(na, a, "a moved to a new address");
        let nb = unsafe { *(na as *const usize) };
        assert_ne!(nb, b, "b moved; a's ref field points at b's new address");
        assert_eq!(
            unsafe { *((nb + 8) as *const usize) },
            0xDEAD_BEEF,
            "b's sentinel byte-preserved across the move",
        );

        // Second cycle: re-move the arena-backed copies (root still holds na).
        let stats2 = unsafe { heap.collect_compacting(&slots, &[]) };
        assert_eq!(stats2.survived, 2, "both survive the second compaction");
        assert_eq!(heap.object_count(), 2);
        let na2 = root;
        assert_ne!(na2, na, "a re-moved into a fresh arena");
        let nb2 = unsafe { *(na2 as *const usize) };
        assert_eq!(
            unsafe { *((nb2 + 8) as *const usize) },
            0xDEAD_BEEF,
            "sentinel survives a second relocation",
        );
        drop(heap); // frees both retained arenas + any malloc survivors, each exactly once
    }

    /// **Strict generalization (spec §4):** with nothing movable, `collect_compacting`
    /// behaves as `collect_mixed` — same survivors, same frees, and pinned objects keep
    /// their address (no move). Built on `kind == 0` objects reached conservatively, where
    /// the two collectors' reachability coincides exactly.
    #[test]
    fn collect_compacting_all_pinned_matches_collect_mixed() {
        // Twin heaps, identical shape: a conservatively-rooted survivor `p` + garbage `g`.
        let build = || {
            let mut heap = FlatHeap::new();
            let p = heap.alloc(16, 0) as usize; // kind 0 → conservative → pins
            let _g = heap.alloc(16, 0) as usize; // unreachable
            (heap, p)
        };
        let (mut hm, pm) = build();
        let (mut hc, pc) = build();

        let region_m = [(&pm as *const usize as *const u8, 8usize)];
        let region_c = [(&pc as *const usize as *const u8, 8usize)];
        let sm = unsafe { hm.collect_mixed(&[], &region_m) };
        let sc = unsafe { hc.collect_compacting(&[], &region_c) };

        assert_eq!(sc.freed, sm.freed, "same objects freed as collect_mixed");
        assert_eq!(sc.survived, sm.survived, "same survivor count");
        assert_eq!(hc.object_count(), hm.object_count(), "same live count");
        assert_eq!(hc.live_bytes(), hm.live_bytes(), "same live bytes");
        assert_eq!(sc.survived, 1, "just the pinned survivor");
        // A pinned (kind 0) object is never relocated: its address is unchanged.
        assert_eq!(hc.find_header(pc), (pc - HEADER_SIZE) as *mut FlatHeader);
    }

    /// **UAF stress.** A wider movable graph with garbage, a partial root set, then heavy
    /// reuse of the survivors (deref + write) and a second collection. Under Miri this
    /// exercises: reclaiming from-space originals without freeing a still-referenced block,
    /// never `dealloc`ing an arena slice, and walking the re-threaded all-list.
    #[test]
    fn collect_compacting_reuse_and_recollect_no_uaf() {
        let mut heap = FlatHeap::new();
        let k = heap.register_kind(&[0]);
        // Five movable objects; root only #0 and #2, each pointing at the next.
        let objs: Vec<usize> = (0..5).map(|_| heap.alloc(16, k) as usize).collect();
        unsafe {
            *(objs[0] as *mut usize) = objs[1]; // 0 -> 1
            *(objs[2] as *mut usize) = objs[3]; // 2 -> 3
            *(objs[1] as *mut usize) = 0;
            *(objs[3] as *mut usize) = 0;
            // #4 is unreachable garbage.
        }
        let r0 = objs[0];
        let r2 = objs[2];
        let slots = [&r0 as *const usize as usize, &r2 as *const usize as usize];

        let stats = unsafe { heap.collect_compacting(&slots, &[]) };
        assert_eq!(stats.freed, 1, "only the unreachable #4 is freed");
        assert_eq!(heap.object_count(), 4, "0,1,2,3 survive (moved)");

        // Reuse: deref both rewritten roots to reach their children, and write through them.
        unsafe {
            let c0 = *(r0 as *const usize);
            let c2 = *(r2 as *const usize);
            *((c0 + 8) as *mut usize) = 111;
            *((c2 + 8) as *mut usize) = 222;
        }
        // Second collection keeps everything; then verify the writes survived.
        let stats2 = unsafe { heap.collect_compacting(&slots, &[]) };
        assert_eq!(stats2.freed, 0, "nothing new to free");
        assert_eq!(heap.object_count(), 4);
        unsafe {
            let c0 = *(r0 as *const usize);
            let c2 = *(r2 as *const usize);
            assert_eq!(*((c0 + 8) as *const usize), 111);
            assert_eq!(*((c2 + 8) as *const usize), 222);
        }
        drop(heap);
    }

    /// Degenerate: no roots and no regions → everything is unreachable and reclaimed, the
    /// heap empties, and the (empty) arena is retained harmlessly. Mirrors
    /// `collect_mixed_empty_frees_all` for the compacting entry.
    #[test]
    fn collect_compacting_empty_roots_frees_all() {
        let mut heap = FlatHeap::new();
        let k = heap.register_kind(&[0]);
        heap.alloc(16, k);
        heap.alloc(16, k);
        let stats = unsafe { heap.collect_compacting(&[], &[]) };
        assert_eq!(stats.freed, 2, "both unreachable objects freed");
        assert_eq!(heap.object_count(), 0, "heap emptied");
        assert_eq!(heap.live_bytes(), 0);
        drop(heap);
    }

    // ── Incremental (bounded-pause) collector — tri-colour marking (AOT00-T4 PR-1) ──
    //
    // `incremental_start → step* → finish` decomposes a full mark-sweep's MARK into bounded
    // slices. These prove the decomposition is faithful (same reclamation as one atomic
    // collect), that a step is bounded, and that an object born during a mark survives it.
    // Roots-only graphs (no mutation mid-mark) — the write barrier is PR-2.

    /// **Stepping ≡ stop-the-world.** A precise `a → b` chain plus unreachable garbage `c`,
    /// marked incrementally one object per step, frees EXACTLY what a single atomic
    /// `collect_mixed` with the same root frees — same freed count, survivors, and live bytes.
    /// Incremental is a decomposition of the mark, not a different reachability.
    #[test]
    fn incremental_step_by_step_equals_stop_the_world() {
        // Twin heaps, identical shape: register a 1-ref-field kind, build a → b, leak c.
        let build = || {
            let mut heap = FlatHeap::new();
            let k = heap.register_kind(&[0]);
            let a = heap.alloc(16, k) as usize;
            let b = heap.alloc(16, k) as usize;
            unsafe { *(a as *mut usize) = b }; // a.field0 -> b
            let _c = heap.alloc(16, k); // unreachable garbage
            (heap, a)
        };
        let (mut hi, ai) = build();
        let (mut hs, as_) = build();

        // Stop-the-world reference.
        let root_s = as_;
        let s_stats = unsafe { hs.collect_mixed(&[&root_s as *const usize as usize], &[]) };

        // Incremental: root a, then mark ONE object per step until done, then finish.
        let root_i = ai;
        let slots = [&root_i as *const usize as usize];
        unsafe { hi.incremental_start(&slots, &[]) };
        assert!(hi.incremental_in_progress());
        let mut steps = 0;
        while !unsafe { hi.incremental_step(1) } {
            steps += 1;
            assert!(steps < 100, "must converge");
        }
        let i_stats = unsafe { hi.incremental_finish() };

        assert!(!hi.incremental_in_progress(), "phase ended at finish");
        assert_eq!(i_stats.freed, s_stats.freed, "same objects freed as stop-the-world");
        assert_eq!(i_stats.survived, s_stats.survived, "same survivor count");
        assert_eq!(hi.object_count(), hs.object_count(), "same live count");
        assert_eq!(hi.live_bytes(), hs.live_bytes(), "same live bytes");
        assert_eq!(i_stats.freed, 1, "just the garbage c");
        assert_eq!(hi.object_count(), 2, "a and b survive");
    }

    /// **A step is bounded.** A root object with three registered children: after `start` the
    /// grey frontier is just the root; `step(1)` scans exactly that one object (greying its
    /// three children, so the frontier grows to 3) and reports *not done*; a `step(3)` then
    /// drains the leaves to completion. The per-step scan is bounded by the budget.
    #[test]
    fn incremental_step_scans_at_most_budget() {
        let mut heap = FlatHeap::new();
        let k = heap.register_kind(&[0, 8, 16]); // three ref fields
        let c1 = heap.alloc(16, k) as usize;
        let c2 = heap.alloc(16, k) as usize;
        let c3 = heap.alloc(16, k) as usize;
        let r = heap.alloc(24, k) as usize; // holds all three children
        unsafe {
            *(r as *mut usize) = c1;
            *((r + 8) as *mut usize) = c2;
            *((r + 16) as *mut usize) = c3;
        }

        let root = r;
        let slots = [&root as *const usize as usize];
        unsafe { heap.incremental_start(&slots, &[]) };
        assert_eq!(heap.incremental_grey_count(), 1, "only the root is grey at start");

        // One step scans exactly the root → greys its 3 children, not yet done.
        let done1 = unsafe { heap.incremental_step(1) };
        assert!(!done1, "one step over a 4-object graph is not complete");
        assert_eq!(heap.incremental_grey_count(), 3, "root scanned; its 3 children now grey");

        // Drain the three leaves; now complete.
        let done2 = unsafe { heap.incremental_step(3) };
        assert!(done2, "the three leaves are scanned to completion");
        assert_eq!(heap.incremental_grey_count(), 0);

        let stats = unsafe { heap.incremental_finish() };
        assert_eq!(stats.freed, 0, "everything reachable — nothing freed");
        assert_eq!(heap.object_count(), 4);
    }

    /// **An object born during a mark survives the cycle** (alloc-black). Allocated *between*
    /// two steps, unrooted, it must not be swept by the cycle that was already running — its
    /// reachable snapshot was fixed at `start`, so it is coloured black on birth.
    #[test]
    fn incremental_new_object_during_mark_is_retained() {
        let mut heap = FlatHeap::new();
        let k = heap.register_kind(&[0]);
        let a = heap.alloc(16, k) as usize; // the rooted survivor

        let root = a;
        let slots = [&root as *const usize as usize];
        unsafe { heap.incremental_start(&slots, &[]) };
        // Allocate mid-mark — born black. Deliberately rooted NOWHERE.
        let _newborn = heap.alloc(16, k);
        assert_eq!(heap.object_count(), 2);
        while !unsafe { heap.incremental_step(8) } {}
        let stats = unsafe { heap.incremental_finish() };

        assert_eq!(stats.freed, 0, "the mid-mark newborn is retained (born black), not swept");
        assert_eq!(heap.object_count(), 2, "rooted `a` + the newborn both survive");
        // The next cycle (newborn now white + unrooted) reclaims it — proving it was only
        // spared because it was born during the mark, not because it is reachable.
        let root2 = a;
        let next = unsafe { heap.collect_mixed(&[&root2 as *const usize as usize], &[]) };
        assert_eq!(next.freed, 1, "newborn reclaimed on the following cycle");
        assert_eq!(heap.object_count(), 1);
    }

    // ── Incremental collector — the bounded SWEEP (AOT00-T4 §4) ──

    /// Build a heap with `live` rooted survivors (a chain `r → …`) and `garbage` unreachable
    /// objects, mark it to completion incrementally, and return `(heap, root, ())` poised at
    /// the start of the sweep. Every object uses a single-ref-field kind.
    unsafe fn swept_heap(live: usize, garbage: usize) -> (FlatHeap, usize) {
        let mut heap = FlatHeap::new();
        let k = heap.register_kind(&[0]);
        // A chain of `live` reachable objects: root -> n1 -> n2 -> … (all kept).
        let root = heap.alloc(16, k) as usize;
        let mut prev = root;
        for _ in 1..live {
            let next = heap.alloc(16, k) as usize;
            *(prev as *mut usize) = next;
            prev = next;
        }
        // Unreachable garbage, rooted nowhere.
        for _ in 0..garbage {
            let _g = heap.alloc(16, k);
        }
        // Mark to completion so the sweep can run.
        let slots = [&root as *const usize as usize];
        heap.incremental_start(&slots, &[]);
        while !heap.incremental_step(4) {}
        (heap, root)
    }

    /// **A stepped sweep reclaims exactly what a monolithic one does.** Twin heaps of identical
    /// shape (3 live, 5 garbage): one is swept in `budget=1` steps then finished; the other is
    /// finished directly (monolithic sweep). Freed/survived/live counts must match byte-for-byte.
    #[test]
    fn incremental_stepped_sweep_equals_monolithic_sweep() {
        let (mut hs, _rs) = unsafe { swept_heap(3, 5) }; // stepped
        let (mut hm, _rm) = unsafe { swept_heap(3, 5) }; // monolithic

        // Stepped: one block per call until the whole list is swept, then finish.
        assert!(!hs.incremental_sweeping(), "no sweep until the first step");
        let mut steps = 0;
        while !unsafe { hs.incremental_sweep_step(1) } {
            assert!(hs.incremental_sweeping(), "a sweep is in progress mid-drain");
            steps += 1;
            assert!(steps < 100, "must converge");
        }
        let s_stats = unsafe { hs.incremental_finish() };

        // Monolithic: finish with no prior sweep step does the whole sweep at once.
        let m_stats = unsafe { hm.incremental_finish() };

        assert_eq!(s_stats.freed, m_stats.freed, "same garbage reclaimed");
        assert_eq!(s_stats.freed, 5, "all five garbage objects freed");
        assert_eq!(s_stats.survived, m_stats.survived, "same survivor count");
        assert_eq!(hs.object_count(), hm.object_count(), "same live object count");
        assert_eq!(hs.object_count(), 3, "the three-object live chain survives");
        assert_eq!(hs.live_bytes(), hm.live_bytes(), "same live bytes");
        assert!(!hs.incremental_sweeping(), "sweep state cleared at finish");
        assert!(!hs.incremental_in_progress(), "mark phase ended at finish");
    }

    /// **A sweep step is bounded by its budget.** With every step visiting exactly one block
    /// (`budget=1`), the number of steps to completion equals the total block count — the pause
    /// per step is O(budget), not O(heap). 2 live + 6 garbage = 8 blocks ⇒ 8 single-block steps.
    #[test]
    fn incremental_sweep_step_is_bounded() {
        let (mut heap, _root) = unsafe { swept_heap(2, 6) };
        assert_eq!(heap.object_count(), 8, "2 live + 6 garbage");

        let mut steps = 0;
        loop {
            let done = unsafe { heap.incremental_sweep_step(1) };
            steps += 1;
            if done {
                break;
            }
            assert!(steps < 100, "must converge");
        }
        // Exactly one block visited per step ⇒ steps == total blocks (8).
        assert_eq!(steps, 8, "one block visited per budget-1 step across all eight blocks");

        let stats = unsafe { heap.incremental_finish() };
        assert_eq!(stats.freed, 6, "the six garbage objects reclaimed");
        assert_eq!(heap.object_count(), 2, "the two live objects survive");
    }

    /// **Finish drains a partially-stepped sweep.** After only a couple of `sweep_step`s over an
    /// 8-block heap, `finish` must sweep the remaining blocks monolithically so the all-list ends
    /// fully swept — regardless of how far the stepped sweep got.
    #[test]
    fn incremental_finish_drains_partial_sweep() {
        let (mut heap, _root) = unsafe { swept_heap(2, 6) };
        // Take only two small steps — the sweep is nowhere near done.
        let d1 = unsafe { heap.incremental_sweep_step(2) };
        let d2 = unsafe { heap.incremental_sweep_step(2) };
        assert!(!d1 && !d2, "four of eight blocks visited — not finished");
        assert!(heap.incremental_sweeping(), "a partial sweep is outstanding");

        // Finish drains the rest.
        let stats = unsafe { heap.incremental_finish() };
        assert_eq!(stats.freed, 6, "all garbage reclaimed even though only half was stepped");
        assert_eq!(heap.object_count(), 2, "the live chain survives");
        assert!(!heap.incremental_sweeping(), "sweep fully drained at finish");
    }

    /// **An object born mid-sweep survives the cycle** (alloc-black). Allocated between sweep
    /// steps, unrooted, it must not be reclaimed by the running sweep — it is coloured black on
    /// birth (`mark_in_progress` still set), so whether or not the cursor revisits the list head
    /// it is kept. The *following* cycle, where it is white and unrooted, reclaims it.
    #[test]
    fn incremental_newborn_during_sweep_survives() {
        let (mut heap, root) = unsafe { swept_heap(1, 3) }; // 1 live, 3 garbage
        let k = heap.register_kind(&[0]);

        // Start sweeping, then allocate mid-sweep — born black, rooted nowhere.
        let _d = unsafe { heap.incremental_sweep_step(1) };
        assert!(heap.incremental_sweeping());
        let _newborn = heap.alloc(16, k);
        // Drive the sweep to completion and finish.
        while !unsafe { heap.incremental_sweep_step(2) } {}
        let stats = unsafe { heap.incremental_finish() };

        assert_eq!(stats.freed, 3, "only the three original garbage objects are reclaimed");
        assert_eq!(heap.object_count(), 2, "rooted survivor + the mid-sweep newborn both live");

        // Next cycle: the newborn is now white and unrooted ⇒ reclaimed, proving it was spared
        // only because it was born black during the sweep.
        let root2 = root;
        let next = unsafe { heap.collect_mixed(&[&root2 as *const usize as usize], &[]) };
        assert_eq!(next.freed, 1, "the newborn is reclaimed on the following cycle");
        assert_eq!(heap.object_count(), 1, "just the rooted survivor remains");
    }

    // ── Incremental collector — the Dijkstra insertion write barrier (AOT00-T4 PR-2) ──

    /// Build the shared "black parent gains a white child" scenario and return `(heap, P, C)`
    /// with the mutation applied. Objects: `P` (rooted, scanned to BLACK), `Q` (rooted, still
    /// GREY), `C` (a leaf, reachable at start ONLY via `Q.field0`, so still WHITE after `P`'s
    /// scan). The mutation stores `C` into black `P` and drops the `Q → C` edge — exactly the
    /// state where, without a barrier, `C` is stranded white and swept. The caller decides
    /// whether to fire `write_barrier(P, C)`.
    unsafe fn incremental_barrier_setup() -> (FlatHeap, usize, usize) {
        let mut heap = FlatHeap::new();
        let k = heap.register_kind(&[0]); // one ref field at offset 0
        let p = heap.alloc(16, k) as usize; // leaf parent (field0 = 0)
        let q = heap.alloc(16, k) as usize;
        let c = heap.alloc(16, k) as usize; // leaf
        *(q as *mut usize) = c; // Q.field0 -> C  (C reachable only via Q for now)

        // Root Q *then* P so the worklist (a LIFO stack) pops P first.
        let root_q = q;
        let root_p = p;
        let slots = [&root_q as *const usize as usize, &root_p as *const usize as usize];
        heap.incremental_start(&slots, &[]);
        // One step scans P (a leaf) → P BLACK; Q stays grey, C stays white.
        let done = heap.incremental_step(1);
        assert!(!done, "Q still unscanned after one step");

        // Mutate: install C into the black parent P, and drop the Q -> C edge.
        *(p as *mut usize) = c;
        *(q as *mut usize) = 0;
        (heap, p, c)
    }

    /// **The barrier is load-bearing.** With the Dijkstra insertion barrier, storing the white
    /// `C` into the black `P` shades `C` grey, so it survives the cycle — nothing is freed.
    #[test]
    fn incremental_write_barrier_keeps_mutator_installed_child() {
        let (mut heap, p, c) = unsafe { incremental_barrier_setup() };
        // Fire the barrier for the store `P.field0 = C` — shades C grey.
        unsafe { heap.write_barrier(p, c) };
        while !unsafe { heap.incremental_step(8) } {}
        let stats = unsafe { heap.incremental_finish() };
        assert_eq!(stats.freed, 0, "the barrier-shaded child C survives");
        assert_eq!(heap.object_count(), 3, "P, Q, C all live");
        // C is reachable through P and its value is intact.
        assert_eq!(unsafe { *(p as *const usize) }, c, "P.field0 still points at the live C");
    }

    /// **The load-bearing twin.** The *identical* sequence with the barrier call OMITTED: `C`
    /// is never shaded, stays white behind the already-scanned black `P`, and the sweep frees
    /// it — the use-after-free the barrier exists to prevent. Proves the barrier is necessary,
    /// not decorative.
    #[test]
    fn incremental_without_barrier_strands_and_frees_the_child() {
        let (mut heap, _p, _c) = unsafe { incremental_barrier_setup() };
        // NO write_barrier call — the store went straight to memory.
        while !unsafe { heap.incremental_step(8) } {}
        let stats = unsafe { heap.incremental_finish() };
        assert_eq!(stats.freed, 1, "without the barrier, the stranded white C is swept");
        assert_eq!(heap.object_count(), 2, "only P and Q survive — C was lost");
    }

    /// The incremental half of the barrier is inert when no mark is in progress: a store
    /// outside an incremental cycle only does the generational bookkeeping (the worklist stays
    /// empty, nothing is shaded), so the existing generational barrier behaviour is unchanged.
    #[test]
    fn incremental_write_barrier_is_noop_outside_a_mark() {
        let mut heap = FlatHeap::new();
        let k = heap.register_kind(&[0]);
        let p = heap.alloc(16, k) as usize;
        let c = heap.alloc(16, k) as usize;
        assert!(!heap.incremental_in_progress());
        unsafe { heap.write_barrier(p, c) }; // no mark → no shading
        assert_eq!(heap.incremental_grey_count(), 0, "nothing shaded outside a mark");
        // A subsequent full collect rooting only P reclaims the unreferenced C, exactly as
        // before this rung (the barrier didn't spuriously retain anything).
        let root = p;
        let stats = unsafe { heap.collect_mixed(&[&root as *const usize as usize], &[]) };
        assert_eq!(stats.freed, 1, "C reclaimed — the barrier had no incremental effect");
    }
}
