# dsp-dct (C++)

**Discrete Cosine and Sine Transforms** — header-only, pure ISO C++17 in namespace `ca::dsp_dct`.

## What it does

Implements 1-D **DCT-I..IV** and **DST-I..IV**, plus separable 2-D **DCT/IDCT** and **DST/IDST**
under un-normalised (`DctNorm::None`) and orthonormal (`DctNorm::Ortho`) scaling. DCT-II and
DCT-III use `O(N log N)` Makhoul reduction over an internal iterative radix-2 + Bluestein FFT
in double precision.

## Usage

```cpp
#include "dsp_dct.hpp"
namespace dd = ca::dsp_dct;

std::vector<float> sig = {1.0f, 2.0f, 3.0f, 4.0f, 5.0f, 6.0f, 7.0f, 8.0f};
auto coeffs = dd::dct(sig, dd::DctType::II, dd::DctNorm::Ortho);
auto recon  = dd::idct(coeffs, dd::DctType::III, dd::DctNorm::Ortho);
```

## Building

```sh
sh BUILD
```
