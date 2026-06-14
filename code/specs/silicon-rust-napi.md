# silicon-rust-napi — Node.js N-API bindings for the silicon simulation stack

## Overview

`silicon-rust-napi` is a Node.js native addon that exposes `device-physics`,
`mosfet-models`, and `fab-process-simulation` to JavaScript and TypeScript.
It uses the zero-dependency `node-bridge` crate (raw N-API, no napi-rs) and
compiles to a `.node` file loadable with `require('silicon_rust_napi')`.

The API surface mirrors `silicon-rust-python` — the same 26 functions are
available, with JavaScript naming conventions (camelCase) instead of Python's
snake_case.

## Cross-section wire format

A `CrossSection` travels as a pipe-separated string of `material:thickness_nm`
pairs, identical to the Python binding.

```
""                               # empty cross-section
"Si:500.0"                       # bare silicon substrate
"SiO2:4.8|Si:500.0"             # gate oxide on silicon
"Poly:50.0|SiO2:4.8|Si:500.0"  # poly gate on gate oxide
```

## Public API

### Physical constants (no arguments)

| JS name        | Value            | Unit    | Description                  |
|---------------|------------------|---------|------------------------------|
| `kBoltzmann()`| 1.380649×10⁻²³  | J/K     | Boltzmann constant           |
| `qElectron()` | 1.602176634×10⁻¹⁹| C      | Elementary charge            |
| `eps0()`      | 8.8541878×10⁻¹² | F/m     | Vacuum permittivity          |
| `epsSi()`     | 1.0359×10⁻¹⁰    | F/m     | Silicon permittivity         |
| `epsOx()`     | 3.4531×10⁻¹¹    | F/m     | SiO₂ permittivity           |
| `niAt300k()`  | 1×10¹⁶          | /m³     | Intrinsic concentration at 300 K |
| `egSiAt300k()`| 1.12            | eV      | Silicon bandgap at 300 K     |
| `muN300k()`   | 0.1350          | m²/V/s  | Electron mobility at 300 K   |
| `muP300k()`   | 0.0480          | m²/V/s  | Hole mobility at 300 K       |

### Device-physics functions

```typescript
thermalVoltage(tKelvin: number): number
// V_T = kT/q [V]. At 300 K → ~0.02585 V.

intrinsicConcentration(tKelvin: number): number
// Intrinsic carrier concentration n_i(T) [/m³]. Throws below 100 K.

fermiPotential(nDoping: number, kind: 'p' | 'n', tKelvin: number): number
// Fermi potential φ_F [V]. +|φ_F| for p-type, −|φ_F| for n-type.

pnJunctionBuiltInVoltage(na: number, nd: number, t: number): number
// Built-in voltage V_bi [V] for an abrupt p-n junction.

pnJunctionDepletionWidth(na: number, nd: number, t: number, vApplied: number): number
// Total depletion-region width W [m]. Positive vApplied = forward bias.

pnJunctionSaturationCurrent(
  na: number, nd: number, a: number,
  t: number, tauN: number, tauP: number
): number
// Shockley saturation current I_S [A]. a = junction area [m²].

pnJunctionCurrent(
  na: number, nd: number, a: number,
  t: number, tauN: number, tauP: number, v: number
): number
// Shockley diode current I [A] at applied voltage v [V].

mosfetThresholdVoltage(
  deviceType: 'NMOS' | 'PMOS',
  l: number, w: number, tOx: number,
  nBody: number, phiMs: number, qOx: number,
  t: number, vSb: number
): number
// Threshold voltage V_t [V] with body effect. vSb ≥ 0.
```

### MOSFET Level-1 model

