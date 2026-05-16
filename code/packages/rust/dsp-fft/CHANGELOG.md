# Changelog — dsp-fft

## 0.6.0 — 2026-05-15

### Added — DSP01 Phase 4b (rfft / irfft, half-spectrum API for real inputs)

Adds the `rfft` / `irfft` pair to the public API.  These exploit
conjugate symmetry of real-input spectra (`X[N - k] = conj(X[k])`)
to return only the first `⌊N / 2⌋ + 1` complex bins, saving ~2×
memory over `fft` + slice.

#### New module: `dsp_fft::rfft`

```rust
pub fn rfft_scalar(signal: &[f32]) -> Result<Vec<f32>, FftError>;
pub fn irfft_scalar(
    half_spectrum: &[f32],
    output_length: u32,
) -> Result<Vec<f32>, FftError>;

pub fn rfft(signal: &[f32]) -> Result<ComplexTensor, FftError>;
pub fn irfft(
    half_spectrum: &ComplexTensor,
    output_length: u32,
) -> Result<Vec<f32>, FftError>;
```

Re-exported at the crate root as `dsp_fft::{rfft, irfft,
rfft_scalar, irfft_scalar}`.

#### Algorithm

`rfft`:

1. Wrap the real signal as interleaved complex with `im = 0`.
2. Run the existing complex `fft` (radix-2 for power-of-two
   `N`, Bluestein for everything else).
3. Slice off the first `⌊N / 2⌋ + 1` bins.

`irfft`:

1. Reconstruct the full length-`N` spectrum using
   `X[N - k] = conj(X[k])` for `k in 1..(N + 1) / 2`.  For even
   `N`, bin `N / 2` (Nyquist) is taken straight from the
   half-spectrum; for odd `N` there's no Nyquist and the
   reflection loop hits every non-DC bin.
2. Run the existing complex `ifft`.
3. Return only the real lane.

Both directions work for **any `N ≥ 1`** — power-of-two via
radix-2, non-pow2 via Bluestein.  `output_length` is required on
`irfft` because `⌊N / 2⌋ + 1` doesn't uniquely determine `N`
(matches `numpy.fft.irfft(a, n)`).

#### Why no "half-length packing trick" yet?

The classic optimisation packs even/odd real samples into `N/2`
complex elements, FFTs at half the length, and unpacks via
twiddle multiplication — saves ~2× wall-clock but doesn't change
asymptotic complexity.  Our V1 implementation just calls the
existing `fft` / `ifft` (which already handle any `N`), then
slices / mirrors.  Phase 5 will add the packing optimisation
where perf actually matters.

#### New unit tests (18)

Error paths:
- `rfft_rejects_empty_signal`
- `irfft_rejects_zero_output_length`
- `irfft_rejects_odd_buffer_length`
- `irfft_rejects_mismatched_bin_count`

Closed-form known vectors:
- `rfft_of_impulse_is_all_ones_n8`
- `rfft_of_dc_is_single_bin_n8`
- `rfft_of_pure_cosine_concentrates_one_bin_n16`

Round-trips (`irfft(rfft(x)) ≈ x` within 1e-4):
- N = 1, 8, 16, 64 (power of two)
- N = 3, 7, 12 (non-power of two — exercises Bluestein in both
  directions inside `rfft`/`irfft`)
- Stress sweep for every `N ∈ 1..=20`

Public API:
- `public_rfft_returns_complex_tensor`
- `public_irfft_round_trips_via_complex_tensor`
- `public_rfft_irfft_round_trip_non_pow2`

All 63 unit tests pass.

### What this phase does NOT include

- **Phase 4c**: matrix-ir-lowered Bluestein so non-pow2 FFTs
  also lift onto the matrix execution layer.  `rfft` / `irfft`
  internally still call `fft_scalar` / `bluestein_scalar` /
  `ifft_scalar` — all CPU-only.
- **Phase 5**: the half-length packing optimisation for `rfft`.
  Constant-factor speedup; not on the correctness path.

## 0.5.0 — 2026-05-15

### Added — DSP01 Phase 4a (scalar Bluestein for arbitrary lengths)

The public `fft` / `ifft` API now accepts **arbitrary `N ≥ 1`**.
Previously, non-power-of-two lengths returned
`FftError::NotPowerOfTwo`; now they fall back to a scalar
Bluestein (chirp z-transform) implementation that handles every
length with one code path.

#### New module: `dsp_fft::bluestein`

```rust
pub fn bluestein_scalar(
    signal: &[f32],
    direction: Direction,
) -> Result<Vec<f32>, FftError>;
```

