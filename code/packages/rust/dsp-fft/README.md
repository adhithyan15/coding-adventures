# dsp-fft

Pure-Rust FFT / IFFT for the DSP layer on interleaved `[re, im]`
f32 buffers — same layout as
[`dsp-complex::ComplexTensor`](../dsp-complex/README.md).

As of **0.6.0 (DSP01 Phase 4b)** the crate offers the full
NumPy-style FFT surface:

- `fft` / `ifft` accept **arbitrary `N ≥ 1`** (radix-2 for
  power-of-two, scalar Bluestein for everything else).
- Power-of-two real inputs run end-to-end on the matrix
  execution layer (`matrix-runtime` + `matrix-cpu`, GPU once
  Metal / CUDA claim Slice + Concat) via the radix-2 graph.
- `rfft` / `irfft` (new in 0.6.0) exploit conjugate symmetry of
  real-input spectra to return only `⌊N / 2⌋ + 1` bins.
- `fft_scalar` / `ifft_scalar` (radix-2, pow2-only) and
  `bluestein_scalar` remain available as the canonical oracles.

```rust
use dsp_fft::{fft, ifft};
use dsp_complex::ComplexTensor;

let real = vec![1.0_f32, 0.0, 0.0, 0.0];   // impulse
let spectrum: ComplexTensor = fft(&real, /* complex = */ false).unwrap();
let recovered: ComplexTensor = ifft(&spectrum).unwrap();
assert_eq!(recovered.real(), real);
```

## Algorithm

Standard decimation-in-time radix-2 FFT:

1. Bit-reverse the input in place.
2. For each stage `s = 1..=log₂(N)`, run butterflies with twiddles
   `w_j = exp(±2πi · j / 2^s)`.
3. Inverse FFT uses positive-sign twiddles and divides every output
   element by `N` ("backward" normalization — matches numpy / scipy
   / MATLAB defaults, per DSP01 spec).

## Numerical accuracy

Per the DSP01 spec contract:

- `ifft(fft(x))` round-trips within `1e-5` relative tolerance for
  `N ≤ 64K`, f32 dtype.
- Closed-form known vectors:
  - `fft(impulse) → [1, 1, …, 1]`.
  - `fft(DC) → [N, 0, …, 0]`.
  - `fft(cos(2π · k₀ · n / N))` concentrates magnitude `N/2` at bins
    `k₀` and `N - k₀`.

The test suite exercises all of these plus a deterministic
pseudorandom round-trip up to `N = 1024`.

## Phase scope

| Phase  | Lands                                                | Status |
| ------ | ---------------------------------------------------- | ------ |
| 2      | Scalar reference                                     | landed (0.1.0) |
| 3a     | `Op::Slice` in matrix-ir                              | landed |
| 3b.i   | `Op::Concat` in matrix-ir                             | landed |
| 3b.ii  | matrix-ir-lowered FFT graph builder                   | landed (0.2.0) |
| 3b.iii | End-to-end execution via `matrix-runtime` + `matrix-cpu` | landed (0.3.0) |
| 3b.iv  | Public `fft()` real-input path routes through `fft_via_runtime` | landed (0.4.0) |
| 4a     | Scalar Bluestein for arbitrary lengths               | landed (0.5.0) |
| **4b** | **`rfft` / `irfft` (half-spectrum API)**             | **this PR (0.6.0)** |
| 4c     | Matrix-ir-lowered Bluestein                          | pending |
| 5      | MX05 specialised emitters (folded twiddles)          | pending |

## Matrix-IR graph build (Phase 3b.ii)

`build_fft_graph(n, Direction::Forward)` returns a
`matrix_ir::Graph` that computes the radix-2 Cooley-Tukey FFT of a
length-`n` real signal entirely through generic tensor ops.  Same
graph runs on every backend MX supports (currently CPU; Metal /
CUDA once they claim Slice / Concat).

Inputs:
- One input of shape `[n]`, dtype F32.

Outputs:
- One output of shape `[n, 2]`, dtype F32.  Interleaved `[re, im]`
  complex spectrum.

