# silicon-rust-napi

Node.js N-API addon that exposes the Rust silicon simulation stack —
`device-physics`, `mosfet-models`, and `fab-process-simulation` — to
JavaScript and TypeScript.

Uses the zero-dependency `node-bridge` crate (raw N-API via `extern "C"`,
no napi-rs, no bindgen, no Node.js headers at build time).  Compiles to a
`.node` file loadable with `require('silicon_rust_napi.node')`.

## How it fits in the stack

```
silicon-rust-napi (this package)
├── node-bridge        — raw N-API declarations, zero deps
├── device-physics     — physical constants + semiconductor equations
├── mosfet-models      — SPICE Level-1 MOSFET model
└── fab-process-simulation — thin-film deposition, etch, oxidation, implant
```

## JavaScript usage

```javascript
const srp = require('./silicon_rust_napi.node');

// Physical constants
console.log(srp.kBoltzmann());      // 1.380649e-23 J/K
console.log(srp.thermalVoltage(300)); // 0.025852 V

// PN junction physics
const vbi = srp.pnJunctionBuiltInVoltage(1e23, 1e22, 300);
const w   = srp.pnJunctionDepletionWidth(1e23, 1e22, 300, 0.0);
const is_ = srp.pnJunctionSaturationCurrent(1e23, 1e22, 1e-8, 300, 1e-6, 1e-6);
const i   = srp.pnJunctionCurrent(1e23, 1e22, 1e-8, 300, 1e-6, 1e-6, 0.6);

// MOSFET threshold voltage
const vt = srp.mosfetThresholdVoltage('NMOS', 130e-9, 1e-6, 2e-9, 1e24, -0.05, 0, 300, 0);

// Level-1 MOSFET DC operating point (default 130 nm NMOS params)
const r = srp.evaluateLevel1Defaults(1.8, 1.8, 0.0, 300.15);
console.log(r.region);  // "saturation"
console.log(r.id);      // drain current [A]
console.log(r.gm);      // transconductance [S]

// Full Level-1 with explicit parameters
const r2 = srp.evaluateLevel1(
  0.42,   // vt0 [V]
  220e-6, // kp [A/V²]
  0.05,   // lambda [1/V]
  0.27,   // gamma [√V]
  0.84,   // phi [V]
  1e-6,   // W [m]
  130e-9, // L [m]
  1.4,    // n_sub [×10²⁴/m³]
  1.8,    // vGs [V]
  1.8,    // vDs [V]
  0.0,    // vBs [V]
  300.15  // T [K]
);

// Process simulation
let cs = srp.deposit("", "Si", 500.0);       // Si substrate
cs = srp.dealGroveOxidation(cs, 5.0);        // grow SiO₂ gate oxide
cs = srp.deposit(cs, "Poly", 50.0);          // deposit poly gate
console.log(cs);  // "Poly:50.0|SiO2:...|Si:500.0"

cs = srp.implant(cs, "B", 30.0, 1e13);       // boron source/drain
cs = srp.diffuse(cs, 30.0, 1000.0);          // 30-min anneal at 1000°C

const { rp, straggle } = srp.implantRange("B", 30.0);  // 92 nm, 38 nm
const d = srp.diffusivityCm2PerS("B", 1000.0);          // 1e-14 cm²/s
```

## TypeScript

A full type declaration file is provided at `silicon_rust_napi.d.ts`.

```typescript
import type { MosResult, ImplantRangeResult } from './silicon_rust_napi';
const srp: typeof import('./silicon_rust_napi') = require('./silicon_rust_napi.node');

const r: MosResult = srp.evaluateLevel1Defaults(1.8, 1.8, 0.0, 300.15);
```

## Cross-section wire format

A `CrossSection` is serialised as a pipe-separated list of
`material:thickness_nm` pairs, ordered top-to-bottom:

```
""                               # empty cross-section
"Si:500.0"                       # bare silicon substrate, 500 nm thick
"SiO2:4.8|Si:500.0"             # gate oxide on silicon
"Poly:50.0|SiO2:4.8|Si:500.0"  # poly gate on gate oxide on silicon
```

Material names must not contain `|` or `:` — `deposit()` enforces this to
prevent wire-format injection.

## API reference

### Constants (no arguments)

