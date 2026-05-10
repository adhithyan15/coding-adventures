//! Specialisation infrastructure — Phase 3 V1 of MX05.
//!
//! This module defines the **shape** of profile-guided specialisation
//! without yet plugging it into the dispatch path:
//!
//! - [`SpecKey`] — the equivalence class identifying which observed
//!   pattern a specialised kernel targets.
//! - [`ShapeClass`] / [`RangeClass`] — the two non-trivial fields of
//!   `SpecKey`, both designed to be `Hash + Eq` so a `SpecKey` can be
//!   a HashMap key.
//! - [`Specialiser`] — the trait backends opt into.  Default
//!   implementation is a no-op so an executor that never specialises
//!   keeps working.
//! - [`SpecCache`] — small LRU keyed by `SpecKey`.  Phase 3 V2 will
//!   wire dispatch routing through this; Phase 3 V1 ships the cache
//!   data structure and proves the key shape is right.
//!
//! ## Why ship the wrapper without the policy
//!
//! The MX05 spec calls out **five components** of the tiered
//! specialisation runtime: profile sampler, specialisation trigger,
//! SpecKey, per-backend specialiser, spec cache + dispatch routing.
//! Phases 1–2a built the sampler; Phase 3 V1 ships components 3, 4,
//! and the cache half of 5.  The remaining piece — the trigger
//! policy that turns a `ProfileObservation` into a `SpecKey` and
//! decides whether to call the backend's specialiser — is Phase 3 V2.
//!
//! Splitting the work this way keeps each PR small enough to review
//! cleanly and lets us validate the data-types interface (which is
//! load-bearing for everything later) before we build the policy on
//! top.
//!
//! ## Locality
//!
//! Spec MX05 plans for an eventual `matrix-profile` crate.  Phase 1
//! and 2a kept the profile sampler inline in `matrix-runtime`; Phase
//! 3 V1 keeps the spec types and cache here for the same reason —
//! avoid premature crate-splitting until we have the full Phase 3
//! plus Phase 4 (constant folding) shape locked in.

use matrix_ir::{DType, Shape};
use std::collections::{HashMap, VecDeque};

// ────────────────────────── SpecKey ──────────────────────────

/// Identifies a specialised kernel.  Two specialisations with the
/// same `SpecKey` are interchangeable; two that differ on any field
/// are not.
///
/// `Hash + Eq` so the key works in a `HashMap`.  Backends that want
/// to disambiguate further can use `backend_id` to encode their own
/// extra dimensions (e.g. f32-with-fast-math vs f32-strict).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct SpecKey {
    /// `matrix_ir::Op::wire_tag()` of the op being specialised.
    pub op_kind: u8,
    /// Dtype the specialised kernel runs on.  May differ from the
    /// declared dtype if Phase 2b's narrowing pass fired and the
    /// observed range fits a tighter type.
    pub dtype: DType,
    /// What the specialiser knows about the input/output shapes.
    pub shape_class: ShapeClass,
    /// What the specialiser knows about the input value range.
    pub range_class: RangeClass,
    /// Which executor this kernel was generated for.  Free-form per
    /// backend (matrix-cpu uses 0; matrix-metal might use 1; a CUDA
    /// backend would use 2; etc.) — `ExecutorId` would be ambiguous
    /// since registry assignment depends on registration order.
    pub backend_id: u32,
}

/// Shape information available at specialisation time.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum ShapeClass {
    /// Exact shape known and stable across observed invocations.
    /// Specialisers that see this can unroll loops, hoist bounds
    /// checks, and pick optimal tiling for the specific dimensions.
    Static(Shape),
    /// Same rank seen on every invocation but at least one dim
    /// varies.  Specialisers can still pick rank-specific code
    /// paths (e.g. a different kernel for rank-2 vs rank-4) without
    /// committing to specific extents.
    StaticRank(u8),
    /// Shape varies entirely.  Use the generic kernel.
    Dynamic,
}

/// Value-range information available at specialisation time.
#[derive(Clone, PartialEq, Hash, Debug)]
pub enum RangeClass {
    /// Floating-point min/max observed.  Encoded as IEEE-754 bits so
    /// the enum can derive `Hash` (raw `f64` is `PartialEq` but not
    /// `Hash` because of NaN).  Use [`Self::float`] / [`Self::range_f32`]
    /// to construct without dealing with bit-encoding directly.
    FloatBits {
        min_bits: u64,
        max_bits: u64,
    },
    /// Integer min/max observed.  Includes both signed and unsigned
    /// — the `i64` range covers `u8` and `i32` losslessly.
    Integer {
        min: i64,
        max: i64,
    },
    /// Constant input — every observed value was the same.  Encoded
    /// as bytes so backends that want to constant-fold can splice
    /// the value directly into emitted code regardless of dtype.
    Constant {
        bytes: Vec<u8>,
    },
    /// Range is unknown or not-yet-narrow-enough to act on.  Use the
    /// generic kernel.
    Unknown,
}

