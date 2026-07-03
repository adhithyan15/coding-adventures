# device-physics

Semiconductor device physics primitives: physical constants, thermal voltage, intrinsic carrier concentration, Fermi potential, PN junction analysis, and MOSFET threshold voltage with body effect.

## What it does

This crate provides the lowest-level analytical models used throughout the silicon stack:

- **Physical constants** — Boltzmann constant, electron charge, silicon and SiO₂ permittivities, mobility values, bandgap, intrinsic carrier concentration at 300 K.
- **Thermal voltage** — `thermal_voltage(T)` = kT/q; ≈ 25.85 mV at room temperature.
- **Intrinsic concentration** — `intrinsic_concentration(T)` with T³/² × exp(−Eg/2kT) temperature scaling.
- **Fermi potential** — `fermi_potential(N, kind, T)` for p-type (+) and n-type (−) silicon.
- **PN junction** — `PNJunction` with built-in voltage, depletion width, saturation current, and Shockley current.
- **MOSFET threshold voltage** — `MOSFETParams` with body effect (γ coefficient), flat-band voltage, and oxide capacitance.

## How it fits in the stack

```
device-physics   ← fundamental constants and models (this crate)
     │
mosfet-models   ← Level-1 SPICE MOSFET I-V model (uses thermal_voltage)
     │
spice-engine    ← MNA circuit simulator (uses MOSFET dc evaluation)
```

## Usage

```rust
use device_physics::{thermal_voltage, intrinsic_concentration, PNJunction, MOSFETParams};

// Thermal voltage at 300 K
let vt = thermal_voltage(300.0); // ≈ 0.02585 V

// Intrinsic concentration at 350 K
let ni = intrinsic_concentration(350.0).unwrap(); // > 1e16 /m³

// PN junction analysis
let j = PNJunction::new(1e23, 1e22, 1e-8, 300.0, 1e-6, 1e-6).unwrap();
println!("V_bi = {} V", j.built_in_voltage());
println!("I(0.6 V) = {:.3e} A", j.current(0.6));

// MOSFET threshold voltage
let p = MOSFETParams::new("NMOS", 130e-9, 1e-6, 2e-9, 1e24, -0.05, 0.0, 300.0).unwrap();
let vt_mos = p.threshold_voltage(0.0).unwrap();
println!("V_t = {} V", vt_mos);
```

## SI units

All inputs and outputs use SI units unless explicitly stated otherwise:

| Quantity            | Unit   |
|---------------------|--------|
| Temperature T       | K      |
| Doping N            | /m³    |
| Current I           | A      |
| Voltage V           | V      |
| Area A              | m²     |
| Capacitance C_ox    | F/m²   |
| Permittivity ε      | F/m    |
