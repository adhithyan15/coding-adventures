//! Specialisation policy — Phase 3 V2 of MX05.
//!
//! Turns a [`ProfileObservation`] (Phase 2a output) into a [`SpecKey`]
//! (Phase 3 V1 type) when the observation crosses the spec's
//! "this op is hot enough and narrow enough to be worth specialising"
//! threshold.  The result is consumed by the dispatch loop (Phase 3
//! V3, future) which calls the backend's [`Specialiser`] with the
//! returned key, caches the result in [`SpecCache`], and routes
//! subsequent dispatches to the cached kernel.
//!
//! ## Spec rules (per MX05 §"Specialisation trigger")
//!
//! Trigger when **all** of:
//!
//! 1. `invocation_count >= min_invocations` (default 1000), AND
//! 2. **at least one** of:
//!    - **Constant input**: an input's `observed_min == observed_max`
//!      with `samples ≥ stability_threshold × invocation_count`.
//!    - **Range narrowing**: declared dtype is wider than the
//!      smallest dtype that contains observed range (e.g. F32 with
//!      observed range fitting in I8 / I16).  This V2 detects the
//!      *opportunity* and emits a `SpecKey` with the narrowed
//!      `RangeClass`; whether to actually narrow at the IR level is
//!      Phase 2b work that needs an expanded V2 dtype set
//!      (F16/I8/I16) which doesn't ship in V1.
//!
//! Shape stability — "same shape seen for ≥ 95% of invocations" — is
//! already implicit in `Profiler::subhash`: the subhash includes per-
//! tensor shape metadata, so distinct shapes produce distinct
//! observations and any 95%-stable case naturally collapses to a
//! single hot subhash.  Explicit shape-stability tracking is
//! deferred until we want to specialise *across* shape variants
//! (Phase 4 work).
//!
//! ## Pluggability
//!
//! The trait is the contract; [`DefaultPolicy`] is the V1 default
//! implementation.  Frameworks targeting specific workloads (LLM
//! inference, image processing, scientific simulation) can supply
//! their own policies that bias toward their workload's
//! specialisation opportunities — e.g. an LLM policy might bias
//! toward narrowing weights to I8 even when the observed range
//! formally fits I16.

use crate::profile::{ProfileObservation, TensorObservation};
use crate::spec::{RangeClass, ShapeClass, SpecKey};
use matrix_ir::DType;

// ────────────────────────── Trait ──────────────────────────

/// A pluggable policy that decides when an observed compute op is
/// worth specialising.  Implementations should be pure (no side
/// effects) and idempotent — the same observation must always yield
/// the same answer, modulo `&self` configuration.
///
/// `Send + Sync` so a policy can live behind a `Box<dyn>` shared
/// across threads (the dispatch loop is multi-threaded by design).
pub trait SpecialisationPolicy: Send + Sync {
    /// Examine `observation` and decide whether to specialise.  The
    /// caller supplies the op metadata that's not stored in the
    /// observation itself: which `Op` kind ran, what the output dtype
    /// was, and which `backend_id` should own the resulting kernel.
    ///
    /// Returns `Some(SpecKey)` when the policy fires and `None` to
    /// keep using the generic kernel.
    fn should_specialise(
        &self,
        observation: &ProfileObservation,
        op_kind: u8,
        output_dtype: DType,
        backend_id: u32,
    ) -> Option<SpecKey>;
}

// ────────────────────────── DefaultPolicy ──────────────────────────

/// V1 default policy.  Implements the rules from MX05 §"Specialisation
/// trigger" with conservative thresholds.
pub struct DefaultPolicy {
    /// Minimum number of dispatches before specialisation is
    /// considered.  Default 1000 — sits between V8 SparkPlug
    /// (~200) and V8 TurboFan (~10 000), per the spec's
    /// "Threshold rationale" section.
    pub min_invocations: u64,

    /// Fraction of `invocation_count` that an observation must reach
    /// for an input to count as "stable" (constant-valued).  Default
    /// 0.95 per the spec.  Caller can lower for aggressive
    /// specialisation or raise for higher-confidence-only.
    pub stability_threshold: f64,
}

impl DefaultPolicy {
    /// Construct with the spec defaults: 1000 invocations, 0.95
    /// stability threshold.
    pub fn new() -> Self {
        DefaultPolicy {
            min_invocations: 1000,
            stability_threshold: 0.95,
        }
    }

