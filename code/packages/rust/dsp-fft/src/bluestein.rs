//! # Bluestein's algorithm — FFT for arbitrary lengths
//!
//! **DSP01 Phase 4a (scalar reference).**  Computes the discrete
//! Fourier transform of a length-`N` sequence for *any* `N ≥ 1`,
//! including non-power-of-two lengths the radix-2 path can't
//! touch.  This phase ships the scalar reference only;
//! later phases (4b: rfft/irfft, 4c: matrix-ir lowered Bluestein)
//! build on top.
//!
//! ## Why Bluestein?
//!
//! The matrix-ir-lowered FFT in [`crate::radix2`] requires `N` to
//! be a power of two — the butterfly tree's depth is `log₂(N)`
//! and the bit-reversal permutation is a power-of-two identity.
//! Real-world signal lengths are usually *not* powers of two
//! (think 1000 samples, 882 samples for audio resampling, etc.).
//!
//! Bluestein's algorithm — also called the chirp z-transform —
//! recasts the length-`N` DFT as a length-`M` *linear convolution*,
//! where `M ≥ 2N - 1` and we pick the next power of two.  The
//! length-`M` convolution is then computed via three length-`M`
//! FFTs (which the radix-2 path handles natively).  The price is
//! one extra FFT-pair worth of memory traffic and three times the
//! arithmetic of a same-size radix-2 FFT; the win is that *every*
//! `N` works with one code path.
//!
//! ## Algorithm
//!
//! The chirp-z identity reorganizes `n · k` as a difference of
//! squares (a "chirp"):
//!
//! ```text
//!     n · k = ( n² + k² - (k - n)² ) / 2
//! ```
//!
//! Substituting into the forward DFT:
//!
//! ```text
//!     X[k] = Σ_n  x[n] · exp(-2πi · n · k / N)
//!          = exp(-iπ · k² / N) · Σ_n  ( x[n] · exp(-iπ · n² / N) )
//!                                      · exp(+iπ · (k - n)² / N)
//! ```
//!
//! Define:
//!
//! - `a[n] = x[n] · exp(-iπ · n² / N)`   ("pre-chirp"; length `N`)
//! - `b[n] = exp(+iπ · n² / N)`           ("anti-chirp"; length
//!   `2N - 1` if indexed from `-(N - 1)` to `N - 1`)
//!
//! Then `X[k] = exp(-iπ · k² / N) · (a ⋆ b)[k]` for `k = 0..N-1`,
//! where `⋆` is linear convolution.
//!
//! To compute the convolution we pick `M = next_pow2(2N - 1)` and:
//!
//! 1. Zero-pad `a` from length `N` to length `M`.
//! 2. Build `b'` of length `M` by wrapping the bilateral chirp:
//!    `b'[k] = exp(+iπ · k² / N)` for `k = 0..N-1`,
//!    `b'[k] = exp(+iπ · (k - M)² / N)` for `k = M - N + 1..M-1`,
//!    `b'[k] = 0` elsewhere.
//!    The "wrap" gives us the negative-index half of `b`.
//! 3. Compute `A = FFT(a)`, `B = FFT(b')`, `C[k] = A[k] · B[k]`
//!    (elementwise complex multiply).
//! 4. `c = IFFT(C)`.  The first `N` samples of `c` are the
//!    linear convolution `(a ⋆ b)[0..N]`.
//! 5. `X[k] = exp(-iπ · k² / N) · c[k]` for `k = 0..N-1`.
//!
//! Inverse FFT is the same algorithm with the chirp sign flipped
//! and a final `1/N` scaling (backward normalization, matching
//! the radix-2 path).
//!
//! ## Complexity
//!
//! - **Time**: three length-`M` FFTs at `M · log₂(M)` ops each,
//!   plus three length-`M` pointwise complex multiplies — i.e.
//!   `O(M log M) = O(N log N)` since `M < 4N`.
//! - **Memory**: `O(M) = O(N)` working storage.
//! - **Numerical accuracy**: the chirp computation involves
//!   floating-point modular arithmetic on `n²`, which loses
//!   precision for very large `N` (`n²` for `n = 65535` is
//!   `2³²`-scale and the residue mod `2N` loses bits).  This
//!   matters past `N ≈ 1M` in `f32`; below that, the round-trip
//!   stays within `1e-4` like the radix-2 path.
//!
//! ## What this module did NOT include in Phase 4a
//!
//! - ~Matrix-ir lowering~ — Phase 4c (this release) adds
//!   [`build_bluestein_graph`] and [`bluestein_via_runtime`] so
//!   the whole convolution lifts onto the matrix execution
//!   layer.  The scalar `bluestein_scalar` stays as the oracle.
//! - `rfft` / `irfft` half-spectrum APIs.  Phase 4b (already
//!   shipped in 0.6.0).
//! - Real-input optimisation.  Bluestein on a real signal still
//!   does the full complex convolution; the half-spectrum win
//!   comes from `rfft`.

