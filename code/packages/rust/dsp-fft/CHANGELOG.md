# Changelog — dsp-fft

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
