# device-physics (C)

Semiconductor **device-physics primitives**, in pure ISO C17 — a faithful port
of the Rust `device-physics` crate. Closed-form textbook models (Sedra/Smith,
Pierret, Streetman) in SI units. **No libm**: `exp`/`ln`/`sqrt` are computed
from scratch.

## What it computes

| Function | Model |
|----------|-------|
| `dp_thermal_voltage` | V_T = kT/q |
| `dp_intrinsic_concentration` | n_i(T) = N_i(300)·(T/300)^1.5·exp(−Eg/2kT·(1−T/300)) |
| `dp_fermi_potential` | φ_F = ±V_T·ln(N/n_i) |
| PN junction | built-in voltage (ln), depletion width (sqrt), Shockley I_S, diode `I = I_S(exp(V/V_T)−1)` |
| MOSFET | oxide capacitance, flat-band & threshold voltage with body effect (sqrt) |

## API

```c
#include "device_physics.h"

double vt = dp_thermal_voltage(300.0);          /* ≈ 0.02585 V */

DpPNJunction j;
dp_pn_new(1e23, 1e22, 1e-8, 300.0, 1e-6, 1e-6, &j);
double vbi = dp_pn_built_in_voltage(&j);        /* ≈ 0.774 V   */
double i   = dp_pn_current(&j, 0.6);            /* Shockley diode current */

DpMOSFET m;
dp_mos_new(DP_NMOS, 130e-9, 1e-6, 2e-9, 1e24, -0.05, 0.0, 300.0, &m);
double vt0;
dp_mos_threshold_voltage(&m, 0.0, &vt0);        /* ≈ 1.228 V */
```

## Divergence from the Rust crate

Rust returns `Result<_, String>`; this port returns a `DpStatus`
(`DP_OK` / `DP_ERR_INVALID` / `DP_ERR_TEMP_RANGE` / `DP_ERR_BODY_FORWARD`) and
writes results through out-parameters. `exp`/`ln`/`sqrt` are computed without
`<math.h>`; reference values (captured from the real Rust crate via an oracle
run) match to ~1e-9 relative.

## Building

```sh
sh BUILD    # builds & runs the tests under every C compiler present
```

Pure ISO C17, no `<math.h>`. Builds clean under GCC, Clang, and MSVC with
`-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../iso-harness); the test suite also runs clean under
UndefinedBehaviorSanitizer.

## Where it fits

The device-operation companion to
[`fab-process-simulation`](../fab-process-simulation) (which models how the
device is *fabricated*): this models how the finished PN junction and MOSFET
*behave*.
