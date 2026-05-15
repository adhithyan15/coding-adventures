//! Matrix-IR-lowered radix-2 Cooley-Tukey FFT (DSP01 Phase 3b.ii).
//!
//! Builds a `matrix_ir::Graph` that computes the radix-2 FFT of a
//! power-of-2 length signal entirely through generic tensor ops:
//! `Slice`, `Concat`, `Mul`, `Sub`, `Add`, `Reshape`, `Broadcast`,
//! and `Const`.
//!
//! The same graph runs on every backend the matrix execution layer
//! supports (CPU today; Metal/CUDA once they claim Slice/Concat in
//! their `supported_ops` bitsets).
//!
//! ## Algorithm
//!
//! Standard radix-2 Cooley-Tukey decimation-in-time FFT.  Three
//! pieces:
//!
//! 1. **Real → complex** wrap: reshape `[N]` to `[N, 1]` and concat
//!    with a `[N, 1]` zeros tensor on axis 1 → `[N, 2]`.
//! 2. **Bit-reversal permutation**: for each output index `i`, pick
//!    input index `bit_reverse(i, log2(N))`.  V1 emits this as `N`
//!    width-1 `Slice` ops + one `Concat` on axis 0.  This generates
//!    `O(N)` ops per FFT; a future `Gather` op will collapse it to
//!    one op.  Skipped entirely for `N = 2` where bit-reversal is
//!    a no-op (`bit_reverse(0) = 0`, `bit_reverse(1) = 1`).
//! 3. **Butterfly stages**: `log2(N)` stages.  At each stage:
//!    - Reshape `[N, 2]` → `[N/full, full, 2]` to expose groups.
//!    - Slice axis 1 to split each group into `even` (size `half`)
//!      and `odd` (size `half`).
//!    - Multiply `odd` by a stage-specific twiddle Const of shape
//!      `[half, 2]` broadcast to `[N/full, half, 2]`.  The complex
//!      multiply is realised as four `Mul`s plus a `Sub` and an
//!      `Add` over the interleaved `[re, im]` axis.
//!    - `next_first = even + tw_odd`, `next_second = even - tw_odd`.
//!    - Concat the two halves on axis 1 → `[N/full, full, 2]` and
//!      reshape back to `[N, 2]` for the next stage.
//!
//! For inverse FFT, twiddles use positive sign and the final output
//! is scaled by `1 / N` (backward normalization, matching numpy /
//! scipy / MATLAB).
//!
//! ## V1 scope
//!
//! - Power-of-2 `N` only.  Bluestein for arbitrary lengths is
//!   Phase 4 of DSP01.
//! - F32 dtype only.
//! - Single-channel: input is `[N]`, output is `[N, 2]`.  Batched
//!   FFT is V2.
//! - The bit-reversal explosion (`O(N)` ops) caps practical `N` at
//!   maybe 1024 before the graph becomes unwieldy; a `Gather` op
//!   (future MX01 V2 extension) will fix this.
//!
//! ## Test strategy
//!
//! - **Graph validates** for `N ∈ {2, 4, 8, 16, 32}`: ensure the
//!   builder produces a well-formed graph (passes
//!   `Graph::validate()`).
//! - **End-to-end** for `N ∈ {2, 4}`: plan + dispatch on
//!   `matrix-cpu`, compare to `fft_scalar` within ULP tolerance.
//!   Larger `N` is exercised via the same path once Phase 3b.iii
//!   tightens the cross-backend test harness.

use crate::Direction;
use crate::FftError;
use matrix_ir::{DType, Graph, GraphBuilder, Shape, Tensor};

