# silicon-rust-go

Go package that exposes the Rust silicon simulation stack to Go programs
via CGo.  Wraps `silicon-rust-cgo`, a Rust cdylib that exports a plain C ABI.

## How it fits in the stack

```
Go caller
  ↓ import silicon_rust_go
silicon_rust_go (this package, CGo)
  ↓ import "C" → silicon_cgo.h
silicon-rust-cgo (Rust cdylib, plain C ABI)
  ↓ Rust function calls
device-physics   mosfet-models   fab-process-simulation
```

## Quick start

```go
import srg "github.com/adhithyan15/coding-adventures/code/packages/go/silicon-rust-go"

// Physical constants
fmt.Println(srg.KBoltzmann())        // 1.380649e-23 J/K
fmt.Println(srg.ThermalVoltage(300)) // 0.025852 V

// PN junction
vbi, _ := srg.PNJunctionBuiltInVoltage(1e23, 1e22, 300.0)
w,   _ := srg.PNJunctionDepletionWidth(1e23, 1e22, 300.0, 0.0)
is_, _ := srg.PNJunctionSaturationCurrent(1e23, 1e22, 1e-8, 300.0, 1e-6, 1e-6)
i,   _ := srg.PNJunctionCurrent(1e23, 1e22, 1e-8, 300.0, 1e-6, 1e-6, 0.6)

// MOSFET threshold voltage
vt, _ := srg.MosfetThresholdVoltage("NMOS", 130e-9, 1e-6, 2e-9, 1e24, -0.05, 0, 300, 0)

// Level-1 MOSFET DC operating point (default 130 nm NMOS params)
r, _ := srg.EvaluateLevel1Defaults(1.8, 1.8, 0.0, 300.15)
fmt.Println(r.Region) // "saturation"
fmt.Println(r.Id)     // drain current [A]

// Full Level-1 with explicit parameters
r, _ = srg.EvaluateLevel1(0.42, 220e-6, 0.05, 0.27, 0.84, 1e-6, 130e-9, 1.4,
    1.8, 1.8, 0.0, 300.15)

// Process simulation
cs, _ := srg.Deposit("", "Si", 500.0)            // Si substrate
cs, _ = srg.DealGroveOxidation(cs, 5.0)          // grow gate oxide
cs, _ = srg.Deposit(cs, "Poly", 50.0)             // deposit poly gate
// cs == "Poly:50.0|SiO2:...|Si:500.0"

cs, _ = srg.Implant(cs, "B", 30.0, 1e13)          // boron implant
cs, _ = srg.Diffuse(cs, 30.0)                     // 30-min anneal
cs, _ = srg.DiffuseWithTemp(cs, 30.0, 1000.0)     // at explicit temperature

rp, straggle, _ := srg.ImplantRange("B", 30.0)    // => 92.0, 38.0 nm
d := srg.DiffusivityCm2PerS("B", 1000.0)          // => ~1e-14 cm²/s
```

## Cross-section wire format

A `CrossSection` is serialised as a pipe-separated list of
`material:thickness_nm` pairs, ordered top-to-bottom:

```
""                               empty cross-section
"Si:500.0"                       bare silicon substrate, 500 nm
"SiO2:4.8|Si:500.0"             gate oxide on silicon
"Poly:50.0|SiO2:4.8|Si:500.0"  poly gate on gate oxide on silicon
```

Material names must not contain `|` or `:` — `Deposit`, `Etch`, and `Implant`
enforce this.

## API reference

### Physical constants

All return `float64`, no error.

| Function | Value | Unit |
|----------|-------|------|
| `KBoltzmann()` | 1.380649×10⁻²³ | J/K |
| `QElectron()` | 1.602176634×10⁻¹⁹ | C |
| `Eps0()` | 8.8541878×10⁻¹² | F/m |
| `EpsSi()` | 1.0359×10⁻¹⁰ | F/m |
| `EpsOx()` | 3.4531×10⁻¹¹ | F/m |
| `NiAt300K()` | 1×10¹⁶ | /m³ |
| `EgSiAt300K()` | 1.12 | eV |
| `MuN300K()` | 0.1350 | m²/V·s |
| `MuP300K()` | 0.0480 | m²/V·s |

### device-physics

```go
ThermalVoltage(tKelvin float64) float64
IntrinsicConcentration(tKelvin float64) (float64, error)
FermiPotential(nDoping float64, kind string, tKelvin float64) (float64, error)
    // kind: "p" or "n"
PNJunctionBuiltInVoltage(na, nd, t float64) (float64, error)
PNJunctionDepletionWidth(na, nd, t, vApplied float64) (float64, error)
PNJunctionSaturationCurrent(na, nd, a, t, tauN, tauP float64) (float64, error)
PNJunctionCurrent(na, nd, a, t, tauN, tauP, v float64) (float64, error)
MosfetThresholdVoltage(deviceType string, l, w, tOx, nBody, phiMs, qOx, t, vSb float64) (float64, error)
    // deviceType: "NMOS" or "PMOS"
```

### mosfet-models

```go
type MosResult struct {
    Id, Gm, Gds, Gmb         float64  // [A, S, S, S]
    Cgs, Cgd, Cgb, Cbs, Cbd float64  // [F]
    Region                   string   // "cutoff"|"subthreshold"|"triode"|"saturation"
}

EvaluateLevel1(vt0, kp, lambda, gamma, phi, w, l, nSub, vGs, vDs, vBs, t float64) (MosResult, error)
EvaluateLevel1Defaults(vGs, vDs, vBs, t float64) (MosResult, error)
```

### fab-process-simulation

```go
Deposit(cs, material string, thicknessNm float64) (string, error)
Etch(cs, target string, depthNm float64) (string, error)
Implant(cs, species string, energyKev, doseCm2 float64) (string, error)
Diffuse(cs string, timeMin float64) (string, error)
DiffuseWithTemp(cs string, timeMin, temperatureC float64) (string, error)
DealGroveOxidation(cs string, timeMin float64) (string, error)
DealGroveOxidationCustom(cs string, timeMin, aUm, bUm2PerHr float64) (string, error)
ImplantRange(species string, energyKev float64) (rp, straggle float64, err error)
DiffusivityCm2PerS(species string, temperatureC float64) float64
```

## Build

```bash
# Build the Rust cdylib first
cargo build -p silicon-rust-cgo --release

# Then run Go tests
go test ./...
```

## Platform notes

| Platform | Library | Notes |
|----------|---------|-------|
| Linux | `libsilicon_rust_cgo.so` | rpath set to target/release via LDFLAGS |
| macOS | `libsilicon_rust_cgo.dylib` | rpath set to target/release via LDFLAGS |
| Windows | `silicon_rust_cgo.dll` | MinGW CGo toolchain required |

## Testing

41 test functions covering constants, physics, process simulation, MosResult
struct fields, error cases, and the injection guard.