| Function      | Value             | Unit    | Description                    |
|--------------|-------------------|---------|--------------------------------|
| `kBoltzmann()`| 1.380649×10⁻²³   | J/K     | Boltzmann constant             |
| `qElectron()` | 1.602176634×10⁻¹⁹| C       | Elementary charge              |
| `eps0()`      | 8.8541878×10⁻¹²  | F/m     | Vacuum permittivity            |
| `epsSi()`     | 1.0359×10⁻¹⁰     | F/m     | Silicon permittivity           |
| `epsOx()`     | 3.4531×10⁻¹¹     | F/m     | SiO₂ permittivity             |
| `niAt300k()`  | 1×10¹⁶           | /m³     | Intrinsic concentration at 300 K |
| `egSiAt300k()`| 1.12             | eV      | Silicon bandgap at 300 K       |
| `muN300k()`   | 0.1350           | m²/V·s  | Electron mobility at 300 K     |
| `muP300k()`   | 0.0480           | m²/V·s  | Hole mobility at 300 K         |

### device-physics

```
thermalVoltage(tKelvin)                         → number [V]
intrinsicConcentration(tKelvin)                 → number [/m³]
fermiPotential(nDoping, kind, tKelvin)          → number [V]
pnJunctionBuiltInVoltage(na, nd, t)             → number [V]
pnJunctionDepletionWidth(na, nd, t, vApplied)   → number [m]
pnJunctionSaturationCurrent(na,nd,a,t,tauN,tauP)→ number [A]
pnJunctionCurrent(na,nd,a,t,tauN,tauP,v)       → number [A]
mosfetThresholdVoltage(deviceType,l,w,tOx,nBody,phiMs,qOx,t,vSb) → number [V]
```

### mosfet-models

```
evaluateLevel1(vt0,kp,lambda,gamma,phi,w,l,nSub,vGs,vDs,vBs,t) → MosResult
evaluateLevel1Defaults(vGs, vDs, vBs, t)                       → MosResult

MosResult: { id, gm, gds, gmb, cgs, cgd, cgb, cbs, cbd [all numbers],
             region: 'cutoff'|'subthreshold'|'triode'|'saturation' }
```

### fab-process-simulation

```
dealGroveOxidation(csStr, timeMin[, aUm, bUm2PerHr]) → string
deposit(csStr, material, thicknessNm)                 → string
etch(csStr, targetMaterial, depthNm)                  → string
implant(csStr, species, energyKev, doseCm2)            → string
diffuse(csStr, timeMin[, temperatureC])               → string
implantRange(species, energyKev)                      → { rp, straggle } [nm]
diffusivityCm2PerS(species, temperatureC)             → number [cm²/s]
```

## Building

```bash
# Install Rust if needed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build the .node addon (output: target/debug/silicon_rust_napi.so/.dylib/.dll)
cargo build -p silicon-rust-napi

# Run pure-Rust unit tests (no Node.js required)
cargo test --lib -p silicon-rust-napi
```

## Platform notes

| Platform | cdylib build | Tests |
|----------|-------------|-------|
| Linux    | ✓ ELF allows undefined symbols | ✓ |
| macOS    | ✓ `-undefined dynamic_lookup` via node-bridge | ✓ |
| Windows  | ⚠ requires node.lib (skipped in CI) | ✓ via `--lib` |

## Testing

All 18 unit tests are pure Rust and run without Node.js:

```
running 18 tests
test tests::cs_wire_empty_roundtrip           ... ok
test tests::cs_wire_single_layer              ... ok
test tests::cs_wire_multi_layer_roundtrip     ... ok
test tests::cs_wire_malformed_entry_skipped   ... ok
test tests::validate_material_rejects_delimiters ... ok
test tests::thermal_voltage_at_300k           ... ok
test tests::intrinsic_concentration_valid     ... ok
test tests::intrinsic_concentration_below_100k_fails ... ok
test tests::pn_junction_built_in_voltage_typical ... ok
test tests::mosfet_threshold_voltage_nmos     ... ok
test tests::eval_level1_saturation            ... ok
test tests::eval_level1_cutoff                ... ok
test tests::deal_grove_adds_sio2_layer        ... ok
test tests::deposit_prepends_layer            ... ok
test tests::etch_removes_top_layer            ... ok
test tests::implant_range_boron_30kev         ... ok
test tests::diffusivity_boron_1000c           ... ok
test tests::wire_roundtrip_after_gate_stack   ... ok
```
