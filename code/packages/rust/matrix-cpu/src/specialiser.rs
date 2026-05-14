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

/// **MX05 Phase 4.9.**  Given a `SpecKey`, build the Rust closure
/// that implements the specialised op on a CPU `BufferStore`.  This
/// is the matrix-cpu equivalent of matrix-metal's `msl_emitter` —
/// instead of generating an MSL string, we generate a closure
/// (`Box<SpecialisedKernelFn>`) ready to install on a `CpuExecutor`
/// via [`crate::CpuExecutor::install_specialised`].
///
/// Mirrors matrix-metal's emitter coverage:
///   - **Commutative binary** ops (Add, Mul, Max, Min) with a
///     folded constant — `out[i] = a[i] OP K`.
///   - **Non-commutative binary** ops (Sub, Div, Pow) with a
///     `folded_slot` set — `out[i] = K OP a[i]` or `a[i] OP K`.
///   - **Unary** ops (Neg, Abs, Sqrt, Exp, Log, Tanh, Recip) with a
///     folded input constant — `out[i] = f(K)` (memset).
///
/// f32 only in V1, matching the metal emitter.  Returns `None` for
/// shapes the matrix-cpu specialiser doesn't yet know how to lower.
///
/// ## Why a separate function, not a trait method
///
/// The `Specialiser` trait in matrix-profile lives in a crate that
/// doesn't know about closures or executors — its
/// `specialise(key) -> Option<SpecialisedKernel>` returns just the
/// opaque handle.  Phase 4.9 keeps that contract intact and instead
/// adds this dedicated function that downstream callers
/// (image-gpu-core's auto-installer) invoke after a successful
/// `route()` to build the matching closure.
///
/// Pattern matches matrix-metal's `emit_specialised_kernel(key, handle)`.
pub fn build_specialised_kernel(
    key: &SpecKey,
    _handle: u64,
) -> Option<Box<crate::SpecialisedKernelFn>> {
    use matrix_ir::DType;
    use matrix_profile::RangeClass;

    // V1: f32 only, RangeClass::Constant only.
    if key.dtype != DType::F32 {
        return None;
    }
    let RangeClass::Constant { bytes } = &key.range_class else {
        return None;
    };

    // **MX05 Phase 4.10.**  Op::MatMul (0x15) with a folded matrix.
    // The constant is more than 4 bytes — a flat row-major f32
    // matrix.  Branch off before the single-f32 check below.
    if key.op_kind == 0x15 {
        return build_matmul_with_folded_matrix(key, bytes);
    }

    if bytes.len() != 4 {
        return None;
    }
    let arr: [u8; 4] = bytes.as_slice().try_into().ok()?;
    let constant = f32::from_le_bytes(arr);

    // Commutative binary ops: same kernel regardless of which slot
    // was folded.  Pass-through pattern: `a[i] OP K`.
    let commutative: Option<fn(f32, f32) -> f32> = match key.op_kind {
        0x07 => Some(|a, k| a + k), // Add
        0x09 => Some(|a, k| a * k), // Mul
        0x0B => Some(|a, k| a.max(k)), // Max
        0x0C => Some(|a, k| a.min(k)), // Min
        _ => None,
    };
    if let Some(op) = commutative {
        return Some(build_binary_f32_kernel(constant, op));
    }

    // Unary with folded input: precompute f(K) at build time, then
    // the closure is a pure memset of `precomputed` for every
    // element.  `folded_slot` must be Some(0) since unary ops have
    // exactly one input slot.
    let folded_slot = key.folded_slot?;
    let unary_precomputed: Option<f32> = match key.op_kind {
        0x00 if folded_slot == 0 => Some(-constant),
        0x01 if folded_slot == 0 => Some(constant.abs()),
        0x02 if folded_slot == 0 => Some(constant.sqrt()),
        0x03 if folded_slot == 0 => Some(constant.exp()),
        0x04 if folded_slot == 0 => Some(constant.ln()),
        0x05 if folded_slot == 0 => Some(constant.tanh()),
        0x06 if folded_slot == 0 => Some(1.0_f32 / constant),
        _ => None,
    };
    if let Some(precomputed) = unary_precomputed {
        return Some(build_unary_memset_f32_kernel(precomputed));
    }

    // Non-commutative binary ops: pick the variant based on
    // `folded_slot`.
    //   Some(0) → constant is LHS → `out[i] = K OP a[i]`
    //   Some(1) → constant is RHS → `out[i] = a[i] OP K`
    let non_commutative_op: Option<(fn(f32, f32) -> f32, fn(f32, f32) -> f32)> = match key.op_kind {
        0x08 => Some((|a, k| k - a, |a, k| a - k)), // Sub: (lhs-folded, rhs-folded)
        0x0A => Some((|a, k| k / a, |a, k| a / k)), // Div
        0x0D => Some((|a, k| k.powf(a), |a, k| a.powf(k))), // Pow
        _ => None,
    };
    if let Some((lhs_op, rhs_op)) = non_commutative_op {
        let op = match folded_slot {
            0 => lhs_op,
            1 => rhs_op,
            _ => return None,
        };
        return Some(build_binary_f32_kernel(constant, op));
    }

    None
}

