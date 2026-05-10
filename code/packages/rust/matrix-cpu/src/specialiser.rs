//! `CpuSpecialiser` — first real backend `Specialiser` implementation.
//!
//! MX05 Phase 4 (minimum-viable scope).  Up to this point the only
//! `Specialiser` impl in the workspace was `NoopSpecialiser`, which
//! declined every key.  This module ships the first real backend
//! that says "yes" to specialisation requests and emits a
//! [`matrix_profile::SpecialisedKernel`] for any [`matrix_profile::SpecKey`]
//! the policy hands it.
//!
//! ## What "specialised" means in V1
//!
//! Phase 4's *minimum viable* scope is **observation parity, not
//! execution speedup**.  The kernel handle that `CpuSpecialiser`
//! emits is opaque to the runtime — the dispatch path doesn't yet
//! consume it (that needs an executor-protocol extension to add
//! something like `ExecutorRequest::DispatchSpecialised { key, .. }`,
//! which is V2 work).  But emitting the handle proves the wiring is
//! live: under a `SpecRouter` configured with this specialiser plus
//! a low policy threshold, hot graphs visibly populate the
//! `SpecCache`, which is the contract Phase 4 promised.
//!
//! Future phases turn the handle into an actual specialised
//! evaluator:
//!
//! - **Phase 4.1**: emit a closure that takes pre-uploaded inputs
//!   and writes outputs (matrix-cpu can store these in a per-handle
//!   `Vec<Box<dyn Fn>>` and dispatch to them via a new
//!   `ExecutorRequest::DispatchSpecialised`).
//! - **Phase 4.2**: matrix-metal emits an MSL string per `SpecKey`
//!   with constants folded in, then compiles to a
//!   `MetalComputePipelineState` cached by handle.
//! - **Phase 5**: deoptimisation when an observed assumption fails.
//!
//! ## Determinism
//!
//! For a given `SpecKey`, `specialise()` always returns a kernel
//! with the same handle (a deterministic hash of the key).  This
//! lets tests assert on handle values without taking a dependency
//! on call order.

use matrix_profile::{SpecKey, SpecialisedKernel, Specialiser};

/// CPU backend specialiser.  Ships in matrix-cpu as the first real
/// `Specialiser` impl; in V1 it emits opaque handles only — Phase
/// 4.1 wires the handles into a per-backend kernel table that the
/// dispatch path can invoke.
///
/// `Send + Sync` so it can sit behind a `Box<dyn Specialiser>`
/// (which the trait requires) and be shared across the runtime's
/// dispatch threads.
#[derive(Default, Debug)]
pub struct CpuSpecialiser;

impl CpuSpecialiser {
    /// Construct a fresh `CpuSpecialiser`.
    pub fn new() -> Self {
        CpuSpecialiser
    }
}

impl Specialiser for CpuSpecialiser {
    fn specialise(&self, key: &SpecKey) -> Option<SpecialisedKernel> {
        let handle = handle_for_key(key);
        Some(SpecialisedKernel {
            key: key.clone(),
            handle,
            source_summary: format!(
                "matrix-cpu specialiser: op_kind=0x{:02X}, dtype={:?}, backend={}",
                key.op_kind, key.dtype, key.backend_id
            ),
        })
    }
}

/// Convenience: returns a `Box<dyn Specialiser>` ready to plug into a
/// `SpecRouter::new(...)` call.  Makes the common construction path
/// `SpecRouter::new(policy, cache, matrix_cpu::specialiser())` work
/// without the caller writing the `Box::new(...)` themselves.
pub fn specialiser() -> Box<dyn Specialiser> {
    Box::new(CpuSpecialiser::new())
}

// ────────────────────────── deterministic handles ──────────────────────────

/// Produce a deterministic 64-bit handle for a given `SpecKey`.
/// FNV-1a over a stable byte serialisation — same approach
/// `matrix_profile::Profiler::subhash` uses, kept here so the two
/// hashes are independent and can evolve separately.
fn handle_for_key(key: &SpecKey) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

    fn feed_byte(b: u8, h: &mut u64) {
        *h ^= b as u64;
        *h = h.wrapping_mul(FNV_PRIME);
    }
    fn feed_le_u32(v: u32, h: &mut u64) {
        for b in v.to_le_bytes() {
            feed_byte(b, h);
        }
    }

    let mut h = FNV_OFFSET;
    feed_byte(key.op_kind, &mut h);
    feed_byte(key.dtype.wire_tag(), &mut h);
    feed_le_u32(key.backend_id, &mut h);

    // ShapeClass discriminator + payload.
    use matrix_profile::ShapeClass;
    match &key.shape_class {
        ShapeClass::Static(shape) => {
            feed_byte(0x00, &mut h);
            feed_le_u32(shape.dims.len() as u32, &mut h);
            for d in &shape.dims {
                feed_le_u32(*d, &mut h);
            }
        }
        ShapeClass::StaticRank(r) => {
            feed_byte(0x01, &mut h);
            feed_byte(*r, &mut h);
        }
        ShapeClass::Dynamic => {
            feed_byte(0x02, &mut h);
        }
    }

    // RangeClass discriminator + payload.
    use matrix_profile::RangeClass;
    match &key.range_class {
        RangeClass::FloatBits { min_bits, max_bits } => {
            feed_byte(0x00, &mut h);
            for b in min_bits.to_le_bytes() {
                feed_byte(b, &mut h);
            }
            for b in max_bits.to_le_bytes() {
                feed_byte(b, &mut h);
            }
        }
        RangeClass::Integer { min, max } => {
            feed_byte(0x01, &mut h);
            for b in min.to_le_bytes() {
                feed_byte(b, &mut h);
            }
            for b in max.to_le_bytes() {
                feed_byte(b, &mut h);
            }
        }
        RangeClass::Constant { bytes } => {
            feed_byte(0x02, &mut h);
            feed_le_u32(bytes.len() as u32, &mut h);
            for &b in bytes {
                feed_byte(b, &mut h);
            }
        }
        RangeClass::Unknown => {
            feed_byte(0x03, &mut h);
        }
    }

    h
}

