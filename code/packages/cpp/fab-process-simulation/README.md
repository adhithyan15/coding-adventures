# fab-process-simulation (C++)

A **1-D analytical CMOS process-flow simulator**, header-only in pure ISO C++17
(namespace `ca::fab_process_simulation`) — a faithful port of the Rust
`fab-process-simulation` crate. **No `<cmath>` / libm**.

## The models

| Step | Model |
|------|-------|
| Thermal oxidation | Deal-Grove quadratic growth law (`sqrt`) |
| Deposition | uniform film addition |
| Etching | layer-selective, top-down depth removal |
| Ion implantation | Gaussian profile from an SRIM range table (`exp`) |
| Diffusion | Fick's-law broadening (v0.1.0: samples preserved) |

A `CrossSection` is a `std::vector<Layer>` (top-to-bottom; `layers[0]` is the
top); each `Layer` has a `doping` map (species →
`std::vector<std::pair<double,double>>`). Every step returns a new
`CrossSection` by value.

## Usage

```cpp
#include "fab_process_simulation.hpp"
namespace fab = ca::fab_process_simulation;

fab::CrossSection si;
si.layers.emplace_back("Si", 500.0);

fab::CrossSection ox = fab::deal_grove_oxidation(si, 5.0);   // ~5.71 nm SiO2
fab::CrossSection imp = fab::implant(si, "B", 30.0, 1e15);   // boron profile
const auto& profile = imp.layers[0].doping.at("B");          // (depth, conc)…
```

## Divergence from the Rust crate

Rust returns `Result<_, String>`; this port throws `std::invalid_argument` with
the same message (non-positive time/thickness/dose, unknown species, no Si
layer). `sqrt`/`exp` are computed without `<cmath>`; reference values (captured
from the real Rust crate) match to within ~1e-6.

## Building

```sh
sh BUILD    # builds & runs the tests under every C++ compiler present
```

Pure ISO C++17, no `<cmath>`. Builds clean under GCC, Clang, and MSVC with
`-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../../c/iso-harness).
