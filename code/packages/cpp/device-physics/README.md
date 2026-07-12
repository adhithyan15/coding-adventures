# device-physics (C++)

Semiconductor **device-physics primitives**, header-only in pure ISO C++17
(namespace `ca::device_physics`) — a faithful port of the Rust `device-physics`
crate. Closed-form textbook models in SI units. **No `<cmath>` / libm**:
`exp`/`ln`/`sqrt` are computed from scratch.

## What it computes

`thermal_voltage`, `intrinsic_concentration` (exp), `fermi_potential` (ln); a
`PNJunction` (built-in voltage, depletion width, Shockley saturation & diode
current) and a `MOSFETParams` (oxide capacitance, flat-band & threshold voltage
with body effect).

## Usage

```cpp
#include "device_physics.hpp"
namespace dp = ca::device_physics;

double vt = dp::thermal_voltage(300.0);          // ≈ 0.02585 V

dp::PNJunction j(1e23, 1e22, 1e-8, 300.0, 1e-6, 1e-6);
double vbi = j.built_in_voltage();               // ≈ 0.774 V
double i   = j.current(0.6);                     // Shockley diode current

dp::MOSFETParams m(dp::MosType::NMOS, 130e-9, 1e-6, 2e-9, 1e24, -0.05, 0.0,
                   300.0);
double vt0 = m.threshold_voltage(0.0);           // ≈ 1.228 V
```

## Divergence from the Rust crate

Rust returns `Result<_, String>`; this port throws `std::invalid_argument`
(bad doping / area / dimensions / kind) or `std::domain_error` (temperature or
body-bias out of range). Reference values (captured from the real Rust crate)
match to ~1e-9 relative.

## Building

```sh
sh BUILD    # builds & runs the tests under every C++ compiler present
```

Pure ISO C++17, no `<cmath>`. Builds clean under GCC, Clang, and MSVC with
`-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../../c/iso-harness).