/// **MX05 Phase 4.9 helper.**  Build a closure that reads input[0]
/// (f32 buffer) element-by-element and writes `op(in[i], K)` to
/// output[0].  The kernel takes 1 input + 1 output buffer.
fn build_binary_f32_kernel(
    constant: f32,
    op: fn(f32, f32) -> f32,
) -> Box<crate::SpecialisedKernelFn> {
    use executor_protocol::OpTiming;
    Box::new(move |buffers, inputs, outputs| {
        if inputs.len() != 1 {
            return Err(format!(
                "specialised binary kernel expects 1 input, got {}",
                inputs.len()
            ));
        }
        if outputs.len() != 1 {
            return Err(format!(
                "specialised binary kernel expects 1 output, got {}",
                outputs.len()
            ));
        }
        let in_data = buffers.get(inputs[0])?.to_vec();
        let n_elems = in_data.len() / 4;
        let mut out_data = vec![0u8; n_elems * 4];
        for i in 0..n_elems {
            let arr: [u8; 4] = in_data[i * 4..(i + 1) * 4].try_into().unwrap();
            let a = f32::from_le_bytes(arr);
            let result = op(a, constant);
            out_data[i * 4..(i + 1) * 4].copy_from_slice(&result.to_le_bytes());
        }
        buffers.write(outputs[0], 0, &out_data)?;
        Ok(vec![OpTiming { op_index: 0, ns: 0 }])
    })
}

/// **MX05 Phase 4.9 helper.**  Build a closure that writes the
/// precomputed `value` to every f32 element of `output[0]`.  Kernel
/// takes 0 inputs + 1 output — matching matrix-metal's unary
/// memset pattern from Phase 4.7.
fn build_unary_memset_f32_kernel(value: f32) -> Box<crate::SpecialisedKernelFn> {
    use executor_protocol::OpTiming;
    Box::new(move |buffers, inputs, outputs| {
        if !inputs.is_empty() {
            return Err(format!(
                "specialised unary memset kernel expects 0 inputs, got {}",
                inputs.len()
            ));
        }
        if outputs.len() != 1 {
            return Err(format!(
                "specialised unary memset kernel expects 1 output, got {}",
                outputs.len()
            ));
        }
        let out_len = buffers.get(outputs[0])?.len();
        let n_elems = out_len / 4;
        let mut out_data = vec![0u8; n_elems * 4];
        let val_bytes = value.to_le_bytes();
        for i in 0..n_elems {
            out_data[i * 4..(i + 1) * 4].copy_from_slice(&val_bytes);
        }
        buffers.write(outputs[0], 0, &out_data)?;
        Ok(vec![OpTiming { op_index: 0, ns: 0 }])
    })
}

