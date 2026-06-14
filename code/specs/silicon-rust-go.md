# Specification — silicon-rust-go

CGo bindings for the Rust silicon simulation stack.  Exposes
`device-physics`, `mosfet-models`, and `fab-process-simulation` to Go
programs through a plain C ABI (`silicon-rust-cgo` cdylib) and a thin
`silicon_rust_go` Go package that wraps it with `import "C"`.

## Motivation

All other language stacks in this repo (Python, JS/TS, Ruby) have bindings
for the silicon simulation stack.  Go is the primary build-tool language in
this repo and deserves first-class access to the same physics primitives.

## Architecture

```
Go caller
  ↓ import "silicon_rust_go"
silicon_rust_go  (Go package, CGo)
  ↓ import "C" → silicon_cgo.h
silicon-rust-cgo  (Rust cdylib, plain C ABI)
  ↓ Rust function calls
device-physics   mosfet-models   fab-process-simulation
```

No Python/Ruby/NAPI overhead.  The cdylib exports only plain C functions
and one C struct, which CGo calls directly.

## File Layout

```
code/specs/silicon-rust-go.md       ← this file

code/packages/rust/silicon-rust-cgo/
  Cargo.toml
  build.rs
  include/
    silicon_cgo.h                   ← C header shared with CGo
  src/lib.rs
  BUILD
  BUILD_windows
  README.md
  CHANGELOG.md

code/packages/go/silicon-rust-go/
  go.mod
  silicon_rust_go.go                ← package + CGo directives
  silicon_rust_go_test.go           ← go test suite
  BUILD
  BUILD_windows
  README.md
  CHANGELOG.md
```

## C API (`silicon_cgo.h`)

### Calling conventions

* **Infallible functions** return `double` directly.
* **Fallible functions** return `int` (0 = success, -1 = error) and write
  the result into an out-pointer.  On error they write a nul-terminated
  UTF-8 message into a caller-supplied `err[err_cap]` buffer.
* **String-returning functions** (cross-section wire format) write into a
  caller-supplied `out[out_cap]` buffer, nul-terminated.

### Physical constants (infallible)

```c
double silicon_k_boltzmann(void);      // 1.380649e-23 J/K
double silicon_q_electron(void);       // 1.602176634e-19 C
double silicon_eps0(void);             // 8.8541878e-12 F/m
double silicon_eps_si(void);           // 1.0359e-10 F/m
double silicon_eps_ox(void);           // 3.4531e-11 F/m
double silicon_ni_at_300k(void);       // 1e16 /m³
double silicon_eg_si_at_300k(void);    // 1.12 eV
double silicon_mu_n_300k(void);        // 0.1350 m²/V·s
double silicon_mu_p_300k(void);        // 0.0480 m²/V·s
```

### device-physics

```c
// Infallible
double silicon_thermal_voltage(double t_kelvin);

// Fallible (return 0/−1)
int silicon_intrinsic_concentration(double t_kelvin, double *out,
    char *err, size_t err_cap);
int silicon_fermi_potential(double n_doping, const char *kind,
    double t_kelvin, double *out, char *err, size_t err_cap);
    // kind: "p" or "n"
int silicon_pn_junction_built_in_voltage(double na, double nd, double t,
    double *out, char *err, size_t err_cap);
int silicon_pn_junction_depletion_width(double na, double nd, double t,
    double v_applied, double *out, char *err, size_t err_cap);
int silicon_pn_junction_saturation_current(double na, double nd, double a,
    double t, double tau_n, double tau_p, double *out,
    char *err, size_t err_cap);
int silicon_pn_junction_current(double na, double nd, double a, double t,
    double tau_n, double tau_p, double v, double *out,
    char *err, size_t err_cap);
int silicon_mosfet_threshold_voltage(const char *device_type, double l,
    double w, double t_ox, double n_body, double phi_ms, double q_ox,
    double t, double v_sb, double *out, char *err, size_t err_cap);
    // device_type: "NMOS" or "PMOS"
```

### mosfet-models

```c
typedef struct {
    double id, gm, gds, gmb, cgs, cgd, cgb, cbs, cbd;
    char   region[32];   // "cutoff"|"subthreshold"|"triode"|"saturation"
} SiliconMosResult;

int silicon_evaluate_level1(
    double vt0, double kp, double lambda, double gamma, double phi,
    double w, double l, double n_sub,
    double v_gs, double v_ds, double v_bs, double t,
    SiliconMosResult *out, char *err, size_t err_cap);

int silicon_evaluate_level1_defaults(
    double v_gs, double v_ds, double v_bs, double t,
    SiliconMosResult *out, char *err, size_t err_cap);
```

`evaluate_level1_defaults` uses 130 nm NMOS default parameters
(`Level1Params::default()`).

### fab-process-simulation

Cross-section wire format: pipe-separated `material:thickness_nm` pairs
ordered top-to-bottom (same as Python/Ruby/NAPI siblings).

```c
int silicon_deposit(const char *cs, const char *material,
    double thickness_nm,
    char *out, size_t out_cap, char *err, size_t err_cap);

int silicon_etch(const char *cs, const char *target, double depth_nm,
    char *out, size_t out_cap, char *err, size_t err_cap);

int silicon_implant(const char *cs, const char *species,
    double energy_kev, double dose_cm2,
    char *out, size_t out_cap, char *err, size_t err_cap);

int silicon_diffuse(const char *cs, double time_min,
    char *out, size_t out_cap, char *err, size_t err_cap);

int silicon_diffuse_with_temp(const char *cs, double time_min,
    double temperature_c,
    char *out, size_t out_cap, char *err, size_t err_cap);

int silicon_deal_grove_oxidation(const char *cs, double time_min,
    char *out, size_t out_cap, char *err, size_t err_cap);

int silicon_deal_grove_oxidation_custom(const char *cs, double time_min,
    double a_um, double b_um2_per_hr,
    char *out, size_t out_cap, char *err, size_t err_cap);

int silicon_implant_range(const char *species, double energy_kev,
    double *rp, double *straggle, char *err, size_t err_cap);

double silicon_diffusivity_cm2_per_s(const char *species,
    double temperature_c);   // infallible
```