impl Eq for RangeClass {}

impl RangeClass {
    /// Construct a `Float` range from `f64` min/max.  Both ends are
    /// stored as bit-patterns so the enum stays `Hash`able even when
    /// callers pass NaN (which compares unequal to itself); NaN ends
    /// are converted to `Unknown` to keep downstream lookups stable.
    pub fn float(min: f64, max: f64) -> RangeClass {
        if min.is_nan() || max.is_nan() {
            return RangeClass::Unknown;
        }
        RangeClass::FloatBits {
            min_bits: min.to_bits(),
            max_bits: max.to_bits(),
        }
    }

    /// Recover the `f64` range from a `Float` variant.  Returns
    /// `None` for any other variant.
    pub fn as_float(&self) -> Option<(f64, f64)> {
        match *self {
            RangeClass::FloatBits {
                min_bits,
                max_bits,
            } => Some((f64::from_bits(min_bits), f64::from_bits(max_bits))),
            _ => None,
        }
    }
}

// ────────────────────────── Specialiser trait ──────────────────────────

/// Per-backend hook for emitting specialised kernels.
///
/// Phase 3 V1 ships the trait surface but the dispatch path doesn't
/// call it yet — a Specialiser implementation will sit dormant until
/// Phase 3 V2 wires the policy that decides when to specialise.
///
/// ## Default no-op implementation
///
/// Every [`SpecCache`] starts with a default-impl `NoopSpecialiser`
/// installed, so executors that haven't opted in still compile and
/// run correctly.  Backends opt in by replacing it with a custom
/// `impl Specialiser`; we keep the trait object in a `Box<dyn>` so
/// the swap is a runtime call, not a generic-monomorphisation knob.
pub trait Specialiser: Send + Sync {
    /// Generate (or look up an already-emitted) specialised kernel
    /// for `key`.  Returns `None` if this backend cannot or will not
    /// specialise this key — the caller falls back to the generic
    /// kernel for that op.
    ///
    /// Implementations are expected to be **idempotent**: the same
    /// `SpecKey` should produce the same `SpecialisedKernel` (modulo
    /// the opaque `handle`).  The cache uses identity of `SpecKey`,
    /// not deep equality of the returned kernel.
    fn specialise(&self, key: &SpecKey) -> Option<SpecialisedKernel>;
}

/// A specialised kernel ready to be invoked.  The `handle` field is
/// opaque to the runtime — interpretation is up to the backend that
/// emitted it (e.g. matrix-metal's `MetalComputePipelineState` index).
#[derive(Clone, Debug)]
pub struct SpecialisedKernel {
    /// The key this kernel specialises against.
    pub key: SpecKey,
    /// Backend-specific handle.  64 bits is enough for an index into
    /// any reasonable per-executor specialised-kernel table; if a
    /// backend ever needs more it can store an integer key here that
    /// indirects through its own table.
    pub handle: u64,
    /// Short human-readable summary of what this specialisation
    /// does.  Surfaces in `dump()` output and logs.  Not used by
    /// dispatch.
    pub source_summary: String,
}

/// The default no-op Specialiser.  Always returns `None` so dispatch
/// falls back to the generic kernel.  Kept around so an executor
/// without specialisation support can still satisfy the
/// `Box<dyn Specialiser>` slot in [`SpecCache::new`].
#[derive(Default, Debug)]
pub struct NoopSpecialiser;

impl Specialiser for NoopSpecialiser {
    fn specialise(&self, _key: &SpecKey) -> Option<SpecialisedKernel> {
        None
    }
}

// ────────────────────────── SpecCache ──────────────────────────

/// Bounded LRU cache of specialised kernels.
///
/// Phase 3 V1 implements the cache as a `HashMap<SpecKey,
/// SpecialisedKernel>` plus a `VecDeque<SpecKey>` tracking insertion
/// order for eviction.  `get` moves a hit to the back of the deque
/// (most-recently-used).  `insert` evicts from the front when the
/// cache is over capacity.
///
/// Capacity bounds memory: V1 default is 64 entries per backend.
/// Each entry is a `SpecKey` (constant-ish size — depends on
/// `Shape::dims` length) plus a `SpecialisedKernel` (small struct +
/// short summary string).  64 entries × ~256 bytes ≈ 16 KB, well
/// within budget for a long-lived process.
pub struct SpecCache {
    capacity: usize,
    entries: HashMap<SpecKey, SpecialisedKernel>,
    /// LRU ordering — front is least-recently-used, back is
    /// most-recently-used.  We accept the O(n) `retain` cost on
    /// `get` because n ≤ 64 by default and Phase 3 V1 doesn't yet
    /// run dispatch through this code path; Phase 3 V2 may switch
    /// to a doubly-linked list keyed by `SpecKey` if the linear
    /// retain shows up in profiling.
    order: VecDeque<SpecKey>,
}

