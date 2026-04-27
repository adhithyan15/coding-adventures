# fab-process-simulation

1-D analytical CMOS process flow simulator. Models thermal oxidation, photolithography (threshold-based), etching, ion implantation, and diffusion.

See [`code/specs/fab-process-simulation.md`](../../../specs/fab-process-simulation.md).

## Quick start

```python
from fab_process_simulation import (
    CrossSection, Layer,
    deal_grove_oxidation, deposit, etch, implant, diffuse,
)

# Start with bare Si wafer
cs = CrossSection(layers=[Layer("Si", thickness_nm=500_000)])

# Grow 5 nm gate oxide (1000 °C dry, ~10 minutes)
cs = deal_grove_oxidation(cs, time_min=10)
print(cs.layers[0])  # SiO2 about 5 nm

# Implant boron (V_t adjust)
cs = implant(cs, species="B", energy_keV=10, dose_per_cm2=5e12)
# Si layer now has a Gaussian B doping profile

# Anneal
cs = diffuse(cs, time_min=30, temperature_C=1000)
```

## v0.1.0 scope

- `CrossSection` + `Layer` data classes (top-down stack with per-species doping profiles).
- `deal_grove_oxidation(cs, time_min, A, B)`: Deal-Grove thermal oxide growth on Si with optional pre-existing oxide.
- `deposit(cs, material, thickness_nm)`: deposit a uniform layer on top.
- `etch(cs, target_layer, depth_nm)`: anisotropic etch of the top layer if it matches.
- `implant(cs, species, energy_keV, dose_per_cm2)`: Gaussian doping profile from SRIM-derived range tables.
- `diffuse(cs, time_min, temperature_C)`: Fick's-law broadening (simplified).

Implant tables: B/P/As/BF2 at typical energies. Tabulated Rp + Rp_std interpolated linearly within species.

## Out of scope (v0.2.0)

- 2-D / 3-D PDE TCAD (real Sentaurus-class simulation).
- Detailed lithography (Hopkins formula aerial-image simulation).
- Wafer-level pattern density-dependent CMP.
- Stress engineering (eSiGe, dual-stress liner).
- BEOL via shape simulation.
- 3-D / FinFET fabrication flows.
- Defectivity / yield modeling.

MIT.