Operates on the same interleaved `[re, im]` `f32` convention
as `fft_scalar`.  Algorithm:

1. Pick `M = next_pow2(2N - 1)` — the smallest power of two
   that fits a linear convolution of two length-`N` sequences.
2. Build the pre-chirp `a[n] = x[n] · exp(-iπ · n² / N)`,
   zero-padded to length `M`.
3. Build the bilateral anti-chirp `b[n] = exp(+iπ · n² / N)`
   wrapped onto `[0, M)`.
4. Convolve via three length-`M` FFTs: `c = IFFT(FFT(a) · FFT(b))`.
5. Multiply by the post-chirp: `X[k] = exp(-iπ · k² / N) · c[k]`.

The chirp uses `k² mod 2N` reduction inside the floating-point
exponent so very large `N` doesn't bleed precision.  Inverse
direction flips the chirp sign and applies the `1/N` backward
normalization.

`bluestein_scalar` is exposed at the crate root as
`dsp_fft::bluestein_scalar` for callers that want to bypass
`fft` / `ifft`.

#### Public API integration

| Input shape                     | `complex` | N parity        | Path                |
| ------------------------------- | --------- | --------------- | ------------------- |
| `[N]` real, `N ≥ 2` pow2        | `false`   | pow2            | matrix-ir → runtime |
| `[N]` real, `N ≥ 2` non-pow2    | `false`   | non-pow2 (new)  | scalar Bluestein    |
| `[N]` real, `N = 1`             | `false`   | degenerate      | scalar (identity)   |
| `[2N]` interleaved, `N` pow2    | `true`    | pow2            | scalar radix-2      |
| `[2N]` interleaved, `N` non-pow2 | `true`    | non-pow2 (new) | scalar Bluestein    |
| `ifft` on `[N, 2]`, `N` pow2    | —         | pow2            | scalar radix-2      |
| `ifft` on `[N, 2]`, `N` non-pow2 | —         | non-pow2 (new) | scalar Bluestein    |

#### New unit tests

**`bluestein` module — 15 tests:**

- Error paths: odd-length buffer, empty buffer.
- N = 1 identity (forward + inverse).
- Sanity vs radix-2 at N = 8 (cross-check the chirp).
- vs naive O(N²) DFT for N ∈ {3, 5, 6, 7, 12}.
- Round-trip for N ∈ {3, 7} + a stress test sweeping every
  N ∈ 1..=32.
- Closed-form impulse at N = 5, DC at N = 7.

**`lib` public API — 3 new tests:**

- `public_fft_real_non_pow2_routes_through_bluestein` —
  verifies N = 3 against an inline naive DFT.
- `public_ifft_round_trip_non_pow2_n5` — `ifft(fft(x)) ≈ x` for
  N = 5 (where both directions go through Bluestein).
- `public_fft_complex_non_pow2_routes_through_bluestein` —
  complex-input round-trip at N = 3.

All 45 unit tests pass.

#### Behavior changes

- `fft(&[1.0, 2.0, 3.0], false)` previously returned
  `Err(FftError::NotPowerOfTwo(3))`; now returns the correct
  3-point DFT.
- `fft_scalar` and `ifft_scalar` retain their power-of-two-only
  behavior — they're the radix-2 oracle and stay that way.
  Use `bluestein_scalar` or the public `fft` / `ifft` for
  arbitrary lengths.

### What this phase does NOT include

- **Phase 4b**: `rfft` / `irfft`.  Real-input half-spectrum APIs
  that exploit conjugate symmetry to return only `N/2 + 1` bins.
- **Phase 4c**: matrix-ir-lowered Bluestein.  The convolution
  inside `bluestein_scalar` currently calls `fft_scalar` /
  `ifft_scalar` directly; lifting it onto the matrix execution
  layer is its own (significant) chunk of work.
- Performance benchmarking.  Bluestein has ~3× the constant
  factor of an in-place radix-2; perf is Phase 5.

## 0.4.0 — 2026-05-15

### Changed — DSP01 Phase 3b.iv (public `fft()` real-input path runs on the matrix execution layer)

The public `fft(signal, complex = false)` entry point now routes
real-valued, power-of-two inputs through `fft_via_runtime` —
i.e. the FFT actually runs end-to-end through `matrix-runtime` +
`matrix-cpu`.  When `matrix-metal` / `matrix-cuda` claim Slice +
Concat in their `supported_ops` bitsets, the same call lifts to
GPU automatically with no `dsp-fft` change required.  That's the
whole point of building on top of MX.

What stays scalar for now:

- `fft(signal, complex = true)` — the matrix-ir graph builder
  always Concats a zero imaginary lane (i.e. it's real-input
  only), so complex inputs continue to call `fft_scalar`.  A
  follow-up phase will add a complex-input graph variant.
- `ifft(spectrum: &ComplexTensor)` — the matrix-ir FFT graph's
  input is `[N]` real and its output is `[N, 2]` complex; an
  inverse from `[N, 2]` complex doesn't fit the same graph and
  is deferred.

`fft_scalar` and `ifft_scalar` remain available as the canonical
oracle.

#### Behavior

- `fft(&real, false)` for power-of-two `N ≥ 2` is byte-for-byte
  equivalent to the Phase 2 scalar path within `1e-4` relative
  tolerance (matches the Phase 3b.iii contract).
- `N = 1` still works — the public API falls back to the scalar
  path for the degenerate single-element case so the API surface
  is unchanged from Phase 2.
- `fft(&[], false)` and `fft(&[1.0, 2.0, 3.0], false)` still
  return errors; the exact `FftError` variant is preserved
  (`InvalidInput` for empty, `NotPowerOfTwo(3)` for non-pow2).

#### New public re-exports

- `pub use radix2::build_fft_graph_with_input;`
- `pub use radix2::fft_via_runtime;`

Previously these were only reachable via `dsp_fft::radix2::*`.
They're now first-class items at the crate root so callers
don't have to dip into the submodule.

#### New unit tests

6 new tests in `tests`:

- `public_fft_real_matches_scalar_via_runtime_n8` — real ramp,
  routed through the matrix-runtime path, matches `fft_scalar`
  to 1e-4.
- `public_fft_real_matches_scalar_via_runtime_n16_sinusoid` —
  pure cosine, every butterfly stage exercised.
- `public_fft_real_impulse_via_runtime` — closed-form impulse
  check, all bins = 1.0 within 1e-5.
- `public_round_trip_real_through_runtime_and_scalar_ifft` —
  `ifft(fft(x)) ≈ x` for an 8-point real signal, where the
  forward goes through the matrix-ir graph and the inverse
  through the scalar reference.  Verifies the forward path's
  correctness end-to-end against the public API.
- `public_fft_complex_input_still_goes_through_scalar` —
  contract test: `fft(&interleaved, true)` returns *exactly*
  what `fft_scalar(&interleaved)` returns (bit-identical, not
  approximate), pinning the Phase 2 behavior for complex
  inputs.
- `public_fft_real_rejects_non_power_of_two` — error path:
  length validation still returns `FftError::NotPowerOfTwo`.

All 28 unit tests pass (17 from earlier phases + 6 new + 5
Phase 3b.iii e2e + … net 6 added at the public-API layer).

### What this phase does NOT include

- Complex-input `fft(&interleaved, true)` on the matrix-runtime
  path.  Needs a new graph builder that takes `[N, 2]` complex
  input instead of building one out of `[N]` real + Concat.
- `ifft` on the matrix-runtime path.  Same reason — the
  current `build_fft_graph(_, Direction::Inverse)` only works
  when the input is `[N]` real.
- Performance benchmarking.  Functional parity is the bar
  for Phase 3b.iv; perf is Phase 5.

## 0.3.0 — 2026-05-13

### Added — DSP01 Phase 3b.iii (end-to-end FFT execution via the matrix execution layer)

The matrix-ir-lowered FFT now **actually runs** end-to-end through
`matrix-runtime` + `matrix-cpu`.  Closes the DSP01 → MX01 loop:
build the graph → plan it → dispatch it → download the spectrum.

#### New public API

- `pub fn build_fft_graph_with_input(signal: &[f32], direction: Direction) -> Result<(Graph, TensorId), FftError>`
  — variant of `build_fft_graph` that embeds the input as a `Const`
  in the graph.  Matches `image-gpu-core::run_graph_with_constant_inputs`'
  pattern: graph has no runtime inputs, only the output is
  downloaded.  Returns the output `TensorId` so callers know which
  buffer to read.
- `pub fn fft_via_runtime(signal: &[f32], direction: Direction) -> Result<Vec<f32>, FftError>`
  — end-to-end helper.  Builds the graph, plans it through
  `Runtime::new(matrix_cpu::profile())`, dispatches it on a fresh
  `CpuExecutor`, downloads the output buffer, and returns the
  interleaved `[re, im, ..., re, im]` spectrum as a `Vec<f32>`.