/// Build a `matrix_ir::Graph` that computes the FFT (or inverse
/// FFT, per `direction`) of a length-`n` real-valued input.
///
/// The graph takes one input of shape `[n]` (dtype F32) and produces
/// one output of shape `[n, 2]` holding the complex spectrum in
/// interleaved `[re, im]` layout.
///
/// Returns `Err` if `n < 2` or `n` is not a power of two.
pub fn build_fft_graph(n: u32, direction: Direction) -> Result<Graph, FftError> {
    if n < 2 || !n.is_power_of_two() {
        return Err(FftError::NotPowerOfTwo(n as usize));
    }
    let log_n = n.trailing_zeros() as usize;
    let mut b = GraphBuilder::new();

    // ── Step 1: real → complex.  [N] → [N, 1] → [N, 2] (concat with zeros).
    let input = b.input(DType::F32, Shape::from(&[n]));
    let zeros_bytes = vec![0u8; (n as usize) * 4];
    let zeros = b.constant(DType::F32, Shape::from(&[n, 1]), zeros_bytes);
    let input_2d = b.reshape(&input, Shape::from(&[n, 1]));
    let mut x = b.concat(&[&input_2d, &zeros], 1);

    // ── Step 2: bit-reversal permutation.
    //
    // For N=2 this is the identity, so we skip.  For larger N we
    // generate `N` width-1 slices and one Concat.  This produces a
    // big graph for big N; a Gather op (future MX01 V2 extension)
    // will collapse this to one op.
    if n > 2 {
        let mut slices: Vec<Tensor> = Vec::with_capacity(n as usize);
        for i in 0..n {
            let bri = bit_reverse(i as usize, log_n) as u32;
            let s = b.slice(&x, 0, bri, bri + 1, 1);
            slices.push(s);
        }
        let refs: Vec<&Tensor> = slices.iter().collect();
        x = b.concat(&refs, 0);
    }

    // ── Step 3: butterfly stages.
    let sign: f32 = match direction {
        Direction::Forward => -1.0,
        Direction::Inverse => 1.0,
    };
    let mut full: u32 = 2;
    while full <= n {
        let half = full / 2;
        let n_groups = n / full;

        // Expose group structure: [N, 2] → [n_groups, full, 2].
        x = b.reshape(&x, Shape::from(&[n_groups, full, 2]));

        // Split each group's `full` elements into even / odd halves.
        let even = b.slice(&x, 1, 0, half, 1);
        let odd = b.slice(&x, 1, half, full, 1);

        // Twiddle factors for this stage: [half, 2] of [cos, sin].
        let mut tw_bytes = Vec::with_capacity((half as usize) * 8);
        for j in 0..half {
            let theta = sign * 2.0 * std::f32::consts::PI * (j as f32) / (full as f32);
            tw_bytes.extend_from_slice(&theta.cos().to_le_bytes());
            tw_bytes.extend_from_slice(&theta.sin().to_le_bytes());
        }
        let tw = b.constant(DType::F32, Shape::from(&[half, 2]), tw_bytes);
        // Broadcast to [n_groups, half, 2].
        let tw_3d = b.reshape(&tw, Shape::from(&[1, half, 2]));
        let tw_b = b.broadcast(&tw_3d, Shape::from(&[n_groups, half, 2]));

        // Complex-multiply odd × twiddle on interleaved [re, im]:
        //   (a + bi)(c + di) = (ac - bd) + (ad + bc) i
        let odd_re = b.slice(&odd, 2, 0, 1, 1);
        let odd_im = b.slice(&odd, 2, 1, 2, 1);
        let tw_re = b.slice(&tw_b, 2, 0, 1, 1);
        let tw_im = b.slice(&tw_b, 2, 1, 2, 1);

        let ac = b.mul(&odd_re, &tw_re);
        let bd = b.mul(&odd_im, &tw_im);
        let ad = b.mul(&odd_re, &tw_im);
        let bc = b.mul(&odd_im, &tw_re);

        let tw_odd_re = b.sub(&ac, &bd);
        let tw_odd_im = b.add(&ad, &bc);
        let tw_odd = b.concat(&[&tw_odd_re, &tw_odd_im], 2);

        // Butterflies.
        let next_first = b.add(&even, &tw_odd);
        let next_second = b.sub(&even, &tw_odd);

        // Reassemble: [n_groups, full, 2] → [N, 2] for the next stage.
        x = b.concat(&[&next_first, &next_second], 1);
        x = b.reshape(&x, Shape::from(&[n, 2]));

        full *= 2;
    }

    // ── Inverse FFT: divide by N.
    if direction == Direction::Inverse {
        let inv_n = 1.0_f32 / (n as f32);
        let scale = b.constant(
            DType::F32,
            Shape::from(&[1, 1]),
            inv_n.to_le_bytes().to_vec(),
        );
        let scale_b = b.broadcast(&scale, Shape::from(&[n, 2]));
        x = b.mul(&x, &scale_b);
    }

    b.output(&x);
    b.build()
        .map_err(|e| FftError::InvalidInput(format!("graph build/validate: {:?}", e)))
}