    /// Construct with custom thresholds.  `min_invocations == 0`
    /// disables the hotness gate entirely (every observation is
    /// considered for specialisation); useful for tests.
    pub fn with_thresholds(min_invocations: u64, stability_threshold: f64) -> Self {
        DefaultPolicy {
            min_invocations,
            stability_threshold,
        }
    }

    /// Test helper: fires on the first call past `min_invocations`,
    /// regardless of any narrowing/constant pattern.  Useful when a
    /// test just wants to confirm the dispatch path threads through
    /// the policy.
    fn forced_fire(
        observation: &ProfileObservation,
        op_kind: u8,
        output_dtype: DType,
        backend_id: u32,
    ) -> SpecKey {
        SpecKey {
            op_kind,
            dtype: output_dtype,
            shape_class: ShapeClass::Dynamic,
            range_class: range_class_for_observation(observation, output_dtype),
            backend_id,
        }
    }
}

impl Default for DefaultPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl SpecialisationPolicy for DefaultPolicy {
    fn should_specialise(
        &self,
        observation: &ProfileObservation,
        op_kind: u8,
        output_dtype: DType,
        backend_id: u32,
    ) -> Option<SpecKey> {
        if observation.invocation_count < self.min_invocations {
            return None;
        }

        // 1. Constant-input check: any input with observed_min == observed_max
        //    and stability threshold met.
        for tobs in observation
            .tensor_observations
            .iter()
            .filter(|t| t.is_input)
        {
            if is_constant_input(tobs, observation.invocation_count, self.stability_threshold) {
                let bytes = encode_constant_bytes(tobs, output_dtype);
                return Some(SpecKey {
                    op_kind,
                    dtype: output_dtype,
                    shape_class: ShapeClass::Dynamic,
                    range_class: RangeClass::Constant { bytes },
                    backend_id,
                });
            }
        }

        // 2. Range narrowing check: any input whose observed range is
        //    materially narrower than the declared dtype's full range.
        //    "Materially narrower" is conservative here — we check
        //    f32 inputs with observed range bounded.  Backends are free
        //    to ignore this signal in V1 because none of them yet
        //    support narrower dtypes; the policy still surfaces the
        //    opportunity so specialisers can record it.
        for tobs in observation.tensor_observations.iter() {
            if let Some(range) = narrowable_range(tobs, output_dtype) {
                return Some(SpecKey {
                    op_kind,
                    dtype: output_dtype,
                    shape_class: ShapeClass::Dynamic,
                    range_class: range,
                    backend_id,
                });
            }
        }

        None
    }
}

// ────────────────────────── helpers ──────────────────────────

/// True iff `tobs` looks like a constant input — same value seen on
/// at least `stability_threshold` fraction of dispatches.
fn is_constant_input(tobs: &TensorObservation, invocation_count: u64, threshold: f64) -> bool {
    if tobs.samples == 0 || tobs.observed_min != tobs.observed_max {
        return false;
    }
    // Approximate: if we've sampled at least `threshold * invocation_count`
    // scalars and the value hasn't changed, treat as stable.  This is
    // a coarse proxy; Phase 3 V3 may track per-invocation distinctness
    // more precisely.
    let samples = tobs.samples as f64;
    let invocations = invocation_count.max(1) as f64;
    samples / invocations >= threshold
}

/// Encode `tobs.observed_min` (== observed_max for a constant input)
/// as bytes in `dtype` little-endian.  Used to build a `RangeClass::Constant`.
fn encode_constant_bytes(tobs: &TensorObservation, dtype: DType) -> Vec<u8> {
    match dtype {
        DType::F32 => (tobs.observed_min as f32).to_le_bytes().to_vec(),
        DType::U8 => {
            let v = tobs.observed_min.clamp(0.0, 255.0) as u8;
            vec![v]
        }
        DType::I32 => {
            let v = tobs.observed_min.clamp(i32::MIN as f64, i32::MAX as f64) as i32;
            v.to_le_bytes().to_vec()
        }
    }
}