use crate::radix2::add_radix2_fft_to_builder;
use crate::{fft_scalar, ifft_scalar, Direction, FftError};
use matrix_ir::{DType, Graph, GraphBuilder, Shape, Tensor, TensorId};

/// Forward / inverse FFT via Bluestein's algorithm.  Accepts any
/// `N ≥ 1`, including non-power-of-two lengths.  Operates on
/// interleaved `[re, im, re, im, …]` `f32` buffers — same layout
/// as [`fft_scalar`].
///
/// `signal` must have even length (one `[re, im]` pair per
/// element).  Length is interpreted as `N = signal.len() / 2`.
///
/// The output has the same length and convention as the
/// corresponding direction of [`fft_scalar`] / [`ifft_scalar`].
/// In particular the inverse direction applies the same
/// "backward" `1/N` normalization (matches numpy / scipy /
/// MATLAB).
///
/// For power-of-two `N` the radix-2 path
/// ([`fft_scalar`] / [`ifft_scalar`]) is faster.  This routine
/// is the canonical fallback for everything else.
pub fn bluestein_scalar(
    signal: &[f32],
    direction: Direction,
) -> Result<Vec<f32>, FftError> {
    if signal.len() % 2 != 0 {
        return Err(FftError::InvalidInput(format!(
            "interleaved buffer must have even length; got {}",
            signal.len()
        )));
    }
    let n = signal.len() / 2;
    if n == 0 {
        return Err(FftError::InvalidInput(
            "FFT length must be at least 1".into(),
        ));
    }

    // ── Trivial N = 1 case: the DFT is the identity.
    //   Skip the chirp construction entirely.  We still apply the
    //   inverse direction's `1/N` factor, but with N = 1 that's a
    //   no-op too.
    if n == 1 {
        return Ok(signal.to_vec());
    }

    // ── Step 0: pick the convolution length M = next_pow2(2N - 1).
    //   This is the smallest power of two that fits a linear
    //   convolution of two length-N sequences (length 2N - 1).
    let conv_len = 2 * n - 1;
    let m = conv_len.next_power_of_two();

    // ── Step 1: build the chirp.  The pre-chirp factor for index
    //   `k` is `exp(sign · iπ · k² / N)` where `sign = -1` for
    //   forward, `+1` for inverse.  We compute `k² mod 2N` to
    //   stay precise — the chirp is periodic with period `2N`
    //   (since `exp(±iπ · 2N / N) = exp(±2πi) = 1`).
    //
    //   We store the chirp interleaved `[re, im]` as a small
    //   helper Vec.  `chirp[k] = exp(sign · iπ · k² / N)`.
    let sign: f32 = match direction {
        Direction::Forward => -1.0,
        Direction::Inverse => 1.0,
    };
    let chirp = build_chirp(n, sign);

    // ── Step 2: a[n] = x[n] · chirp[n], padded to length M with
    //   zeros.  Output is interleaved length 2M.
    let mut a = vec![0.0f32; 2 * m];
    for k in 0..n {
        let xr = signal[2 * k];
        let xi = signal[2 * k + 1];
        let cr = chirp[2 * k];
        let ci = chirp[2 * k + 1];
        // Complex multiply: (xr + i·xi) · (cr + i·ci)
        //   = (xr·cr - xi·ci) + i(xr·ci + xi·cr)
        a[2 * k]     = xr * cr - xi * ci;
        a[2 * k + 1] = xr * ci + xi * cr;
    }

    // ── Step 3: build b' of length M.  This is the bilateral
    //   chirp `exp(-sign · iπ · k² / N)` (note opposite sign from
    //   the pre-chirp), indexed from -(N-1) to N-1 and wrapped
    //   onto [0, M).
    //
    //   - b'[k] for k = 0..N         = chirp_conj[k]
    //   - b'[k] for k = M-N+1..M     = chirp_conj[M - k]
    //   - b'[k] for k = N..M-N       = 0
    //
    //   `chirp_conj` is `exp(-sign · iπ · k² / N)`, which is
    //   the elementwise conjugate of `chirp` (sign-flipped imag).
    let mut b = vec![0.0f32; 2 * m];
    for k in 0..n {
        b[2 * k]     =  chirp[2 * k];      // re unchanged
        b[2 * k + 1] = -chirp[2 * k + 1];  // im negated (conjugate)
    }
    for k in 1..n {
        // Index M - k wraps to the negative half of the bilateral
        // chirp.  Since `(-k)² == k²`, `chirp_conj[M-k]` is just
        // the same chirp_conj[k] value.
        let idx = m - k;
        b[2 * idx]     =  chirp[2 * k];
        b[2 * idx + 1] = -chirp[2 * k + 1];
    }

    // ── Step 4: convolve via FFT.  A = FFT(a), B = FFT(b),
    //   C[k] = A[k] · B[k], c = IFFT(C).
    //
    //   `m` is a power of two by construction so `fft_scalar`
    //   accepts both calls.
    let a_spec = fft_scalar(&a)?;
    let b_spec = fft_scalar(&b)?;
    let mut c_spec = vec![0.0f32; 2 * m];
    for k in 0..m {
        let ar = a_spec[2 * k];
        let ai = a_spec[2 * k + 1];
        let br = b_spec[2 * k];
        let bi = b_spec[2 * k + 1];
        c_spec[2 * k]     = ar * br - ai * bi;
        c_spec[2 * k + 1] = ar * bi + ai * br;
    }
    let conv = ifft_scalar(&c_spec)?;

    // ── Step 5: X[k] = chirp[k] · conv[k] for k = 0..N.
    //
    //   For inverse direction we additionally divide by N to
    //   match the "backward" normalization convention.  The
    //   `ifft_scalar` call above already divided by M (the
    //   convolution length), but the outer DFT is length N, so
    //   we need an extra `M / N` correction — except wait, no:
    //   the chirp identity gives us *linear convolution*, and
    //   `ifft_scalar(FFT(a) · FFT(b))` is *circular convolution*
    //   of length M.  Linear ⊆ circular when M ≥ 2N - 1, which
    //   we guaranteed in Step 0, so the first N samples are the
    //   linear convolution we want.
    //
    //   Normalization: `ifft_scalar` divides by M.  But the
    //   linear convolution we want has *no* such division — it's
    //   `Σ_j a[j] · b[k-j]`.  The chirp identity for the
    //   *forward* DFT comes out clean: `X[k] = chirp[k] · conv[k]`
    //   with no extra factor.  For the *inverse* DFT we also need
    //   a `1/N` factor to match the backward convention.
    let mut out = vec![0.0f32; 2 * n];
    let inv_n: f32 = if direction == Direction::Inverse {
        1.0 / (n as f32)
    } else {
        1.0
    };
    for k in 0..n {
        let cr = conv[2 * k];
        let ci = conv[2 * k + 1];
        let wr = chirp[2 * k];
        let wi = chirp[2 * k + 1];
        // (wr + i·wi) · (cr + i·ci)
        let re = wr * cr - wi * ci;
        let im = wr * ci + wi * cr;
        out[2 * k]     = re * inv_n;
        out[2 * k + 1] = im * inv_n;
    }
    Ok(out)
}