```typescript
interface MosResult {
  id: number;    // Drain current [A]
  gm: number;    // Transconductance [S]
  gds: number;   // Output conductance [S]
  gmb: number;   // Body transconductance [S]
  cgs: number;   // Gate-source capacitance [F]
  cgd: number;   // Gate-drain capacitance [F]
  cgb: number;   // Gate-body capacitance [F]
  cbs: number;   // Body-source capacitance [F]
  cbd: number;   // Body-drain capacitance [F]
  region: 'cutoff' | 'subthreshold' | 'triode' | 'saturation';
}

evaluateLevel1(
  vt0: number, kp: number, lambda: number, gamma: number, phi: number,
  w: number, l: number, nSub: number,
  vGs: number, vDs: number, vBs: number, t: number
): MosResult

evaluateLevel1Defaults(vGs: number, vDs: number, vBs: number, t: number): MosResult
// Same as evaluateLevel1 using default 130 nm NMOS parameter set.
```

### Fab-process simulation functions

```typescript
dealGroveOxidation(
  csStr: string, timeMin: number,
  aUm?: number, bUm2PerHr?: number
): string
// Grow thermal SiO₂ via Deal-Grove. Optional A/B use dry-O₂ 1000°C defaults.

deposit(csStr: string, material: string, thicknessNm: number): string
// Deposit a uniform film on top of the cross-section.

etch(csStr: string, targetMaterial: string, depthNm: number): string
// Remove depthNm nm of targetMaterial from the top.

implant(
  csStr: string, species: 'B' | 'P' | 'As' | 'BF2',
  energyKev: number, doseCm2: number
): string
// Add a Gaussian ion-implant profile to the topmost Si layer.

diffuse(csStr: string, timeMin: number, temperatureC?: number): string
// Broaden all Gaussian doping profiles (Fick's law). Default temp: 1000 °C.

interface ImplantRangeResult { rp: number; straggle: number; }
implantRange(species: string, energyKev: number): ImplantRangeResult
// Return { rp, straggle } (both in nm) from the SRIM table.

diffusivityCm2PerS(species: string, temperatureC: number): number
// Arrhenius-scaled diffusivity D(T) [cm²/s].
```

## Architecture

```
Node.js process
└── require('silicon_rust_napi')   → loads silicon_rust_napi.node (.so/.dylib/.dll)
    └── napi_register_module_v1()  → registers 26 JS functions via N-API
        ├── node-bridge             (raw extern "C" N-API declarations)
        ├── device-physics          (pure Rust physics)
        ├── mosfet-models           (Level-1 MOSFET model)
        └── fab-process-simulation  (process step library)
```

N-API symbols (`napi_create_function`, etc.) are resolved at `dlopen()` time:
- macOS: `-undefined dynamic_lookup` (emitted by `node-bridge`'s `build.rs`)
- Linux: ELF allows undefined symbols in shared objects by default
- Windows: requires `node.lib`; skipped in CI via `BUILD_windows`

## JavaScript usage example

```javascript
const srp = require('./silicon_rust_napi.node');

// Physical constants
console.log(srp.kBoltzmann());     // 1.380649e-23
console.log(srp.thermalVoltage(300)); // 0.025852

// PN junction
const vbi = srp.pnJunctionBuiltInVoltage(1e23, 1e22, 300);
const w   = srp.pnJunctionDepletionWidth(1e23, 1e22, 300, 0.0);

// Level-1 MOSFET
const r = srp.evaluateLevel1Defaults(1.8, 1.8, 0.0, 300.15);
console.log(r.region, r.id);  // "saturation" 1.23e-4

// Process simulation
let cs = srp.deposit("", "Si", 500.0);
cs = srp.dealGroveOxidation(cs, 5.0);
cs = srp.deposit(cs, "Poly", 50.0);
const { rp, straggle } = srp.implantRange("B", 30.0);
```

## Testing

`cargo test -p silicon-rust-napi` runs all 15 pure-Rust unit tests without
Node.js. N-API code is dead-stripped from the test binary.

## Files

| File                         | Purpose                                     |
|-----------------------------|---------------------------------------------|
| `Cargo.toml`                | crate metadata, cdylib + lib, dependencies  |
| `src/lib.rs`                | N-API callbacks + module registration       |
| `silicon_rust_napi.d.ts`    | TypeScript type declarations                |
| `BUILD` / `BUILD_windows`   | `cargo test` invocations for CI             |
| `README.md`                 | User-facing documentation                   |
| `CHANGELOG.md`              | Version history                             |
