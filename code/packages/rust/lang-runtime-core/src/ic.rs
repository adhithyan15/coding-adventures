//! # Inline-cache machinery (generic over per-language entry shape).
//!
//! Inline caches are V8's secret weapon: each call site / load site
//! / store site has a small per-site cache of *observed shape →
//! resolved target* tuples.  The interpreter records observations,
//! the JIT emits a compare-and-jump against the cached entry, and
//! a hot polymorphic site collapses to a few instructions instead
//! of a hash-table lookup.
//!
//! LANG20 generalises this: every language picks its own
//! [`crate::LangBinding::ICEntry`] shape (V8 hidden-class id, Ruby
//! class+version, Smalltalk receiver class, Lispy type tag) and
//! `lang-runtime-core` provides the storage + state machine
//! infrastructure that surrounds it.
//!
//! ## State machine
//!
//! ```text
//! ┌─────────┐   first observation    ┌──────────────┐
//! │ Uninit  │ ─────────────────────► │ Monomorphic  │
//! └─────────┘                        └──────┬───────┘
//!                                           │ second distinct shape
//!                                           ▼
//!                                  ┌──────────────────┐  Kth+1 distinct shape
//!                                  │  Polymorphic     │ ───────────────────────┐
//!                                  │  (≤K entries)    │                        │
//!                                  └──────────────────┘                        ▼
//!                                                                  ┌──────────────────┐
//!                                                                  │   Megamorphic    │
//!                                                                  │   (terminal)     │
//!                                                                  └──────────────────┘
//! ```
//!
//! Once megamorphic, the IC stops trying to specialise: every call
//! falls through to the runtime's generic dispatch
//! ([`crate::LangBinding::send_message`] or `load_property` etc.).
//! This bounds the worst case at one branch + one runtime call —
//! the same cost as the slow path, but with no IC bookkeeping
//! overhead.
//!
//! ## Why a fixed `MAX_PIC_ENTRIES`?
//!
//! V8 settled on 4 after years of telemetry; SpiderMonkey uses 4–8
//! depending on site kind.  Four is the LANG20 default because:
//!
//! - **Cache footprint matters.**  An IC table is consulted every
//!   call to a polymorphic site; keeping it ≤ one cache line means
//!   the hot loop fits in L1.
//! - **Beyond 4 shapes, megamorphic dispatch is cheaper than
//!   chasing a longer dispatch table.**  The marginal hit rate
//!   from entries 5–8 is tiny in real workloads (V8/Spider data).
//! - **Codegen is simpler.**  The JIT emits a 4-way unrolled
//!   compare; longer chains need a loop or a switch.
//!
//! The constant is `pub` so language-specific tunings can read it
//! (e.g. a language with an unusual dispatch model can mirror its
//! shape table size).
//!
//! ## Why generic over `E`?
//!
//! Each language's IC entry layout differs (LANG20 §"Inline cache
//! machinery"):
//!
//! | Language | `ICEntry` shape |
//! |----------|-----------------|
//! | Lispy | `(type_tag: u32, handler: fn ptr)` |
//! | JavaScript | `(hidden_class_id: u32, offset_or_method: u32)` |
//! | Smalltalk PIC | `(receiver_class: u32, method_addr: usize)` |
//! | Ruby | `(receiver_class: u32, method_version: u16, target: usize)` |
//! | Perl | `(package_id: u32, sub_addr: usize)` |
//!
//! All five fit ≤ 16 bytes and all five emit `compare-and-jump`
//! sequences — that uniformity is why one IC infrastructure serves
//! everyone.

// ---------------------------------------------------------------------------
// MAX_PIC_ENTRIES
// ---------------------------------------------------------------------------

/// Maximum entries before an IC transitions to `Megamorphic`.
///
/// V8/SpiderMonkey converged on 4 after years of telemetry.  See
/// module-level docs for the rationale.
pub const MAX_PIC_ENTRIES: usize = 4;

// ---------------------------------------------------------------------------
// ICState
// ---------------------------------------------------------------------------

/// Lifecycle state of an [`InlineCache`].  Ordered by specialisation
/// quality: `Uninit` < `Monomorphic` < `Polymorphic` < `Megamorphic`.
///
/// The state never moves backwards on its own — only invalidation
/// (class redefinition, `LangBinding::invalidate_ics`) resets a
/// cache to `Uninit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ICState {
    /// No observations recorded yet; the JIT emits a generic
    /// dispatch with a callback that warms the cache on first call.
    Uninit,

    /// Exactly one shape observed.  The JIT emits the fastest
    /// path: a single compare-and-jump against the cached entry.
    Monomorphic,

    /// 2..=`MAX_PIC_ENTRIES` shapes observed.  The JIT emits a
    /// linear chain of compares (or a small switch).
    Polymorphic,

    /// More than `MAX_PIC_ENTRIES` shapes observed.  The JIT
    /// gives up specialisation and emits a generic runtime
    /// dispatch.  Terminal — once mega, the cache stays mega.
    Megamorphic,
}