/// Build the chirp sequence `chirp[k] = exp(sign · iπ · k² / N)`
/// for `k = 0..N`, returned as an interleaved `[re, im]` buffer.
///
/// The exponent argument is `sign · π · (k² mod 2N) / N` —
/// reducing `k²` modulo `2N` before the floating-point divide
/// keeps the value bounded to `(-π, π)` regardless of how large
/// `k` gets, which is what saves us precision for large `N`.
///
/// Why `2N` and not `N`?  Because the chirp has period `2N`:
/// `exp(iπ · (k + 2N)² / N)` differs from `exp(iπ · k² / N)`
/// by `exp(iπ · (4kN + 4N²) / N) = exp(4πi · k + 4πi · N) = 1`.
fn build_chirp(n: usize, sign: f32) -> Vec<f32> {
    use std::f32::consts::PI;
    let two_n = (2 * n) as u64;
    let mut out = Vec::with_capacity(2 * n);
    for k in 0..n {
        // k² mod 2N — compute in u64 to handle large N without
        // overflow.  N ≤ usize::MAX, k < N, k² could be up to
        // N² which on 64-bit usize already fits, but using u64
        // explicit is clearer.
        let k_sq = (k as u64).wrapping_mul(k as u64);
        let residue = (k_sq % two_n) as f32;
        let theta = sign * PI * residue / (n as f32);
        let (im, re) = theta.sin_cos();
        out.push(re);
        out.push(im);
    }
    out
}