impl SpecCache {
    /// Construct a fresh cache with the given capacity.  Capacity 0
    /// is allowed and means "never cache anything"; useful for tests
    /// that want to verify the dispatch path still works without a
    /// cache hit.
    pub fn new(capacity: usize) -> Self {
        SpecCache {
            capacity,
            entries: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    /// Construct a cache with the V1 default capacity of 64 entries.
    pub fn default_capacity() -> Self {
        SpecCache::new(64)
    }

    /// Capacity in entries.  Doesn't change after construction in V1;
    /// resizing live caches is V2 work.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Number of entries currently in the cache.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True iff the cache has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Look up a specialised kernel.  On hit, marks it as
    /// most-recently-used (moves it to the back of the LRU deque).
    /// Returns `None` on miss.
    pub fn get(&mut self, key: &SpecKey) -> Option<SpecialisedKernel> {
        if let Some(entry) = self.entries.get(key) {
            // Touch LRU bookkeeping.  O(n) retain is fine at the V1
            // capacities we ship (≤ 64); Phase 3 V2 can revisit if
            // profiling shows it as a hotspot.
            self.order.retain(|k| k != key);
            self.order.push_back(key.clone());
            Some(entry.clone())
        } else {
            None
        }
    }

    /// Insert a specialised kernel.  Evicts the LRU entry if the
    /// cache is at capacity.  No-op if `capacity == 0` (the kernel
    /// is not stored).
    pub fn insert(&mut self, kernel: SpecialisedKernel) {
        if self.capacity == 0 {
            return;
        }
        let key = kernel.key.clone();
        if self.entries.contains_key(&key) {
            // Update existing entry; touch LRU.
            self.entries.insert(key.clone(), kernel);
            self.order.retain(|k| k != &key);
            self.order.push_back(key);
            return;
        }
        // Evict LRU until under capacity.
        while self.entries.len() >= self.capacity {
            if let Some(oldest) = self.order.pop_front() {
                self.entries.remove(&oldest);
            } else {
                break;
            }
        }
        self.order.push_back(key.clone());
        self.entries.insert(key, kernel);
    }

    /// Drop all cached kernels.  Useful when a backend-level event
    /// (driver reset, recompilation) invalidates everything.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
    }
}

impl Default for SpecCache {
    fn default() -> Self {
        SpecCache::default_capacity()
    }
}

// ────────────────────────── Tests ──────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use matrix_ir::{DType, Shape};

    fn key(op_kind: u8, dtype: DType) -> SpecKey {
        SpecKey {
            op_kind,
            dtype,
            shape_class: ShapeClass::Dynamic,
            range_class: RangeClass::Unknown,
            backend_id: 0,
        }
    }

    fn kernel(key: SpecKey, handle: u64, summary: &str) -> SpecialisedKernel {
        SpecialisedKernel {
            key,
            handle,
            source_summary: summary.to_string(),
        }
    }

    #[test]
    fn spec_key_equality_on_all_fields() {
        let a = key(0x07, DType::F32);
        let b = key(0x07, DType::F32);
        assert_eq!(a, b);

        let mut c = key(0x07, DType::F32);
        c.backend_id = 1;
        assert_ne!(a, c);

        let mut d = key(0x07, DType::F32);
        d.shape_class = ShapeClass::Static(Shape::from(&[4, 4]));
        assert_ne!(a, d);
    }

    #[test]
    fn shape_class_static_is_hashable() {
        use std::collections::HashSet;
        let mut s = HashSet::new();
        s.insert(ShapeClass::Static(Shape::from(&[4, 4])));
        s.insert(ShapeClass::Static(Shape::from(&[8, 8])));
        s.insert(ShapeClass::StaticRank(2));
        s.insert(ShapeClass::Dynamic);
        assert_eq!(s.len(), 4);
    }

    #[test]
    fn range_class_float_round_trip() {
        let r = RangeClass::float(-1.0, 1.0);
        let (lo, hi) = r.as_float().unwrap();
        assert_eq!(lo, -1.0);
        assert_eq!(hi, 1.0);
    }