impl ICState {
    /// `true` if the IC has at least one observed shape and could
    /// support a fast-path dispatch.
    pub fn is_warm(self) -> bool {
        matches!(self, ICState::Monomorphic | ICState::Polymorphic)
    }
}

// ---------------------------------------------------------------------------
// InlineCache<E>
// ---------------------------------------------------------------------------

/// Per-call-site cache of recently observed shapes and their
/// resolved targets.
///
/// `E` is the per-language entry type (see [`crate::LangBinding::ICEntry`]).
/// The cache stores up to [`MAX_PIC_ENTRIES`] entries; beyond that it
/// transitions to [`ICState::Megamorphic`] and stops caching.
///
/// # Storage
///
/// Entries live in a fixed-size `[Option<E>; MAX_PIC_ENTRIES]`
/// array (no heap allocation).  This keeps IC reads to a single
/// cache line and lets the JIT generate inline checks against
/// known offsets.
///
/// # Lifecycle
///
/// 1. Construct via [`InlineCache::new`] — state is [`ICState::Uninit`].
/// 2. The interpreter (or JIT slow path) observes a shape, calls
///    [`InlineCache::record`].
/// 3. Future lookups walk `entries` via the binding's
///    matcher — the IC infra doesn't know how to match because the
///    entry shape is per-language.
///
/// # Invalidation
///
/// When a class is redefined (Ruby reopens, JS prototype mutation,
/// Smalltalk `become:`), affected ICs reset to `Uninit` via
/// [`InlineCache::invalidate`].  The runtime's dispatcher discovers
/// this on the next miss and re-warms the cache.
///
/// # Field privacy
///
/// Fields are private — external callers go through the API so the
/// state machine's invariants can't be desynchronised by direct
/// writes (e.g. `state = Monomorphic` with `entries == [None; 4]`
/// would crash a JIT fast path that trusts the state).
///
/// JIT-emitted code that needs to read these fields at known
/// offsets uses `#[repr(C)]` (declared on this struct) plus a
/// codegen-time `offset_of!` — privacy doesn't restrict the JIT
/// because it doesn't go through Rust's visibility rules.
#[derive(Debug, Clone)]
#[repr(C)]
pub struct InlineCache<E: Copy> {
    // Order matters for the `#[repr(C)]` ABI commitment: entries
    // first (largest), then state, then counters.  JIT codegen
    // bakes these offsets in.
    entries: [Option<E>; MAX_PIC_ENTRIES],
    state: ICState,
    hit_count: u32,
    miss_count: u32,
}

impl<E: Copy> InlineCache<E> {
    /// Construct a fresh IC in [`ICState::Uninit`].
    pub const fn new() -> Self {
        InlineCache {
            entries: [None; MAX_PIC_ENTRIES],
            state: ICState::Uninit,
            hit_count: 0,
            miss_count: 0,
        }
    }

    /// Read-only view of the stored entries.  `None` slots are unused.
    ///
    /// JIT codegen that needs random-access reads of specific entries
    /// uses `#[repr(C)]` + `offset_of!` instead of going through this
    /// accessor — Rust's borrow checker would otherwise serialise
    /// reads that the hardware doesn't need to.
    pub fn entries(&self) -> &[Option<E>; MAX_PIC_ENTRIES] {
        &self.entries
    }

    /// Lifecycle state — `Uninit` until the first observation.
    pub fn state(&self) -> ICState {
        self.state
    }

    /// Hits since last invalidation.  Used by `jit-profiling-insights`
    /// (LANG11) to report site temperature.
    pub fn hit_count(&self) -> u32 {
        self.hit_count
    }

    /// Misses since last invalidation.  A high miss count on a
    /// cold IC is a re-promotion signal for the JIT.
    pub fn miss_count(&self) -> u32 {
        self.miss_count
    }

    /// Number of entries currently stored.
    pub fn len(&self) -> usize {
        self.entries.iter().filter(|e| e.is_some()).count()
    }

