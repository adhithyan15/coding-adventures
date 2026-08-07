# fab-process-simulation (C)

A **1-D analytical CMOS process-flow simulator**, in pure ISO C17 — a faithful
port of the Rust `fab-process-simulation` crate. **No libm**: the two
transcendentals are computed from scratch.

## The models

| Step | Model |
|------|-------|
| Thermal oxidation | Deal-Grove quadratic growth law (`sqrt`) |
| Deposition | uniform film addition |
| Etching | layer-selective, top-down depth removal |
| Ion implantation | Gaussian profile from an SRIM range table (`exp`) |
| Diffusion | Fick's-law broadening (v0.1.0: samples preserved) |

A `FabCrossSection` is a top-to-bottom stack of `FabLayer`s (`layers[0]` is the
top); each layer carries a per-species doping map of sampled
`(depth_nm, conc_per_cm3)` points. Every step returns a **new** cross-section —
inputs are never mutated — so results are deep-copied; release with
`fab_cross_section_free`.

## API

```c
#include "fab_process_simulation.h"

FabCrossSection si; fab_cross_section_init(&si);
fab_cross_section_add_layer(&si, "Si", 500.0);   /* bare substrate */

FabCrossSection ox;
fab_deal_grove_oxidation(&si, 5.0, 0, 0, 0, 0, &ox); /* grow ~5.71 nm SiO2 */

FabCrossSection imp;
fab_implant(&si, "B", 30.0, 1e15, &imp);         /* Gaussian boron profile */
const FabDoping *p = fab_layer_doping(fab_layer_at(&imp, 0), "B");
/* p->samples[i] = (depth_nm, conc_per_cm3) */

fab_cross_section_free(&imp);
fab_cross_section_free(&ox);
fab_cross_section_free(&si);
```

`fab_implant_range` exposes the SRIM lookup with interpolation/extrapolation;
`fab_diffusivity_cm2_per_s` the Arrhenius (T²-scaled) diffusivity.

## Divergence from the Rust crate

Rust returns `Result<_, String>`; this port returns a `FabStatus`
(`FAB_OK` / `FAB_ERR_INVALID` / `FAB_ERR_UNKNOWN_SPECIES` / `FAB_ERR_NO_SI` /
`FAB_ERR_NOMEM`) and writes the new cross-section through an out-parameter. The
`sqrt`/`exp` are computed without `<math.h>`; reference values match the Rust
f64 models (captured via an oracle run) to within ~1e-6.

## Building

```sh
sh BUILD    # builds & runs the tests under every C compiler present
```

Pure ISO C17, no `<math.h>`. Builds clean under GCC, Clang, and MSVC with
`-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../iso-harness); the test suite also runs clean under
AddressSanitizer + UndefinedBehaviorSanitizer.