Constraints (validator-enforced):
- `n ≥ 2`.
- `n` is a power of two.

## End-to-end execution (Phase 3b.iii)

`fft_via_runtime(signal, direction)` builds the graph, plans it
through `matrix-runtime::Runtime`, dispatches via a fresh
`matrix-cpu::CpuExecutor`, downloads the output buffer, and returns
the interleaved `[re, im, ..., re, im]` spectrum:

```rust
use dsp_fft::{fft_via_runtime, Direction};

let signal = vec![1.0_f32, 0.0, 0.0, 0.0];   // impulse
let spectrum = fft_via_runtime(&signal, Direction::Forward)?;
// spectrum is [1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0]
// (every bin = 1.0 in real part, 0.0 in imag)
```

When `matrix-metal` and `matrix-cuda` claim Slice + Concat in their
`supported_ops` bitsets, the same call lifts to GPU automatically.
The Phase 3b.iii test suite verifies the matrix-ir-lowered FFT
matches the scalar oracle within 1e-4 relative tolerance for
N ∈ {2, 4, 8, 16} — including a `ifft(fft(x)) ≈ x` round-trip.

## Public-API routing (as of Phase 4a / 0.5.0)

| Call                                       | Path                          |
| ------------------------------------------ | ----------------------------- |
| `fft(&real, false)`, `N ≥ 2` pow2          | matrix-ir → `fft_via_runtime` |
| `fft(&real, false)`, `N ≥ 2` non-pow2      | scalar Bluestein              |
| `fft(&real, false)`, `N = 1`               | scalar (identity)             |
| `fft(&interleaved, true)`, `N` pow2        | scalar radix-2                |
| `fft(&interleaved, true)`, `N` non-pow2    | scalar Bluestein              |
| `ifft(&spectrum)`, `N` pow2                | scalar radix-2                |
| `ifft(&spectrum)`, `N` non-pow2            | scalar Bluestein              |

The complex-input and `ifft` paths stay scalar because the
matrix-ir graph builder's input shape is `[N]` real (it Concats
a zero imaginary lane internally).  Phase 4c will lift the
Bluestein path onto matrix-ir; a later phase will add a
complex-input radix-2 graph variant.  Until then `fft_scalar` /
`ifft_scalar` remain the canonical radix-2 oracle, and
`bluestein_scalar` is the canonical arbitrary-N oracle.

## Bluestein's algorithm (Phase 4a, scalar)

For non-power-of-two `N`, `bluestein_scalar(signal, direction)`
computes the DFT as a length-`M` linear convolution, where
`M = next_pow2(2N - 1)`.  The three internal FFTs all run on
the radix-2 path (`fft_scalar`).  The chirp construction uses
`k² mod 2N` reduction to keep the floating-point exponent
bounded for large `N`.

```rust
use dsp_fft::{bluestein_scalar, Direction};

// 7-point DFT — prime length, can't use radix-2 directly.
let signal = vec![1.0_f32, 0.0, 2.0, 0.0, 3.0, 0.0, 4.0, 0.0,
                  5.0, 0.0, 6.0, 0.0, 7.0, 0.0];
let spectrum = bluestein_scalar(&signal, Direction::Forward).unwrap();
```

## Tests

`cargo test -p dsp-fft` — 63 unit tests:

- 12 from Phase 2 — error paths, closed-form known vectors, scalar radix-2 round-trips.
- 5 from Phase 3b.ii — graph build / validation.
- 5 from Phase 3b.iii — `fft_via_runtime` matches scalar for N ∈ {2, 4, 8, 16}.
- 8 from Phase 3b.iv — public API routes through the runtime.
- 15 from Phase 4a — Bluestein: error paths, naive-DFT
  cross-checks, round-trip sweep 1..=32.
- 18 from Phase 4b (this release) — `rfft` / `irfft`: error
  paths, closed-form impulse / DC / single-bin at N = 8, 16,
  round-trip at N ∈ {1, 8, 16, 64, 3, 7, 12} plus a 1..=20
  sweep, and 3 public-API tests.