    /// `true` if no entries are stored.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Record a new observation; advance the state machine.
    ///
    /// Returns `true` if the entry was installed, `false` if the
    /// cache is megamorphic (entry dropped).
    ///
    /// The caller is responsible for checking that the new entry's
    /// shape isn't already in `entries` (the IC infrastructure
    /// can't compare entries because their layout is per-language).
    pub fn record(&mut self, entry: E) -> bool {
        if matches!(self.state, ICState::Megamorphic) {
            return false;
        }
        // Find the first empty slot.
        for slot in self.entries.iter_mut() {
            if slot.is_none() {
                *slot = Some(entry);
                self.state = match self.len() {
                    1 => ICState::Monomorphic,
                    n if n <= MAX_PIC_ENTRIES => ICState::Polymorphic,
                    _ => unreachable!(),
                };
                return true;
            }
        }
        // No empty slot — transition to mega.
        self.state = ICState::Megamorphic;
        // Optionally clear entries to free memory; for now keep them
        // for inspectability (`jit-profiling-insights` may want them).
        false
    }

    /// Reset the cache to [`ICState::Uninit`].  Called on class
    /// redefinition or other invalidation events.
    pub fn invalidate(&mut self) {
        self.entries = [None; MAX_PIC_ENTRIES];
        self.state = ICState::Uninit;
        // Counters reset too — the next interval starts fresh.
        self.hit_count = 0;
        self.miss_count = 0;
    }

    /// Note a cache hit.  Cheap; called on every fast-path dispatch.
    #[inline]
    pub fn note_hit(&mut self) {
        self.hit_count = self.hit_count.saturating_add(1);
    }

    /// Note a cache miss.  Called when the runtime falls through to
    /// the generic dispatch path.
    #[inline]
    pub fn note_miss(&mut self) {
        self.miss_count = self.miss_count.saturating_add(1);
    }
}

