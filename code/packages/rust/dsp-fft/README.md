# dsp-fft

Pure-Rust radix-2 Cooley-Tukey FFT / IFFT for the DSP layer on
interleaved `[re, im]` f32 buffers — same layout as
[`dsp-complex::ComplexTensor`](../dsp-complex/README.md).

As of **0.4.0 (DSP01 Phase 3b.iv)** the public `fft()` for
real-valued, power-of-two inputs runs end-to-end through
`matrix-runtime` + `matrix-cpu` — i.e. the FFT actually executes
on the matrix execution layer.  Complex inputs and `ifft` still
call the scalar reference (the matrix-ir graph builder is
real-input only for now).  The scalar reference stays available
as `fft_scalar` / `ifft_scalar`; it's also the oracle every other
path is tested against.

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
| **3b.iv** | **Public `fft()` real-input path routes through `fft_via_runtime`** | **this PR (0.4.0)** |
| 4      | Bluestein for arbitrary lengths; rfft / irfft        | pending |
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

## Public-API routing (Phase 3b.iv)

| Call                                  | Path                        |
| ------------------------------------- | --------------------------- |
| `fft(&real, false)`, `N ≥ 2` pow2     | matrix-ir → `fft_via_runtime` |
| `fft(&real, false)`, `N = 1`          | scalar (degenerate)          |
| `fft(&interleaved, true)`             | scalar                       |
| `ifft(&spectrum)`                     | scalar                       |

The complex-input and `ifft` paths stay scalar because the
matrix-ir graph builder's input shape is `[N]` real (it Concats
a zero imaginary lane internally).  A future phase will add a
complex-input graph variant; until then `fft_scalar` /
`ifft_scalar` remain the canonical entry points for those cases
and the test oracle for everything.

## Tests

`cargo test -p dsp-fft` — 28 unit tests:

- 12 from Phase 2 — error paths, closed-form known vectors, scalar round-trips.
- 5 from Phase 3b.ii — graph build / validation.
- 5 from Phase 3b.iii — `fft_via_runtime` matches scalar for N ∈ {2, 4, 8, 16}.
- 6 from Phase 3b.iv (this release) — public API routes through
  the runtime, complex-input contract preserved bit-for-bit
  against scalar.