`deposit` validates material names: `|` and `:` are rejected (wire-format
injection guard), as are `target` in `etch` and `species` in `implant`.

## Go API (`silicon_rust_go` package)

### Types

```go
type MosResult struct {
    Id, Gm, Gds, Gmb, Cgs, Cgd, Cgb, Cbs, Cbd float64
    Region string
}
```

### Constants

All return `float64`, no error.

```go
func KBoltzmann() float64
func QElectron() float64
func Eps0() float64
func EpsSi() float64
func EpsOx() float64
func NiAt300K() float64
func EgSiAt300K() float64
func MuN300K() float64
func MuP300K() float64
```

### device-physics

```go
func ThermalVoltage(tKelvin float64) float64
func IntrinsicConcentration(tKelvin float64) (float64, error)
func FermiPotential(nDoping float64, kind string, tKelvin float64) (float64, error)
func PNJunctionBuiltInVoltage(na, nd, t float64) (float64, error)
func PNJunctionDepletionWidth(na, nd, t, vApplied float64) (float64, error)
func PNJunctionSaturationCurrent(na, nd, a, t, tauN, tauP float64) (float64, error)
func PNJunctionCurrent(na, nd, a, t, tauN, tauP, v float64) (float64, error)
func MosfetThresholdVoltage(deviceType string, l, w, tOx, nBody, phiMs, qOx, t, vSb float64) (float64, error)
```

### mosfet-models

```go
func EvaluateLevel1(vt0, kp, lambda, gamma, phi, w, l, nSub, vGs, vDs, vBs, t float64) (MosResult, error)
func EvaluateLevel1Defaults(vGs, vDs, vBs, t float64) (MosResult, error)
```

### fab-process-simulation

```go
func Deposit(cs, material string, thicknessNm float64) (string, error)
func Etch(cs, target string, depthNm float64) (string, error)
func Implant(cs, species string, energyKev, doseCm2 float64) (string, error)
func Diffuse(cs string, timeMin float64) (string, error)
func DiffuseWithTemp(cs string, timeMin, temperatureC float64) (string, error)
func DealGroveOxidation(cs string, timeMin float64) (string, error)
func DealGroveOxidationCustom(cs string, timeMin, aUm, bUm2PerHr float64) (string, error)
func ImplantRange(species string, energyKev float64) (rp, straggle float64, err error)
func DiffusivityCm2PerS(species string, temperatureC float64) float64
```

## Wire format injection guard

`Deposit`, `Etch`, and `Implant` validate user-supplied material/species
names on the Rust side.  Names containing `|` or `:` are rejected with an
error to prevent corruption of the pipe-separated wire format.

## Testing requirements

Go test file: `silicon_rust_go_test.go`

Minimum 30 test functions covering:
- All 9 constant accessors return positive (or sign-correct) float64
- `ThermalVoltage(300)` ≈ 0.025852
- `IntrinsicConcentration(300)` ≈ 1e16
- `IntrinsicConcentration(50)` returns error
- `FermiPotential` p-type positive, n-type negative
- `PNJunctionBuiltInVoltage` in (0.5, 1.5) V range
- `PNJunctionDepletionWidth` positive at zero bias
- `PNJunctionSaturationCurrent` positive
- `PNJunctionCurrent` forward bias positive
- `MosfetThresholdVoltage` NMOS > 0.5 V for high body doping
- `EvaluateLevel1Defaults` returns MosResult with Region="saturation" for vGs=vDs=1.8 V
- `EvaluateLevel1Defaults` cutoff for vGs=0
- `EvaluateLevel1` explicit params, saturation
- `Deposit` on empty → starts with material
- `Deposit` prepends layer
- `DealGroveOxidation` starts with "SiO2:"
- `DealGroveOxidationCustom` starts with "SiO2:"
- `Etch` removes top layer
- `Implant` returns string, no error
- `Diffuse` returns string, no error
- `DiffuseWithTemp` returns string
- `ImplantRange("B", 30)` → rp ≈ 92 nm, straggle ≈ 38 nm
- `DiffusivityCm2PerS("B", 1000)` ≈ 1e-14
- Injection guard: `Deposit` with `|` in material name returns error
- Injection guard: `Deposit` with `:` in material name returns error
- `DealGroveOxidation` with negative time returns error
- `ImplantRange` with unknown species returns error

## Platform notes

| Platform | Library | CGo link |
|----------|---------|---------|
| Linux    | `libsilicon_rust_cgo.so` | `-lsilicon_rust_cgo` |
| macOS    | `libsilicon_rust_cgo.dylib` | `-lsilicon_rust_cgo` |
| Windows  | `silicon_rust_cgo.dll` | `-lsilicon_rust_cgo` (MinGW) |

CGo on Windows requires a MinGW-based toolchain (the standard Go Windows
installer ships with a compatible gcc).  `build.rs` is a no-op — the cdylib
contains no undefined external symbols.

## Build

```bash
# 1. Build the Rust cdylib
cargo build -p silicon-rust-cgo --release

# 2. Run Go tests (CGo links the .so from target/release)
cd code/packages/go/silicon-rust-go
go test ./...
```