// ────────────────────────── tests ──────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use matrix_ir::{DType, Shape};
    use matrix_profile::{
        DefaultPolicy, ProfileObservation, RangeClass, ShapeClass, SpecCache, SpecRouter,
        SpecialisationPolicy, TensorObservation,
    };

    fn key(op_kind: u8) -> SpecKey {
        SpecKey {
            op_kind,
            dtype: DType::F32,
            shape_class: ShapeClass::Dynamic,
            range_class: RangeClass::Unknown,
            backend_id: 0,
        }
    }

    #[test]
    fn specialise_emits_kernel_for_any_key() {
        let s = CpuSpecialiser::new();
        let k = s.specialise(&key(0x07)).unwrap();
        assert_eq!(k.key.op_kind, 0x07);
        assert_eq!(k.key.dtype, DType::F32);
        // Source summary is human-readable; just check it mentions the op kind.
        assert!(k.source_summary.contains("0x07"));
    }

    #[test]
    fn handles_are_deterministic_for_same_key() {
        let s = CpuSpecialiser::new();
        let k1 = s.specialise(&key(0x07)).unwrap();
        let k2 = s.specialise(&key(0x07)).unwrap();
        assert_eq!(k1.handle, k2.handle);
    }

    #[test]
    fn handles_differ_for_distinct_keys() {
        let s = CpuSpecialiser::new();
        let k1 = s.specialise(&key(0x07)).unwrap();
        let k2 = s.specialise(&key(0x08)).unwrap();
        assert_ne!(k1.handle, k2.handle);
    }

    #[test]
    fn handle_is_sensitive_to_shape_class() {
        let s = CpuSpecialiser::new();
        let mut a = key(0x07);
        a.shape_class = ShapeClass::Static(Shape::from(&[4, 4]));
        let mut b = key(0x07);
        b.shape_class = ShapeClass::Static(Shape::from(&[8, 8]));
        let ka = s.specialise(&a).unwrap();
        let kb = s.specialise(&b).unwrap();
        assert_ne!(ka.handle, kb.handle);
    }

    #[test]
    fn handle_is_sensitive_to_constant_bytes() {
        let s = CpuSpecialiser::new();
        let mut a = key(0x07);
        a.range_class = RangeClass::Constant {
            bytes: vec![1, 2, 3, 4],
        };
        let mut b = key(0x07);
        b.range_class = RangeClass::Constant {
            bytes: vec![5, 6, 7, 8],
        };
        let ka = s.specialise(&a).unwrap();
        let kb = s.specialise(&b).unwrap();
        assert_ne!(ka.handle, kb.handle);
    }

    #[test]
    fn specialiser_function_returns_box_dyn() {
        // Smoke-test the public convenience function.
        let _b: Box<dyn Specialiser> = specialiser();
    }

    /// **End-to-end Phase 4 integration test**: wire CpuSpecialiser
    /// into a SpecRouter under a low-threshold policy, drive enough
    /// observations to fire, and confirm the cache fills.
    ///
    /// Up to V3, every Phase test under `NoopSpecialiser` saw
    /// `cache.len() == 0`.  This is the first test where the cache
    /// rises above zero — the spec MX05 promise that "Phase 4 will
    /// see spec_cache_len rise" cashed in.
    #[test]
    fn router_with_cpu_specialiser_populates_cache_when_policy_fires() {
        let mut router = SpecRouter::new(
            // Threshold lowered to 1 so every observation past the
            // first invocation_count crosses; default 1000 would need
            // a thousand calls in a unit test.
            Box::new(DefaultPolicy::with_thresholds(1, 0.95)),
            SpecCache::default_capacity(),
            specialiser(),
        );
        // Small detour: SpecRouter takes ownership; we want a
        // borrow-style smoke test, so re-bind `mut` even though
        // we'll only call `&self` methods.
        let _ = &mut router;

        // Build a hot observation: one input slot with a stable
        // constant value, samples × 1.0 above the stability ratio.
        let observation = ProfileObservation {
            graph_subhash: 0x1234,
            op_index: 0,
            invocation_count: 5,
            last_executor: compute_ir::ExecutorId(0),
            tensor_observations: vec![TensorObservation {
                slot: 0,
                is_input: true,
                observed_min: 7.0,
                observed_max: 7.0,
                observed_zeros: 0,
                samples: 5,
            }],
        };

        let r = router.route(&observation, 0x07, DType::F32, 0);
        assert!(
            r.is_some(),
            "expected the router to specialise under DefaultPolicy(1, 0.95) + CpuSpecialiser"
        );
        assert_eq!(router.cache_len(), 1, "cache should hold the new kernel");

        // Second call with the same observation: cache hit, specialiser
        // not invoked again, but the cache still has one entry.
        let r2 = router.route(&observation, 0x07, DType::F32, 0);
        assert!(r2.is_some());
        assert_eq!(router.cache_len(), 1);

        // Different op_kind → distinct SpecKey → cache grows.
        let r3 = router.route(&observation, 0x09, DType::F32, 0);
        assert!(r3.is_some());
        assert_eq!(router.cache_len(), 2);
    }
}