When Metal / CUDA claim Slice + Concat in their `supported_ops`
bitsets, the same call lifts to GPU automatically with no
dsp-fft change required — that's the whole point of building on
top of MX.

#### New end-to-end tests

5 new unit tests in `radix2::tests`:

- `fft_via_runtime_matches_scalar_n2` — closed-form 2-point FFT.
- `fft_via_runtime_matches_scalar_n4` — 4-point with mixed input.
- `fft_via_runtime_matches_scalar_n8` — 8-point with linear ramp.
- `fft_via_runtime_matches_scalar_n16` — 16-point with sinusoidal
  input.  Exercises every stage of the log2(16) = 4-stage butterfly.
- `inverse_fft_via_runtime_round_trips` — `ifft(fft(x)) ≈ x` for an
  8-point real signal, where the forward FFT goes through the
  matrix-ir graph and the inverse FFT goes through the scalar
  reference.  Verifies the forward path's correctness end-to-end.

All compare against the scalar oracle (`fft_scalar` / `ifft_scalar`)
within `1e-4` relative tolerance per the DSP01 spec's contract.

#### Dependencies

Promoted from dev-dependencies to regular dependencies (so the
public `fft_via_runtime` can call them):

- `matrix-cpu`
- `matrix-runtime`
- `compute-ir`
- `executor-protocol`

The public `fft` and `ifft` functions still call the scalar
reference — a follow-up PR will swap them to use `fft_via_runtime`
once we have a story for batched inputs and `complex: true`.

### What this phase does NOT include

- Swapping public `fft` / `ifft` to use `fft_via_runtime`.  The
  current API takes a `complex: bool` parameter, and the
  matrix-ir-lowered path is real-only.  A follow-up phase will
  either add a complex-input graph variant or rework the API.
- Performance benchmarking.  Functional correctness is the bar
  for Phase 3b.iii; perf is Phase 5.
- Batched FFT (`[B, N]` input).  Phase 3b.ii / 3b.iii are
  single-channel only.

## 0.2.0 — 2026-05-13

### Added — DSP01 Phase 3b.ii (matrix-ir-lowered FFT graph builder)

Lands the first matrix-ir-lowered FFT primitive in the DSP layer.
The graph builds run on every backend the matrix execution layer
supports — today CPU; Metal/CUDA once they claim Slice / Concat in
their `supported_ops` bitsets.

- New module `radix2`:
  - `pub fn build_fft_graph(n: u32, direction: Direction) -> Result<Graph, FftError>`
    — builds a `matrix_ir::Graph` that computes the radix-2
    Cooley-Tukey FFT (or its inverse) of a power-of-2 length real
    signal.
  - Implementation uses only the documented MX01 vocabulary:
    `Slice`, `Concat`, `Reshape`, `Broadcast`, `Const`, `Mul`,
    `Sub`, `Add`.  No new IR ops are required.
- Re-exported as `dsp_fft::build_fft_graph`.
- The previously private `Direction` enum is now `pub Direction
  { Forward, Inverse }`.

### Algorithm summary

1. **Real → complex** via Reshape + Concat with a zeros Const.
2. **Bit-reversal permutation** as N width-1 Slice ops + one
   Concat axis=0.  Trivial no-op when N=2 (skipped at build time).
3. **log₂(N) butterfly stages**, each:
   - Reshape `[N, 2]` → `[N/full, full, 2]`.
   - Slice axis 1 to split each group into even / odd halves.
   - Complex-multiply odd × twiddle (Const, broadcast across
     groups) via four Muls + Sub/Add over the `[re, im]` axis.
   - Butterflies: `next_first = even + tw_odd`, `next_second =
     even - tw_odd`, then Concat back to `[N, 2]`.
4. **Inverse** divides every output by N via a broadcast scale.

### V1 limitations

- Power-of-2 N only (Bluestein is Phase 4 of DSP01).
- F32 dtype only.
- Single-channel: input `[N]` → output `[N, 2]`.
- Bit-reversal emits `O(N)` ops, capping practical N at maybe
  1024 before the graph gets unwieldy.  A future `Gather` op
  (matrix-ir V2 extension) will collapse this to one op.

### Tests

5 new unit tests in `radix2::tests`:

- `rejects_non_power_of_two` — error path for N = 3, 6.
- `rejects_too_small` — error path for N = 0, 1.
- `graph_validates_for_small_sizes` — builds and validates the
  graph for N ∈ {2, 4, 8, 16, 32}, both directions.  Verifies
  shape `[N, 2]` on the output.
- `n2_graph_has_expected_shape` — single-input single-output
  shape contract for N = 2.
- `bit_reverse_known_values` — helper sanity check.