/// **MX05 Phase 4.10.**  Build a CPU closure for
/// `Op::MatMul(A[m, k] × B[k, n] = C[m, n])` where one of the
/// operands is a folded constant matrix.  V1 supports
/// `folded_slot = Some(1)` (RHS folded) with the constant matrix
/// being 2×2 (4 elements) or 4×4 (16 elements) — same coverage as
/// matrix-metal's emitter.
///
/// The non-folded input is a runtime `[m, dim]` matrix where `m`
/// derives from the input buffer's byte length.  Each output row
/// is `m` dot products against the folded matrix's columns.
fn build_matmul_with_folded_matrix(
    key: &SpecKey,
    bytes: &[u8],
) -> Option<Box<crate::SpecialisedKernelFn>> {
    use executor_protocol::OpTiming;

    // Only RHS-folded in V1.
    if key.folded_slot != Some(1) {
        return None;
    }
    if bytes.len() % 4 != 0 {
        return None;
    }
    let n_floats = bytes.len() / 4;
    let dim = match n_floats {
        4 => 2usize,
        16 => 4usize,
        _ => return None,
    };
    let matrix: Vec<f32> = bytes
        .chunks(4)
        .map(|c| {
            let arr: [u8; 4] = c.try_into().unwrap();
            f32::from_le_bytes(arr)
        })
        .collect();

    Some(Box::new(move |buffers, inputs, outputs| {
        if inputs.len() != 1 {
            return Err(format!(
                "specialised matmul kernel expects 1 input, got {}",
                inputs.len()
            ));
        }
        if outputs.len() != 1 {
            return Err(format!(
                "specialised matmul kernel expects 1 output, got {}",
                outputs.len()
            ));
        }
        let in_data = buffers.get(inputs[0])?.to_vec();
        // Input is `[m, dim]` row-major f32.  Derive m from byte
        // length: m = bytes / (dim * 4).  Reject mis-shaped inputs.
        if in_data.len() % (dim * 4) != 0 {
            return Err(format!(
                "matmul kernel input length {} not a multiple of {} f32s (dim {})",
                in_data.len(),
                dim,
                dim
            ));
        }
        let m = in_data.len() / (dim * 4);
        // Output is `[m, dim]` row-major f32 = m * dim * 4 bytes.
        let out_len = m * dim * 4;
        let mut out_data = vec![0u8; out_len];

        for r in 0..m {
            for c in 0..dim {
                // C[r, c] = sum_k A[r, k] * B[k, c]
                let mut sum = 0.0_f32;
                for k in 0..dim {
                    let a_bytes_start = (r * dim + k) * 4;
                    let a_arr: [u8; 4] = in_data[a_bytes_start..a_bytes_start + 4]
                        .try_into()
                        .unwrap();
                    let a_val = f32::from_le_bytes(a_arr);
                    let b_val = matrix[k * dim + c];
                    sum += a_val * b_val;
                }
                let out_bytes_start = (r * dim + c) * 4;
                out_data[out_bytes_start..out_bytes_start + 4]
                    .copy_from_slice(&sum.to_le_bytes());
            }
        }

        buffers.write(outputs[0], 0, &out_data)?;
        Ok(vec![OpTiming { op_index: 0, ns: 0 }])
    }))
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

    // **MX05 Phase 4.6.**  `folded_slot` is part of the key
    // identity (an LHS-folded Sub kernel is mathematically distinct
    // from an RHS-folded one), so it has to feed into the handle.
    // `None` and `Some(0)`/`Some(1)`/... are all distinguishable
    // — we encode `None` as discriminator byte 0xFF, then for
    // `Some(s)` we emit `0x00` followed by `s`.
    match key.folded_slot {
        None => feed_byte(0xFF, &mut h),
        Some(s) => {
            feed_byte(0x00, &mut h);
            feed_byte(s, &mut h);
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
            folded_slot: None,
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
