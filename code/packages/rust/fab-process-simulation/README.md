# fab-process-simulation

1-D analytical CMOS process flow simulator: Deal-Grove thermal oxidation, ion implantation (Gaussian profiles), Fick's law diffusion, deposition, and selective etching.

## What it does

This crate implements first-order 1-D analytical models for each major CMOS fabrication step.  It is calibrated against published Sky130 reference profiles and suitable for:

- Teaching process sequence design and parameter sensitivity
- Building test fixtures for TCAD comparison
- Smoke-testing process corners in pre-silicon design flows

| Step        | Model                        | Parameters                               |
|-------------|------------------------------|------------------------------------------|
| Oxidation   | Deal-Grove quadratic law     | time_min, A [µm], B [µm²/hr]            |
| Deposition  | Uniform film                 | material, thickness_nm                   |
| Etch        | Layer-selective depth removal| target_material, depth_nm                |
| Implant     | Gaussian from SRIM table     | species, energy_keV, dose_per_cm2        |
| Diffusion   | Fick's second law broadening | time_min, temperature_C                  |

## How it fits in the stack

```
fab-process-simulation   ←  process sequence → cross-section model (this crate)
         │
device-physics           ←  physical constants (silicon permittivity, etc.)
         │
spice-engine             ←  circuit analysis of devices made by this process
```

## Usage

```rust
use fab_process_simulation::{CrossSection, Layer, deal_grove_oxidation, deposit, etch, implant, diffuse};

// Start with a bare 500 nm silicon substrate.
let cs = CrossSection { layers: vec![Layer::new("Si", 500.0)] };

// Grow a 2-3 nm gate oxide (dry O2, 1000 °C, ~1.5 min).
let cs = deal_grove_oxidation(&cs, 1.5, None, None).unwrap();

// Deposit 50 nm of poly-Si gate.
let cs = deposit(&cs, "Poly", 50.0).unwrap();

// Ion-implant boron source/drain at 30 keV, 1×10¹³ /cm².
let cs = implant(&cs, "B", 30.0, 1e13).unwrap();

// Anneal at 1000 °C for 30 min.
let cs = diffuse(&cs, 30.0, None);

// Inspect the cross-section.
for layer in &cs.layers {
    println!("{}: {:.1} nm", layer.material, layer.thickness_nm);
}
```

## Supported implant species

| Species | Energies (keV) | Notes                  |
|---------|----------------|------------------------|
| B       | 10, 30, 100    | p-type dopant           |
| P       | 30, 100        | n-type dopant           |
| As      | 30, 100        | n-type, heavy, slow diff|
| BF2     | 30, 60         | low-energy B surrogate  |

Intermediate energies are linearly interpolated.  Out-of-range energies are extrapolated.

## Limitations

- All models are 1-D (vertical only); lateral dopant spread requires 2-D/3-D TCAD.
- Diffusion is simplified: the Gaussian width broadening is computed but the sampled profile is not re-convolved (v0.2.0 work).
- Oxidation uses dry-O₂ 1000 °C constants; wet-O₂ or other temperatures require supplying custom A and B parameters.