/// **DSP01 Phase 4c.**  Build a `matrix_ir::Graph` that computes
/// the Bluestein FFT of a length-`N` signal (interleaved
/// `[re, im]` complex, length `2N` `f32`).  The signal is
/// embedded as a `Const` — no runtime inputs are declared — and
/// the returned `TensorId` identifies the output buffer to
/// download.
///
/// This is the matrix-ir analogue of [`bluestein_scalar`].  The
/// chirp Consts and the bilateral `b'` Const are precomputed in
/// Rust at graph-build time; the three length-`M` FFTs run as
/// composed radix-2 subgraphs (via
/// [`crate::radix2::add_radix2_fft_to_builder`]).  When
/// `matrix-metal` / `matrix-cuda` claim Slice + Concat in their
/// `supported_ops` bitsets, the whole graph lifts onto the GPU
/// — exactly what Phase 4c was meant to unlock.
///
/// Output shape is `[N, 2]` interleaved complex.
///
/// Returns `Err` if `signal` has odd length, is empty, or
/// represents a single complex element (`N = 1` is degenerate;
/// callers should route it through the scalar path).
pub fn build_bluestein_graph_with_input(
    signal: &[f32],
    direction: Direction,
) -> Result<(Graph, TensorId), FftError> {
    if signal.len() % 2 != 0 {
        return Err(FftError::InvalidInput(format!(
            "interleaved buffer must have even length; got {}",
            signal.len()
        )));
    }
    let n_usize = signal.len() / 2;
    if n_usize < 2 {
        // N = 0 or 1 are degenerate.  N = 0 has no Const to
        // embed; N = 1 has no FFT structure to lower (M = 1
        // would underflow `add_radix2_fft_to_builder`'s N ≥ 2
        // contract).  Callers should handle these via the
        // scalar path.
        return Err(FftError::InvalidInput(format!(
            "matrix-ir Bluestein requires N ≥ 2; got N = {}",
            n_usize
        )));
    }
    let n = n_usize as u32;

    // ── Step 0: pick the convolution length M = next_pow2(2N - 1).
    let conv_len = 2 * n_usize - 1;
    let m_usize = conv_len.next_power_of_two();
    let m = m_usize as u32;

    // ── Precompute chirp + bilateral-chirp values in Rust.
    let sign: f32 = match direction {
        Direction::Forward => -1.0,
        Direction::Inverse => 1.0,
    };
    let chirp = build_chirp(n_usize, sign);
    //   `b_full[k]` = conj(chirp[k]) for k ∈ 0..N,
    //   `b_full[M-k]` = conj(chirp[k]) for k ∈ 1..N,
    //   zeros otherwise (the linear-convolution zero pad).
    let mut b_full = vec![0.0f32; 2 * m_usize];
    for k in 0..n_usize {
        b_full[2 * k]     =  chirp[2 * k];
        b_full[2 * k + 1] = -chirp[2 * k + 1];
    }
    for k in 1..n_usize {
        let idx = m_usize - k;
        b_full[2 * idx]     =  chirp[2 * k];
        b_full[2 * idx + 1] = -chirp[2 * k + 1];
    }

    // Pack chirp / b_full into little-endian byte buffers for Consts.
    let chirp_bytes = floats_to_le_bytes(&chirp);
    let b_full_bytes = floats_to_le_bytes(&b_full);

    let mut bb = GraphBuilder::new();

    // ── Step 1: embed the signal as a `Const` shaped `[N, 2]`.
    //   The input arrives interleaved, so this is a direct
    //   reinterpret — no Concat/Reshape needed unlike the radix-2
    //   real-only path.
    let signal_bytes = floats_to_le_bytes(signal);
    let signal_const = bb.constant(DType::F32, Shape::from(&[n, 2]), signal_bytes);
    let chirp_const = bb.constant(DType::F32, Shape::from(&[n, 2]), chirp_bytes);
    let b_full_const = bb.constant(DType::F32, Shape::from(&[m, 2]), b_full_bytes);

    // ── Step 2: a = signal · chirp  (elementwise complex multiply).
    let a_n2 = complex_multiply_n2(&mut bb, &signal_const, &chirp_const, n);

    // ── Step 3: pad a from [N, 2] to [M, 2] with zeros.
    let zeros_pad = bb.constant(
        DType::F32,
        Shape::from(&[m - n, 2]),
        vec![0u8; (m - n) as usize * 8],
    );
    let a_padded = bb.concat(&[&a_n2, &zeros_pad], 0);

    // ── Step 4: A = FFT(a_padded).
    let cap_a = add_radix2_fft_to_builder(&mut bb, a_padded, m, Direction::Forward)?;

    // ── Step 5: B = FFT(b_full_const).
    let cap_b =
        add_radix2_fft_to_builder(&mut bb, b_full_const, m, Direction::Forward)?;

    // ── Step 6: C = A · B  (elementwise complex multiply on [M, 2]).
    let cap_c = complex_multiply_n2(&mut bb, &cap_a, &cap_b, m);

    // ── Step 7: c = IFFT(C).  The radix-2 inverse divides by M.
    let c_m = add_radix2_fft_to_builder(&mut bb, cap_c, m, Direction::Inverse)?;

    // ── Step 8: take the first N samples of c.
    let c_n = bb.slice(&c_m, 0, 0, n, 1);

    // ── Step 9: x = c_n · chirp  (post-chirp; reuses the same Const).
    let mut x_n2 = complex_multiply_n2(&mut bb, &c_n, &chirp_const, n);

    // ── Step 10 (inverse direction only): scale by 1/N.
    //   The intermediate IFFT already divided by M, so this gives
    //   the final backward-normalised inverse.
    if direction == Direction::Inverse {
        let inv_n = 1.0_f32 / (n as f32);
        let scale = bb.constant(
            DType::F32,
            Shape::from(&[1, 1]),
            inv_n.to_le_bytes().to_vec(),
        );
        let scale_b = bb.broadcast(&scale, Shape::from(&[n, 2]));
        x_n2 = bb.mul(&x_n2, &scale_b);
    }

    let out_id = x_n2.id;
    bb.output(&x_n2);
    let graph = bb
        .build()
        .map_err(|e| FftError::InvalidInput(format!("graph build/validate: {:?}", e)))?;
    Ok((graph, out_id))
}