impl<E: Copy> Default for InlineCache<E> {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// ICId
// ---------------------------------------------------------------------------

/// Compile-time-assigned identifier for a single inline cache.
///
/// Each IIRFunction has its own ID space starting at 0.  The runtime
/// allocates IC storage at function-load time based on the highest
/// id seen.
///
/// `IIRInstr::ic_slot` (LANG20 §"IIR additions") carries
/// `Option<ICId>`: `None` for instructions without ICs (arithmetic,
/// control flow), `Some(id)` for `send` / `load_property` /
/// `store_property`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
pub struct ICId(pub u32);

// Compile-time size commitments — caught at compile time so a
// downstream re-export pulled in by mistake can't grow the type.
const _: () = assert!(std::mem::size_of::<ICId>() == 4);

impl ICId {
    /// Return the underlying `u32` for ABI passing.
    pub const fn as_u32(self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for ICId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ic#{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// ClassId
// ---------------------------------------------------------------------------

/// Per-language opaque class identifier used as IC keys.
///
/// Distinct from [`crate::LangBinding::ClassRef`] (which is the
/// binding's own type for class identity); `ClassId` is the
/// universal `u32` view used in IC entries and invalidation calls.
/// Bindings convert their `ClassRef` to/from `ClassId` themselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
pub struct ClassId(pub u32);

const _: () = assert!(std::mem::size_of::<ClassId>() == 4);

impl ClassId {
    /// Return the underlying `u32` for ABI passing.
    pub const fn as_u32(self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for ClassId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cls#{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// ICInvalidator
// ---------------------------------------------------------------------------

/// Callback the runtime hands to a binding's
/// `LangBinding::invalidate_ics` so the binding can request the
/// runtime to invalidate specific caches or all caches keyed on a
/// class.
///
/// The runtime owns the IC side-table; the binding doesn't have
/// direct access.  This trait is the bridge.
pub trait ICInvalidator {
    /// Reset a specific IC to `Uninit`.
    fn invalidate_ic(&mut self, ic: ICId);

    /// Reset every IC keyed on `class` to `Uninit`.  The runtime's
    /// implementation walks its IC side-table and invalidates each
    /// IC whose entries reference the class.
    fn invalidate_class(&mut self, class: ClassId);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// A tiny entry shape for tests — pretend we're Lispy
    /// (type_tag, handler).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct LispyICEntry {
        type_tag: u32,
        handler: usize,
    }

    fn entry(t: u32, h: usize) -> LispyICEntry {
        LispyICEntry { type_tag: t, handler: h }
    }

    #[test]
    fn fresh_cache_is_uninit_and_empty() {
        let ic: InlineCache<LispyICEntry> = InlineCache::new();
        assert_eq!(ic.state(), ICState::Uninit);
        assert!(ic.is_empty());
        assert_eq!(ic.len(), 0);
    }

    #[test]
    fn first_record_advances_to_monomorphic() {
        let mut ic = InlineCache::new();
        assert!(ic.record(entry(1, 0xAAA)));
        assert_eq!(ic.state(), ICState::Monomorphic);
        assert_eq!(ic.len(), 1);
    }

    #[test]
    fn second_distinct_record_advances_to_polymorphic() {
        let mut ic = InlineCache::new();
        ic.record(entry(1, 0xAAA));
        assert!(ic.record(entry(2, 0xBBB)));
        assert_eq!(ic.state(), ICState::Polymorphic);
        assert_eq!(ic.len(), 2);
    }

    #[test]
    fn fills_to_max_then_one_more_promotes_to_mega() {
        let mut ic = InlineCache::new();
        for i in 0..MAX_PIC_ENTRIES {
            assert!(ic.record(entry(i as u32, i)));
        }
        assert_eq!(ic.state(), ICState::Polymorphic);
        assert_eq!(ic.len(), MAX_PIC_ENTRIES);
        // One more push hits the mega path:
        assert!(!ic.record(entry(99, 0)));
        assert_eq!(ic.state(), ICState::Megamorphic);
    }

    #[test]
    fn megamorphic_cache_drops_further_records() {
        let mut ic = InlineCache::new();
        for i in 0..=MAX_PIC_ENTRIES {
            ic.record(entry(i as u32, i));
        }
        assert_eq!(ic.state(), ICState::Megamorphic);
        assert!(!ic.record(entry(100, 0)));
        assert_eq!(ic.state(), ICState::Megamorphic);
    }

    #[test]
    fn invalidate_resets_state_and_entries() {
        let mut ic = InlineCache::new();
        ic.record(entry(1, 0xAAA));
        ic.note_hit();
        ic.note_hit();
        ic.invalidate();
        assert_eq!(ic.state(), ICState::Uninit);
        assert!(ic.is_empty());
        assert_eq!(ic.hit_count, 0);
    }

    #[test]
    fn note_hit_and_miss_saturate() {
        let mut ic: InlineCache<LispyICEntry> = InlineCache::new();
        ic.hit_count = u32::MAX;
        ic.note_hit();
        assert_eq!(ic.hit_count, u32::MAX, "saturating add should not overflow");
        ic.miss_count = u32::MAX;
        ic.note_miss();
        assert_eq!(ic.miss_count, u32::MAX);
    }

    #[test]
    fn ic_state_is_warm_for_mono_and_poly() {
        assert!(!ICState::Uninit.is_warm());
        assert!(ICState::Monomorphic.is_warm());
        assert!(ICState::Polymorphic.is_warm());
        assert!(!ICState::Megamorphic.is_warm());
    }

    #[test]
    fn ic_id_and_class_id_display() {
        assert_eq!(format!("{}", ICId(7)), "ic#7");
        assert_eq!(format!("{}", ClassId(42)), "cls#42");
    }

    #[test]
    fn ic_id_is_repr_transparent_size_4() {
        assert_eq!(std::mem::size_of::<ICId>(), 4);
        assert_eq!(std::mem::size_of::<ClassId>(), 4);
    }

    /// Test invalidator records calls instead of touching real state.
    struct RecordingInvalidator {
        invalidated_ics: Vec<ICId>,
        invalidated_classes: Vec<ClassId>,
    }

    impl ICInvalidator for RecordingInvalidator {
        fn invalidate_ic(&mut self, ic: ICId) {
            self.invalidated_ics.push(ic);
        }
        fn invalidate_class(&mut self, class: ClassId) {
            self.invalidated_classes.push(class);
        }
    }

    #[test]
    fn ic_invalidator_is_object_safe() {
        let mut inv = RecordingInvalidator {
            invalidated_ics: Vec::new(),
            invalidated_classes: Vec::new(),
        };
        let dyn_inv: &mut dyn ICInvalidator = &mut inv;
        dyn_inv.invalidate_ic(ICId(3));
        dyn_inv.invalidate_class(ClassId(7));
        assert_eq!(inv.invalidated_ics, vec![ICId(3)]);
        assert_eq!(inv.invalidated_classes, vec![ClassId(7)]);
    }

    #[test]
    fn cache_default_is_uninit() {
        let ic: InlineCache<LispyICEntry> = Default::default();
        assert_eq!(ic.state(), ICState::Uninit);
    }

    #[test]
    fn cache_is_clone() {
        let mut ic: InlineCache<LispyICEntry> = InlineCache::new();
        ic.record(entry(1, 0xAAA));
        let copy = ic.clone();
        assert_eq!(copy.state, ic.state);
        assert_eq!(copy.entries[0], ic.entries[0]);
    }
}