All 17 unit tests pass (12 from Phase 2 + 5 new).

### What this phase does NOT include

End-to-end execution of the graph through `matrix-runtime` +
`matrix-cpu` against the scalar reference.  Deferred to **Phase
3b.iii** because the executor-protocol plumbing (AllocBuffer /
UploadBuffer / Dispatch / DownloadBuffer / FreeBuffer round-trip)
is its own significant chunk of code.  Phase 3b.ii ships the graph
build alone; Phase 3b.iii will:

1. Add an `execute_via_runtime(graph, &[f32]) -> Result<Vec<f32>, _>`
   helper.
2. Run the matrix-ir-lowered FFT for N ∈ {2, 4, 8, 16} and
   compare against `fft_scalar` within tolerance.
3. Replace the public `fft` / `ifft` bodies to call
   `execute_via_runtime` once verified.

The scalar reference (`fft_scalar` / `ifft_scalar`) stays as the
test oracle throughout.

### Dependencies

- `matrix-ir` — Graph + GraphBuilder + Slice/Concat ops.
- New dev-deps: `matrix-cpu`, `matrix-runtime`, `compute-ir`,
  `executor-protocol` (preparing for Phase 3b.iii's execution
  test harness; unused in 3b.ii).

No FFI, no `unsafe`.

## 0.1.0 — 2026-05-13

### Added — DSP01 Phase 2 (scalar reference FFT / IFFT)

Initial release.  The pure-Rust scalar oracle that future phases of
DSP01 will test against.

- `fft_scalar(&[f32]) -> Result<Vec<f32>, FftError>` — radix-2
  Cooley-Tukey on interleaved `[re, im]` buffers.  Power-of-two
  lengths only in Phase 2.
- `ifft_scalar(&[f32]) -> Result<Vec<f32>, FftError>` — inverse
  with "backward" normalization (output divided by `N`).
- `fft(&[f32], complex: bool) -> Result<ComplexTensor, _>` —
  high-level entry point; wraps real signals as complex with
  imag=0, or passes interleaved through.
- `ifft(&ComplexTensor) -> Result<ComplexTensor, _>` — mirror entry.

Both high-level functions forward to the scalar implementations in
Phase 2; Phase 3 will replace their bodies with matrix-ir graph
builders without changing the API.

### Algorithm

Standard decimation-in-time radix-2 FFT:

1. Bit-reverse the input in place.
2. For each stage `s = 1..=log2(N)`, run butterflies with twiddles
   `w_j = exp(±2πi · j / 2^s)`.
3. Inverse FFT uses positive-sign twiddles and divides every
   output element by `N` (backward normalization, matches numpy /
   scipy / MATLAB defaults).

### Numerical accuracy

Per the DSP01 spec contract:

- `ifft(fft(x))` round-trips within `1e-5` relative tolerance for
  `N ≤ 64K`, f32 dtype.
- Closed-form known vectors:
  - `fft(impulse) → [1, 1, …, 1]` exactly.
  - `fft(DC) → [N, 0, …, 0]` exactly.
  - `fft(cos(2π · k0 · n / N))` magnitude `N/2` at bins `k0` and
    `N - k0`.

### Tests

12 unit tests:

Error paths
- `fft_rejects_odd_interleaved_length`
- `fft_rejects_non_power_of_two_length`
- `fft_rejects_empty_signal`

Known vectors
- `fft_of_impulse_is_all_ones`
- `fft_of_dc_is_single_bin`
- `fft_of_pure_cosine_concentrates_in_two_bins`

Round-trip identities
- `round_trip_recovers_real_signal_n8`
- `round_trip_recovers_random_complex_n64` (xorshift32 pseudorandom)
- `round_trip_works_for_n_up_to_1024` (N ∈ {2, 4, 16, 64, 256, 1024})

Public API
- `public_fft_wraps_real_signal`
- `public_ifft_round_trips_via_complex_tensor`
- `public_fft_accepts_already_complex_input`

### Dependencies

- `dsp-complex` — for `ComplexTensor` in the public API.

No FFI, no `unsafe`, no external crates (no `proptest` yet — the
deterministic test set is enough for the scalar reference; Phase 3
will add proptest once the matrix-ir lowering is in).

### What this crate does NOT do (Phase 2)

- No matrix-ir lowering.  Phase 3 replaces `fft` / `ifft` bodies.
- No Bluestein / arbitrary lengths.  Phase 4.
- No `rfft` / `irfft`.  Phase 4.
- No GPU dispatch.  Comes for free when Phase 3 lands.