/// **DSP01 Phase 4c.**  End-to-end execution of the matrix-ir
/// lowered Bluestein FFT.  Builds the graph via
/// [`build_bluestein_graph_with_input`], plans it through
/// `matrix-runtime`, dispatches on a fresh `matrix-cpu`
/// executor, downloads the spectrum, and returns it as an
/// interleaved `[re, im, …, re, im]` `Vec<f32>` of length `2N`.
///
/// Same output convention and `direction` semantics as
/// [`bluestein_scalar`].  This is the canonical "Bluestein
/// actually runs on the matrix execution layer" entry point;
/// once Metal / CUDA claim Slice + Concat the call lifts to GPU
/// with no `dsp-fft` change.
pub fn bluestein_via_runtime(
    signal: &[f32],
    direction: Direction,
) -> Result<Vec<f32>, FftError> {
    use compute_ir::ComputeGraph;
    use executor_protocol::{ExecutorRequest, ExecutorResponse};
    use matrix_cpu::CpuExecutor;
    use matrix_runtime::Runtime;

    let (graph, output_id) = build_bluestein_graph_with_input(signal, direction)?;
    let n = signal.len() / 2;
    let output_byte_count = n * 2 * 4;

    let runtime = Runtime::new(matrix_cpu::profile());
    let placed: ComputeGraph = runtime
        .plan(&graph)
        .map_err(|e| FftError::InvalidInput(format!("plan: {:?}", e)))?;

    // Find the output residency the same way `fft_via_runtime` does.
    let output_residency = placed
        .outputs
        .iter()
        .find(|t| t.id == output_id)
        .map(|t| t.residency)
        .or_else(|| placed.tensors.get(output_id.0 as usize).map(|t| t.residency))
        .ok_or_else(|| {
            FftError::InvalidInput(format!(
                "output tensor {} not in placed graph",
                output_id.0
            ))
        })?;

    let executor = CpuExecutor::new();

    match executor.handle(ExecutorRequest::Dispatch {
        job_id: 1,
        graph: placed,
    }) {
        ExecutorResponse::DispatchDone { .. } => {}
        ExecutorResponse::Error { code, message, .. } => {
            return Err(FftError::InvalidInput(format!(
                "dispatch error 0x{:04X}: {}",
                code.0, message
            )));
        }
        other => {
            return Err(FftError::InvalidInput(format!(
                "unexpected response to Dispatch: {:?}",
                other
            )));
        }
    }

    let download = executor.handle(ExecutorRequest::DownloadBuffer {
        buffer: output_residency.buffer,
        offset: 0,
        len: output_byte_count as u64,
    });
    let bytes = match download {
        ExecutorResponse::BufferData { data, .. } => data,
        ExecutorResponse::Error { code, message, .. } => {
            return Err(FftError::InvalidInput(format!(
                "download error 0x{:04X}: {}",
                code.0, message
            )));
        }
        other => {
            return Err(FftError::InvalidInput(format!(
                "unexpected response to DownloadBuffer: {:?}",
                other
            )));
        }
    };

    let mut out: Vec<f32> = Vec::with_capacity(bytes.len() / 4);
    for chunk in bytes.chunks_exact(4) {
        let arr: [u8; 4] = chunk.try_into().unwrap();
        out.push(f32::from_le_bytes(arr));
    }
    Ok(out)
}

/// Helper: elementwise complex multiply on two `[N, 2]` tensors.
/// `(ar + i ai)(br + i bi) = (ar br - ai bi) + i (ar bi + ai br)`.
/// Returns a `[N, 2]` tensor.
///
/// The `_n` parameter (number of rows) is currently unused inside
/// the body — matrix-ir's Mul / Slice / Concat infer the shape
/// from the operand metadata — but it's kept in the signature to
/// document the caller's contract and to keep the call site
/// self-explanatory at the use point.
fn complex_multiply_n2(
    b: &mut GraphBuilder,
    lhs: &Tensor,
    rhs: &Tensor,
    _n: u32,
) -> Tensor {
    let lhs_re = b.slice(lhs, 1, 0, 1, 1);
    let lhs_im = b.slice(lhs, 1, 1, 2, 1);
    let rhs_re = b.slice(rhs, 1, 0, 1, 1);
    let rhs_im = b.slice(rhs, 1, 1, 2, 1);

    let ac = b.mul(&lhs_re, &rhs_re);
    let bd = b.mul(&lhs_im, &rhs_im);
    let ad = b.mul(&lhs_re, &rhs_im);
    let bc = b.mul(&lhs_im, &rhs_re);

    let out_re = b.sub(&ac, &bd);
    let out_im = b.add(&ad, &bc);
    b.concat(&[&out_re, &out_im], 1)
}