/// If `tobs`'s observed range is materially narrower than `dtype`'s
/// declared range, return a `RangeClass` that captures the
/// narrowing.  Conservative — for V1 we only fire when an F32 input
/// has its range bounded (i.e. neither end is infinite/NaN).  Phase
/// 2b will extend this to actually emit `Cast` ops; for V2 we just
/// record the observation so backends can act on it if they choose.
fn narrowable_range(tobs: &TensorObservation, dtype: DType) -> Option<RangeClass> {
    if tobs.samples == 0 {
        return None;
    }
    let min = tobs.observed_min;
    let max = tobs.observed_max;
    if !min.is_finite() || !max.is_finite() {
        return None;
    }
    match dtype {
        DType::F32 => {
            // Always emit FloatBits — backends decide whether the
            // bounds are tight enough to do anything with.  Skip the
            // sentinel "no observation" range.
            if min == f64::INFINITY || max == f64::NEG_INFINITY {
                return None;
            }
            Some(RangeClass::float(min, max))
        }
        DType::I32 | DType::U8 => {
            // Integer dtypes already fit; no narrowing opportunity in
            // V1's dtype set.  V2 with I8/I16 will revisit.
            None
        }
    }
}

/// Used by `forced_fire` (test helper) and by future policies that
/// want a sensible default range for an observation that doesn't
/// trigger narrowing.
#[allow(dead_code)]
fn range_class_for_observation(observation: &ProfileObservation, dtype: DType) -> RangeClass {
    for tobs in observation.tensor_observations.iter().filter(|t| t.is_input) {
        if let Some(rc) = narrowable_range(tobs, dtype) {
            return rc;
        }
    }
    RangeClass::Unknown
}

