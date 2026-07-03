# silicon-rust-python

Python C extension bindings for the Rust silicon simulation stack —
`device-physics`, `mosfet-models`, and `fab-process-simulation`.

Exposes **26 Python functions** covering semiconductor physics constants,
P-N junction analysis, MOSFET Level-1 evaluation, and 1-D CMOS process
simulation.  Uses the repo's zero-dependency `python-bridge` crate (PEP 384
Limited API) — no PyO3, no bindgen, no Python headers at build time.

## How it fits in the stack

```
silicon-rust-python
├── device-physics      ← physical constants, thermal voltage, P-N junction, V_t
├── mosfet-models       ← SPICE Level-1 I-V model, regions, Jacobian
└── fab-process-simulation ← Deal-Grove oxidation, deposit, etch, implant, diffuse
```

## Build

```bash
# From the Rust workspace root:
cargo build -p silicon-rust-python
# Produces:
#   target/debug/silicon_rust_python.so   (Linux)
#   target/debug/silicon_rust_python.dylib (macOS)
#   target/debug/silicon_rust_python.pyd  (Windows)
```

Copy (or symlink) the artefact to a directory on Python's path and import it:

```python
import silicon_rust_python as srp
```

## API reference

### Physical constants (no arguments)

| Function | Returns | Value |
|---|---|---|
| `k_boltzmann()` | `float` | 1.380649×10⁻²³ J/K |
| `q_electron()` | `float` | 1.602176634×10⁻¹⁹ C |
| `eps0()` | `float` | 8.854187…×10⁻¹² F/m |
| `eps_si()` | `float` | 11.7 × ε₀ F/m |
| `eps_ox()` | `float` | 3.9 × ε₀ F/m |
| `n_i_300k()` | `float` | 1×10¹⁶ /m³ |
| `eg_si_300k()` | `float` | 1.12 eV |
| `mu_n_300k()` | `float` | 1350×10⁻⁴ m²/V·s |
| `mu_p_300k()` | `float` | 480×10⁻⁴ m²/V·s |

### Device physics

```python
# Thermal voltage kT/q [V]
srp.thermal_voltage(t_kelvin)

# Intrinsic concentration n_i(T) [/m³]; raises ValueError below 100 K
srp.intrinsic_concentration(t_kelvin)

# Fermi potential φ_F [V]; kind = "p" or "n"
srp.fermi_potential(n_doping, kind, t_kelvin)

# P-N junction: built-in voltage [V]
srp.pn_junction_built_in_voltage(na, nd, t)

# P-N junction: depletion width [m]; v_applied > 0 → forward bias
srp.pn_junction_depletion_width(na, nd, t, v_applied)

# P-N junction: saturation current [A]
srp.pn_junction_saturation_current(na, nd, area_m2, t, tau_n_s, tau_p_s)

# P-N junction: Shockley diode current [A]
srp.pn_junction_current(na, nd, area_m2, t, tau_n_s, tau_p_s, v)

# MOSFET threshold voltage [V] with body effect
# device_type = "NMOS" or "PMOS"
srp.mosfet_threshold_voltage(device_type, l, w, t_ox, n_body, phi_ms, q_ox, t, v_sb)
```

### MOSFET Level-1 evaluation

```python
# Full parameter set (returns dict)
result = srp.evaluate_level1(
    vt0, kp, lambda_, gamma, phi, w, l, n_sub,   # 8 device params
    v_gs, v_ds, v_bs, t                           # 4 operating-point values
)

# Default 130 nm NMOS (returns dict)
result = srp.evaluate_level1_defaults(v_gs, v_ds, v_bs, t)

# Result dict keys:
#   id [A], gm [A/V], gds [A/V], gmb [A/V]
#   cgs [F], cgd [F], cgb [F], cbs [F], cbd [F]
#   region: "cutoff" | "subthreshold" | "triode" | "saturation"
```

### Process simulation (CrossSection wire format)

A `CrossSection` is passed as a pipe-separated `"material:thickness_nm"` string:

```
""                              # empty (start here)
"Si:500.0"                      # 500 nm silicon substrate
"SiO2:4.8|Si:500.0"            # 4.8 nm gate oxide on silicon
"Poly:50.0|SiO2:4.8|Si:500.0"  # poly gate stack
```

All process functions take a cross-section string as their first argument and
return a new cross-section string after the step.

```python
cs = srp.deposit("", "Si", 500.0)            # bare Si substrate
cs = srp.deal_grove_oxidation(cs, 5.0)        # grow ~5 nm dry-O2 oxide
cs = srp.deal_grove_oxidation(cs, 5.0,        # custom A/B coefficients
                              0.2, 0.015)
cs = srp.deposit(cs, "Poly", 50.0)            # deposit poly
cs = srp.etch(cs, "Poly", 25.0)               # etch 25 nm of poly
cs = srp.implant(cs, "B", 30.0, 1e13)         # boron, 30 keV, 1e13 /cm²
cs = srp.diffuse(cs, 30.0)                    # 30 min at 1000 °C (default)
cs = srp.diffuse(cs, 30.0, 900.0)             # 30 min at 900 °C

# Range table lookup (SRIM; interpolated)
rp_nm, straggle_nm = srp.implant_range("B", 30.0)   # → (92.0, 38.0)
rp_nm, straggle_nm = srp.implant_range("P", 100.0)  # → (130.0, 50.0)

# Arrhenius diffusivity [cm²/s]
d = srp.diffusivity_cm2_per_s("B", 1000.0)   # → 1e-14
d = srp.diffusivity_cm2_per_s("As", 900.0)   # → scaled from 4e-15
```

Supported species for `implant` and `implant_range`: `"B"`, `"P"`, `"As"`, `"BF2"`.

### End-to-end example: CMOS inverter gate-oxide process

```python
import silicon_rust_python as srp

# Start with a bare silicon substrate (500 nm).
cs = srp.deposit("", "Si", 500.0)

# Grow 5-minute dry-O2 gate oxide at 1000 °C.
cs = srp.deal_grove_oxidation(cs, 5.0)
print(cs)  # "SiO2:X.X|Si:500.0"  (X.X ≈ 4.8 nm)

# Deposit 50 nm poly gate.
cs = srp.deposit(cs, "Poly", 50.0)

# Implant boron for PMOS source/drain.
cs = srp.implant(cs, "B", 30.0, 2e13)

# Anneal 30 minutes at 950 °C.
cs = srp.diffuse(cs, 30.0, 950.0)

# Evaluate the NMOS threshold at the gate stack.
vt = srp.mosfet_threshold_voltage("NMOS", 130e-9, 1e-6, 2e-9, 1e24, -0.05, 0.0, 300.0, 0.0)
print(f"V_t = {vt:.3f} V")

# Level-1 DC operating point.
r = srp.evaluate_level1_defaults(1.8, 1.8, 0.0, 300.15)
print(r["region"], r["id"])
```

## Wire format limitations (v0.1)

- **Doping profiles are not serialised.**  `implant` and `diffuse` update the
  doping tables inside the `CrossSection` but those tables are dropped when the
  cross-section is converted back to the wire string.  The layer thicknesses and
  material stack are preserved; doping values are not.
- Full doping-aware round-trip is planned for v0.2 via a JSON or binary wire
  format.

## Running tests

```bash
cargo test -p silicon-rust-python
```

All 15 unit tests are pure Rust (no Python interpreter required).
