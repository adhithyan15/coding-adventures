# dsp-wavelets (C++)

**Discrete Wavelet Transforms** — header-only, ISO C++17. A faithful port of
the Rust [`dsp-wavelets`](../../rust/dsp-wavelets) crate, in namespace
`ca::dsp_wavelets`.

## What it does

Implements the Discrete Wavelet Transform (DWT) and its inverse via the
**Mallat pyramid algorithm**, in one and two dimensions, for the orthogonal
wavelet families **Haar**, **Daubechies** (Db2/4/6/8), **Symlets** (Sym4), and
**Coiflets** (Coif1).

One forward step is two FIR filter passes (a lowpass `h` and a highpass `g`)
followed by downsample-by-2; `levels` of DWT applies the pair recursively to
the approximation, producing the flattened layout `[cA_J | cD_J | ... | cD_1]`.
The 2-D transform is separable (row-then-column) and yields the `LL/HL/LH/HH`
sub-bands per level.

## API

```cpp
#include "dsp_wavelets.hpp"
namespace wv = ca::dsp_wavelets;

std::vector<float> signal(32);
// ... fill signal ...
std::vector<float> coeffs = wv::dwt_1d(signal, wv::Wavelet::haar(), 3, wv::Boundary::Periodic);
std::vector<float> recon  = wv::idwt_1d(coeffs, wv::Wavelet::haar(), 3,
                                        wv::Boundary::Periodic, signal.size());
// recon reconstructs signal within ~1e-4
```

Signals and coefficient buffers are `std::vector<float>` (mirroring the crate's
`Vec<f32>`). `slice_level` returns a borrowed `FloatView`. Where the Rust crate
returns `Result`, this port throws a `WaveletError` carrying an `Error` code.
Wavelets are selected with the `Wavelet::haar()` / `daubechies(n)` / `symlets(n)`
/ `coiflets(n)` factories.

## Building

```sh
sh BUILD          # POSIX: g++ and/or clang++, via the shared iso-harness
```

Each compiler prints `N checks, 0 failed`. Verified clean under ASan + UBSan.