/// Helper: pack a slice of `f32`s into little-endian bytes for a
/// `Const` tensor.  Used by `build_bluestein_graph_with_input`
/// to construct chirp / signal Consts.
fn floats_to_le_bytes(floats: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(floats.len() * 4);
    for &x in floats {
        out.extend_from_slice(&x.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    /// Loose float equality with a "scale-aware" tolerance — we
    /// compare magnitudes, so `tol` is interpreted as a relative
    /// epsilon for values above 1.0 and an absolute epsilon below.
    fn approx_eq(a: f32, b: f32, tol: f32) -> bool {
        let scale = a.abs().max(b.abs()).max(1.0);
        (a - b).abs() <= scale * tol
    }

    fn assert_close(a: &[f32], b: &[f32], tol: f32) {
        assert_eq!(a.len(), b.len(), "length mismatch");
        for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
            assert!(
                approx_eq(*x, *y, tol),
                "mismatch at {}: {} vs {} (tol {})",
                i,
                x,
                y,
                tol
            );
        }
    }

    /// Naive O(N²) DFT for use as a small-N test oracle.  We use
    /// it to validate Bluestein against the textbook definition
    /// at the non-power-of-two sizes the radix-2 path can't run.
    fn naive_dft(signal: &[f32], direction: Direction) -> Vec<f32> {
        let n = signal.len() / 2;
        let sign: f32 = match direction {
            Direction::Forward => -1.0,
            Direction::Inverse => 1.0,
        };
        let scale: f32 = match direction {
            Direction::Forward => 1.0,
            Direction::Inverse => 1.0 / (n as f32),
        };
        let mut out = vec![0.0f32; 2 * n];
        for k in 0..n {
            let mut sr = 0.0f32;
            let mut si = 0.0f32;
            for nn in 0..n {
                let theta = sign * 2.0 * PI * (k as f32) * (nn as f32) / (n as f32);
                let (wi, wr) = theta.sin_cos();
                let xr = signal[2 * nn];
                let xi = signal[2 * nn + 1];
                sr += wr * xr - wi * xi;
                si += wr * xi + wi * xr;
            }
            out[2 * k]     = sr * scale;
            out[2 * k + 1] = si * scale;
        }
        out
    }

    // ── error paths ────────────────────────────────────────────

    #[test]
    fn rejects_odd_length_buffer() {
        let err = bluestein_scalar(&[1.0, 2.0, 3.0], Direction::Forward).unwrap_err();
        assert!(matches!(err, FftError::InvalidInput(_)));
    }

    #[test]
    fn rejects_empty_buffer() {
        let err = bluestein_scalar(&[], Direction::Forward).unwrap_err();
        assert!(matches!(err, FftError::InvalidInput(_)));
    }

    // ── degenerate / power-of-two N ─────────────────────────────

    #[test]
    fn n1_is_identity_forward() {
        let signal = vec![3.5f32, -1.25];
        let spectrum = bluestein_scalar(&signal, Direction::Forward).unwrap();
        assert_close(&spectrum, &signal, 1e-7);
    }

    #[test]
    fn n1_is_identity_inverse() {
        let signal = vec![3.5f32, -1.25];
        let recovered = bluestein_scalar(&signal, Direction::Inverse).unwrap();
        assert_close(&recovered, &signal, 1e-7);
    }

    #[test]
    fn bluestein_matches_radix2_for_power_of_two_n8() {
        // Sanity check: at power-of-two N, Bluestein and the
        // radix-2 path should agree to within numerical
        // tolerance.  This is a cross-check that the chirp
        // construction is correct.
        let signal: Vec<f32> = (0..8)
            .flat_map(|i| [(i as f32) * 0.3 - 0.7, ((i as f32) * 0.11).sin()])
            .collect();
        let via_radix2 = fft_scalar(&signal).unwrap();
        let via_bluestein = bluestein_scalar(&signal, Direction::Forward).unwrap();
        assert_close(&via_bluestein, &via_radix2, 1e-4);
    }

    // ── non-power-of-two N (the whole point of Bluestein) ───────

    #[test]
    fn forward_n3_matches_naive_dft() {
        // N = 3: smallest non-trivial non-pow2.  Convolution length
        // M = next_pow2(2·3 - 1) = next_pow2(5) = 8.
        let signal = vec![1.0f32, 0.0, 2.0, 0.0, 3.0, 0.0];
        let via_bluestein = bluestein_scalar(&signal, Direction::Forward).unwrap();
        let via_naive = naive_dft(&signal, Direction::Forward);
        assert_close(&via_bluestein, &via_naive, 1e-4);
    }

    #[test]
    fn forward_n5_matches_naive_dft() {
        // N = 5: M = next_pow2(9) = 16.
        let signal: Vec<f32> = (0..5)
            .flat_map(|i| [(i as f32) - 2.0, (i as f32) * 0.5])
            .collect();
        let via_bluestein = bluestein_scalar(&signal, Direction::Forward).unwrap();
        let via_naive = naive_dft(&signal, Direction::Forward);
        assert_close(&via_bluestein, &via_naive, 1e-4);
    }

    #[test]
    fn forward_n6_matches_naive_dft() {
        // N = 6: M = next_pow2(11) = 16.
        let signal: Vec<f32> = (0..6)
            .flat_map(|i| {
                let x = (2.0 * PI * (i as f32) / 6.0).cos();
                [x, 0.0f32]
            })
            .collect();
        let via_bluestein = bluestein_scalar(&signal, Direction::Forward).unwrap();
        let via_naive = naive_dft(&signal, Direction::Forward);
        assert_close(&via_bluestein, &via_naive, 1e-4);
    }

    #[test]
    fn forward_n7_matches_naive_dft() {
        // N = 7 is a prime — the worst-case scenario for any
        // mixed-radix FFT.  Bluestein still handles it at the
        // same cost (M = next_pow2(13) = 16).
        let signal: Vec<f32> = (0..7)
            .flat_map(|i| [(i as f32).sin(), ((i as f32) * 0.3).cos()])
            .collect();
        let via_bluestein = bluestein_scalar(&signal, Direction::Forward).unwrap();
        let via_naive = naive_dft(&signal, Direction::Forward);
        assert_close(&via_bluestein, &via_naive, 1e-4);
    }

    #[test]
    fn forward_n12_matches_naive_dft() {
        // N = 12 is composite but not a power of two.
        // M = next_pow2(23) = 32.
        let signal: Vec<f32> = (0..12)
            .flat_map(|i| [((i as f32) * 0.1).sin(), ((i as f32) * 0.07).cos()])
            .collect();
        let via_bluestein = bluestein_scalar(&signal, Direction::Forward).unwrap();
        let via_naive = naive_dft(&signal, Direction::Forward);
        assert_close(&via_bluestein, &via_naive, 1e-4);
    }

    // ── round-trip ─────────────────────────────────────────────

    #[test]
    fn round_trip_n3() {
        let original = vec![1.0f32, 0.0, 2.0, 0.0, 3.0, 0.0];
        let spectrum = bluestein_scalar(&original, Direction::Forward).unwrap();
        let recovered = bluestein_scalar(&spectrum, Direction::Inverse).unwrap();
        assert_close(&original, &recovered, 1e-4);
    }

    #[test]
    fn round_trip_n7_real_complex_mix() {
        let n: usize = 7;
        let original: Vec<f32> = (0..n)
            .flat_map(|i| [(i as f32) * 0.3, ((i as f32) * 0.5).sin()])
            .collect();
        let spectrum = bluestein_scalar(&original, Direction::Forward).unwrap();
        let recovered = bluestein_scalar(&spectrum, Direction::Inverse).unwrap();
        assert_close(&original, &recovered, 1e-4);
    }

    #[test]
    fn round_trip_works_for_many_sizes() {
        // Stress test: round-trip every N from 1 to 32.
        // Includes power-of-two N (where Bluestein and radix-2
        // both work) and arbitrary N (Bluestein's home turf).
        for n in 1..=32usize {
            let original: Vec<f32> = (0..n)
                .flat_map(|i| [((i as f32) * 0.15).sin(), 0.0f32])
                .collect();
            let spectrum =
                bluestein_scalar(&original, Direction::Forward).unwrap();
            let recovered =
                bluestein_scalar(&spectrum, Direction::Inverse).unwrap();
            // Tolerance scales loosely with N; 1e-3 covers N=32
            // comfortably.
            for (i, (a, b)) in original.iter().zip(recovered.iter()).enumerate() {
                let scale = a.abs().max(b.abs()).max(1.0);
                assert!(
                    (a - b).abs() <= scale * 1e-3,
                    "round-trip failed for N={}, index {}: {} vs {}",
                    n,
                    i,
                    a,
                    b
                );
            }
        }
    }

    // ── closed-form known vectors ──────────────────────────────

    #[test]
    fn forward_impulse_n5_is_all_ones() {
        // fft(impulse) = [1, 1, …, 1] regardless of N.
        let n = 5;
        let mut signal = vec![0.0f32; 2 * n];
        signal[0] = 1.0;
        let spectrum = bluestein_scalar(&signal, Direction::Forward).unwrap();
        for k in 0..n {
            assert!(
                approx_eq(spectrum[2 * k], 1.0, 1e-4),
                "bin {} real = {}, expected 1.0",
                k,
                spectrum[2 * k]
            );
            assert!(
                approx_eq(spectrum[2 * k + 1], 0.0, 1e-4),
                "bin {} imag = {}, expected 0.0",
                k,
                spectrum[2 * k + 1]
            );
        }
    }

    // ── Phase 4c: matrix-ir-lowered Bluestein ──────────────────

    #[test]
    fn matrix_ir_bluestein_rejects_n1() {
        // N = 1 is degenerate; the matrix-ir builder requires N ≥ 2.
        let signal = vec![3.5f32, -1.25];
        let err =
            build_bluestein_graph_with_input(&signal, Direction::Forward).unwrap_err();
        assert!(matches!(err, FftError::InvalidInput(_)));
    }

    #[test]
    fn matrix_ir_bluestein_rejects_odd_buffer() {
        let err = build_bluestein_graph_with_input(&[1.0, 2.0, 3.0], Direction::Forward)
            .unwrap_err();
        assert!(matches!(err, FftError::InvalidInput(_)));
    }

    #[test]
    fn graph_validates_for_small_non_pow2_sizes() {
        // The graph builder should produce a well-formed graph
        // for the canonical "non-pow2" sizes our scalar tests
        // cover.
        for &n in &[2usize, 3, 5, 6, 7, 8, 12] {
            let signal: Vec<f32> = (0..n).flat_map(|i| [(i as f32) * 0.5, 0.0]).collect();
            for &dir in &[Direction::Forward, Direction::Inverse] {
                let (graph, _id) =
                    build_bluestein_graph_with_input(&signal, dir).unwrap_or_else(
                        |e| panic!("graph build failed for N={}, dir={:?}: {:?}", n, dir, e),
                    );
                // `build` already validates internally, so reaching
                // here means the graph passed validate().  The
                // assertion is implicit; we also check the output
                // shape is the expected [N, 2].
                let out_id = *graph
                    .outputs
                    .first()
                    .expect("graph has at least one output");
                let out_tensor = &graph.tensors[out_id.0 as usize];
                assert_eq!(
                    out_tensor.shape.dims,
                    vec![n as u32, 2],
                    "wrong output shape for N={}, dir={:?}",
                    n,
                    dir
                );
            }
        }
    }

    #[test]
    fn bluestein_via_runtime_matches_scalar_n3_forward() {
        let signal = vec![1.0f32, 0.0, 2.0, 0.0, 3.0, 0.0];
        let via_runtime = bluestein_via_runtime(&signal, Direction::Forward).unwrap();
        let via_scalar = bluestein_scalar(&signal, Direction::Forward).unwrap();
        assert_close(&via_runtime, &via_scalar, 1e-4);
    }

    #[test]
    fn bluestein_via_runtime_matches_scalar_n5_forward() {
        let signal: Vec<f32> = (0..5)
            .flat_map(|i| [(i as f32) - 2.0, (i as f32) * 0.5])
            .collect();
        let via_runtime = bluestein_via_runtime(&signal, Direction::Forward).unwrap();
        let via_scalar = bluestein_scalar(&signal, Direction::Forward).unwrap();
        assert_close(&via_runtime, &via_scalar, 1e-4);
    }

    #[test]
    fn bluestein_via_runtime_matches_scalar_n7_forward() {
        // N = 7 prime — Bluestein's whole reason to exist.
        let signal: Vec<f32> = (0..7)
            .flat_map(|i| [(i as f32).sin(), ((i as f32) * 0.3).cos()])
            .collect();
        let via_runtime = bluestein_via_runtime(&signal, Direction::Forward).unwrap();
        let via_scalar = bluestein_scalar(&signal, Direction::Forward).unwrap();
        assert_close(&via_runtime, &via_scalar, 1e-4);
    }

    #[test]
    fn bluestein_via_runtime_matches_scalar_n6_inverse() {
        // Inverse direction exercises the chirp-sign flip and the
        // outer 1/N scaling in the matrix-ir graph.
        let signal: Vec<f32> = (0..6)
            .flat_map(|i| [((i as f32) * 0.4).cos(), 0.0f32])
            .collect();
        let via_runtime = bluestein_via_runtime(&signal, Direction::Inverse).unwrap();
        let via_scalar = bluestein_scalar(&signal, Direction::Inverse).unwrap();
        assert_close(&via_runtime, &via_scalar, 1e-4);
    }

    #[test]
    fn bluestein_via_runtime_round_trip_n5() {
        // Round-trip: forward then inverse through the runtime,
        // and expect to recover the input.  Tests both directions
        // of the matrix-ir graph in sequence.
        let original: Vec<f32> = (0..5)
            .flat_map(|i| [(i as f32), (i as f32) * 0.5])
            .collect();
        let spectrum = bluestein_via_runtime(&original, Direction::Forward).unwrap();
        let recovered = bluestein_via_runtime(&spectrum, Direction::Inverse).unwrap();
        assert_close(&original, &recovered, 1e-4);
    }

    // ── original closed-form tests (Phase 4a) continue below ──

    #[test]
    fn forward_dc_n7_is_single_bin() {
        // fft(constant) = [N, 0, 0, …, 0].
        let n = 7;
        let signal: Vec<f32> = (0..n).flat_map(|_| [1.0f32, 0.0]).collect();
        let spectrum = bluestein_scalar(&signal, Direction::Forward).unwrap();
        assert!(approx_eq(spectrum[0], n as f32, 1e-4));
        assert!(approx_eq(spectrum[1], 0.0, 1e-4));
        for k in 1..n {
            assert!(
                approx_eq(spectrum[2 * k], 0.0, 1e-3),
                "bin {} real = {}, expected 0",
                k,
                spectrum[2 * k]
            );
            assert!(
                approx_eq(spectrum[2 * k + 1], 0.0, 1e-3),
                "bin {} imag = {}, expected 0",
                k,
                spectrum[2 * k + 1]
            );
        }
    }
}