/// Reverse the bottom `bits` bits of `x`.  Used by the bit-reversal
/// permutation step of build_fft_graph.
fn bit_reverse(mut x: usize, bits: usize) -> usize {
    let mut r = 0usize;
    for _ in 0..bits {
        r = (r << 1) | (x & 1);
        x >>= 1;
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_power_of_two() {
        assert!(matches!(
            build_fft_graph(3, Direction::Forward),
            Err(FftError::NotPowerOfTwo(3))
        ));
        assert!(matches!(
            build_fft_graph(6, Direction::Forward),
            Err(FftError::NotPowerOfTwo(6))
        ));
    }

    #[test]
    fn rejects_too_small() {
        assert!(matches!(
            build_fft_graph(0, Direction::Forward),
            Err(FftError::NotPowerOfTwo(0))
        ));
        assert!(matches!(
            build_fft_graph(1, Direction::Forward),
            Err(FftError::NotPowerOfTwo(1))
        ));
    }

    #[test]
    fn graph_validates_for_small_sizes() {
        // The builder runs validate() on .build(); reaching Ok means
        // every op + tensor passes structural and semantic checks.
        for n in [2u32, 4, 8, 16, 32] {
            let g_fwd = build_fft_graph(n, Direction::Forward).expect(&format!(
                "build_fft_graph N={} Forward should validate",
                n
            ));
            assert_eq!(g_fwd.inputs.len(), 1);
            assert_eq!(g_fwd.outputs.len(), 1);
            let out_t = &g_fwd.tensors[g_fwd.outputs[0].0 as usize];
            assert_eq!(out_t.shape.dims, vec![n, 2]);

            let g_inv = build_fft_graph(n, Direction::Inverse).expect(&format!(
                "build_fft_graph N={} Inverse should validate",
                n
            ));
            assert_eq!(g_inv.outputs.len(), 1);
        }
    }

    /// For N=2 the FFT graph collapses to:
    ///   y0 = x0 + x1, y1 = x0 - x1
    /// (twiddle = 1+0i, no bit-reversal).  Verifies the basic
    /// structure: one input, one output, output shape [2, 2].
    #[test]
    fn n2_graph_has_expected_shape() {
        let g = build_fft_graph(2, Direction::Forward).unwrap();
        let in_t = &g.tensors[g.inputs[0].id.0 as usize];
        assert_eq!(in_t.shape.dims, vec![2]);
        let out_t = &g.tensors[g.outputs[0].0 as usize];
        assert_eq!(out_t.shape.dims, vec![2, 2]);
    }

    #[test]
    fn bit_reverse_known_values() {
        // 3-bit examples.
        assert_eq!(bit_reverse(0b000, 3), 0b000);
        assert_eq!(bit_reverse(0b001, 3), 0b100);
        assert_eq!(bit_reverse(0b011, 3), 0b110);
        assert_eq!(bit_reverse(0b101, 3), 0b101);
        assert_eq!(bit_reverse(0b111, 3), 0b111);
    }
}