// ────────────────────────── tests ──────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use compute_ir::ExecutorId;

    fn obs(invocation_count: u64, tensor_obs: Vec<TensorObservation>) -> ProfileObservation {
        ProfileObservation {
            graph_subhash: 0xDEAD_BEEF,
            op_index: 0,
            invocation_count,
            last_executor: ExecutorId(0),
            tensor_observations: tensor_obs,
        }
    }

    fn t_in(slot: u32, min: f64, max: f64, samples: u64) -> TensorObservation {
        TensorObservation {
            slot,
            is_input: true,
            observed_min: min,
            observed_max: max,
            observed_zeros: 0,
            samples,
        }
    }

    #[test]
    fn below_min_invocations_returns_none() {
        let p = DefaultPolicy::new();
        let o = obs(500, vec![t_in(0, 0.0, 0.0, 500)]);
        assert!(p.should_specialise(&o, 0x07, DType::F32, 1).is_none());
    }

    #[test]
    fn at_min_invocations_with_constant_input_fires() {
        let p = DefaultPolicy::new();
        // 1000 invocations, input was observed as constant value 0.0
        // for all 1000 samples (one scalar per invocation = 100% stability).
        let o = obs(1000, vec![t_in(0, 42.0, 42.0, 1000)]);
        let key = p.should_specialise(&o, 0x07, DType::F32, 1).unwrap();
        assert_eq!(key.op_kind, 0x07);
        assert_eq!(key.dtype, DType::F32);
        assert_eq!(key.backend_id, 1);
        match key.range_class {
            RangeClass::Constant { bytes } => {
                let val = f32::from_le_bytes(bytes.as_slice().try_into().unwrap());
                assert_eq!(val, 42.0);
            }
            other => panic!("expected Constant, got {:?}", other),
        }
    }

    #[test]
    fn constant_input_below_stability_threshold_falls_through_to_narrowing() {
        let p = DefaultPolicy::new();
        // 1000 invocations but only 100 samples (stability ratio 0.1
        // — way below the 0.95 default).  Should NOT fire as constant
        // input.  But it has bounded range, so narrowing path fires.
        let o = obs(1000, vec![t_in(0, 42.0, 42.0, 100)]);
        let key = p.should_specialise(&o, 0x07, DType::F32, 1).unwrap();
        // Falls through to the range-narrowing branch (which always
        // emits FloatBits for bounded F32).
        match key.range_class {
            RangeClass::FloatBits { .. } => {}
            other => panic!("expected FloatBits, got {:?}", other),
        }
    }

    #[test]
    fn nonconstant_bounded_range_uses_narrowing_branch() {
        let p = DefaultPolicy::new();
        // Range [-1.0, 1.0] sampled on every invocation; not constant
        // (min != max) but bounded.
        let o = obs(2000, vec![t_in(0, -1.0, 1.0, 2000)]);
        let key = p.should_specialise(&o, 0x07, DType::F32, 1).unwrap();
        let (lo, hi) = match key.range_class {
            RangeClass::FloatBits {
                min_bits,
                max_bits,
            } => (f64::from_bits(min_bits), f64::from_bits(max_bits)),
            other => panic!("expected FloatBits, got {:?}", other),
        };
        assert_eq!(lo, -1.0);
        assert_eq!(hi, 1.0);
    }

    #[test]
    fn empty_tensor_observations_returns_none() {
        let p = DefaultPolicy::new();
        let o = obs(2000, vec![]);
        assert!(p.should_specialise(&o, 0x07, DType::F32, 1).is_none());
    }

    #[test]
    fn output_only_observations_dont_fire_constant_branch() {
        let p = DefaultPolicy::new();
        // Output observed as constant (samples meet threshold) but
        // we only fire on inputs.  Falls through to narrowing on the
        // output.
        let o = ProfileObservation {
            graph_subhash: 0,
            op_index: 0,
            invocation_count: 2000,
            last_executor: ExecutorId(0),
            tensor_observations: vec![TensorObservation {
                slot: 0,
                is_input: false,
                observed_min: 7.0,
                observed_max: 7.0,
                observed_zeros: 0,
                samples: 2000,
            }],
        };
        let key = p.should_specialise(&o, 0x07, DType::F32, 1).unwrap();
        // The narrowing branch picks up the output too (any
        // observation with bounded F32 range).
        match key.range_class {
            RangeClass::FloatBits { .. } => {}
            RangeClass::Constant { .. } => {
                panic!("constant branch should not fire on output-only observation")
            }
            other => panic!("expected FloatBits, got {:?}", other),
        }
    }

    #[test]
    fn integer_dtype_no_narrowing() {
        let p = DefaultPolicy::new();
        // U8 inputs are already at their narrowest in V1's dtype set.
        // Constant case still fires (it's a separate signal); pure
        // narrowing does not.
        let o = obs(2000, vec![t_in(0, 0.0, 255.0, 2000)]);
        // Constant condition isn't met (min != max) and narrowing
        // doesn't fire on U8 in V1.  Expect None.
        assert!(p.should_specialise(&o, 0x07, DType::U8, 1).is_none());
    }

    #[test]
    fn u8_constant_input_fires_with_byte_value() {
        let p = DefaultPolicy::new();
        // U8 input always == 200.
        let o = obs(2000, vec![t_in(0, 200.0, 200.0, 2000)]);
        let key = p.should_specialise(&o, 0x07, DType::U8, 1).unwrap();
        match key.range_class {
            RangeClass::Constant { bytes } => assert_eq!(bytes, vec![200]),
            other => panic!("expected Constant, got {:?}", other),
        }
    }

    #[test]
    fn i32_constant_input_fires_with_le_bytes() {
        let p = DefaultPolicy::new();
        let o = obs(2000, vec![t_in(0, -42.0, -42.0, 2000)]);
        let key = p.should_specialise(&o, 0x07, DType::I32, 1).unwrap();
        match key.range_class {
            RangeClass::Constant { bytes } => {
                let v = i32::from_le_bytes(bytes.as_slice().try_into().unwrap());
                assert_eq!(v, -42);
            }
            other => panic!("expected Constant, got {:?}", other),
        }
    }

    #[test]
    fn unbounded_range_does_not_fire_narrowing() {
        let p = DefaultPolicy::new();
        let o = obs(
            2000,
            vec![t_in(0, f64::NEG_INFINITY, f64::INFINITY, 2000)],
        );
        assert!(p.should_specialise(&o, 0x07, DType::F32, 1).is_none());
    }

    #[test]
    fn custom_thresholds_work() {
        // Lower the hotness gate so a 100-invocation observation
        // qualifies; lower the stability threshold so 50 samples count
        // as stable.
        let p = DefaultPolicy::with_thresholds(100, 0.5);
        let o = obs(100, vec![t_in(0, 5.0, 5.0, 50)]);
        assert!(p.should_specialise(&o, 0x07, DType::F32, 1).is_some());
    }

    #[test]
    fn forced_fire_helper_constructs_a_speckey() {
        let o = obs(2000, vec![t_in(0, -1.0, 1.0, 2000)]);
        let key = DefaultPolicy::forced_fire(&o, 0x07, DType::F32, 1);
        assert_eq!(key.op_kind, 0x07);
        assert_eq!(key.dtype, DType::F32);
        assert_eq!(key.backend_id, 1);
    }
}