    #[test]
    fn range_class_float_with_nan_collapses_to_unknown() {
        let r = RangeClass::float(f64::NAN, 1.0);
        assert!(matches!(r, RangeClass::Unknown));
        let r = RangeClass::float(0.0, f64::NAN);
        assert!(matches!(r, RangeClass::Unknown));
    }

    #[test]
    fn range_class_constant_is_hashable() {
        use std::collections::HashSet;
        let mut s = HashSet::new();
        s.insert(RangeClass::Constant {
            bytes: vec![1, 2, 3, 4],
        });
        s.insert(RangeClass::Constant {
            bytes: vec![5, 6, 7, 8],
        });
        // Same bytes should not double-insert.
        s.insert(RangeClass::Constant {
            bytes: vec![1, 2, 3, 4],
        });
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn noop_specialiser_returns_none() {
        let s = NoopSpecialiser;
        let k = key(0x07, DType::F32);
        assert!(s.specialise(&k).is_none());
    }

    #[test]
    fn cache_insert_and_get_round_trip() {
        let mut c = SpecCache::default_capacity();
        let k = key(0x07, DType::F32);
        c.insert(kernel(k.clone(), 42, "neg_f32_static_4x4"));
        let got = c.get(&k).unwrap();
        assert_eq!(got.handle, 42);
        assert_eq!(got.source_summary, "neg_f32_static_4x4");
    }

    #[test]
    fn cache_get_miss_returns_none() {
        let mut c = SpecCache::default_capacity();
        let k = key(0x07, DType::F32);
        assert!(c.get(&k).is_none());
    }

    #[test]
    fn cache_evicts_lru_when_full() {
        let mut c = SpecCache::new(2);
        let k1 = key(0x00, DType::F32);
        let k2 = key(0x01, DType::F32);
        let k3 = key(0x02, DType::F32);
        c.insert(kernel(k1.clone(), 1, "neg"));
        c.insert(kernel(k2.clone(), 2, "abs"));
        // Touch k1 — it becomes MRU.
        let _ = c.get(&k1);
        // Insert k3 — should evict k2 (the LRU), not k1.
        c.insert(kernel(k3.clone(), 3, "sqrt"));
        assert!(c.get(&k1).is_some());
        assert!(c.get(&k2).is_none(), "k2 should have been evicted");
        assert!(c.get(&k3).is_some());
        assert_eq!(c.len(), 2);
    }

    #[test]
    fn cache_capacity_zero_does_not_store() {
        let mut c = SpecCache::new(0);
        let k = key(0x07, DType::F32);
        c.insert(kernel(k.clone(), 1, "noop"));
        assert!(c.is_empty());
        assert!(c.get(&k).is_none());
    }

    #[test]
    fn cache_re_insert_updates_value_and_touches_lru() {
        let mut c = SpecCache::new(2);
        let k1 = key(0x00, DType::F32);
        let k2 = key(0x01, DType::F32);
        c.insert(kernel(k1.clone(), 1, "v1"));
        c.insert(kernel(k2.clone(), 2, "v2"));
        // Re-insert k1 — should update handle and touch LRU.
        c.insert(kernel(k1.clone(), 99, "v1.1"));
        let got = c.get(&k1).unwrap();
        assert_eq!(got.handle, 99);
        assert_eq!(got.source_summary, "v1.1");
        // Now insert k3 — k2 should be evicted (LRU), not k1.
        let k3 = key(0x02, DType::F32);
        c.insert(kernel(k3.clone(), 3, "v3"));
        assert!(c.get(&k2).is_none());
        assert!(c.get(&k1).is_some());
    }

    #[test]
    fn cache_clear_drops_everything() {
        let mut c = SpecCache::new(4);
        for i in 0..4 {
            c.insert(kernel(key(i as u8, DType::F32), i as u64, "k"));
        }
        assert_eq!(c.len(), 4);
        c.clear();
        assert!(c.is_empty());
    }

    /// Sanity: a cache with 64 entries holds them all and then evicts
    /// the right one when we overflow.  Captures the V1 default
    /// capacity behaviour.
    #[test]
    fn cache_at_default_capacity_evicts_in_lru_order() {
        let mut c = SpecCache::default_capacity();
        // Fill to capacity.
        for i in 0..64u8 {
            c.insert(kernel(key(i, DType::F32), i as u64, ""));
        }
        assert_eq!(c.len(), 64);
        // Insert one more — capacity 64 keys + 1 = 65 inserts; oldest (0x00) evicts.
        c.insert(kernel(key(64, DType::F32), 64, ""));
        assert_eq!(c.len(), 64);
        assert!(c.get(&key(0, DType::F32)).is_none());
        assert!(c.get(&key(64, DType::F32)).is_some());
    }
}
