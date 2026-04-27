# device-physics

Foundation of the analog stack. Drift-diffusion + depletion-approximation derivations of PN junction (Shockley equation) and MOSFET threshold voltage from first principles.

See [`code/specs/device-physics.md`](../../../specs/device-physics.md).

## Quick start

```python
from device_physics import (
    PNJunction, MOSFETParams,
    intrinsic_concentration, fermi_potential, thermal_voltage,
)

# Thermal voltage at room temperature
V_T = thermal_voltage(300)  # ≈ 0.02585 V

# A PN junction
diode = PNJunction(N_A=1e23, N_D=1e23, A=1e-8)
print(diode.built_in_voltage())   # ≈ 0.836 V
print(diode.current(0.6))          # forward bias current

# An NMOS transistor's threshold voltage
nmos = MOSFETParams(
    type="NMOS",
    L=130e-9, W=1e-6,
    T_ox=4e-9,
    N_body=1e23,    # 1e17/cm^3 in /m^3
    phi_MS=-0.95,
)
print(nmos.threshold_voltage())  # ≈ 0.42 V at V_SB=0
print(nmos.threshold_voltage(V_SB=2.0))  # higher due to body effect
```

## Physics implemented

- **Intrinsic carrier concentration** n_i(T) with bandgap-narrowing temperature scaling.
- **Fermi potential** φ_F = V_T × ln(N/n_i) for p-type and n-type doping.
- **PN junction**:
  - Built-in voltage φ_bi = V_T × ln(N_A·N_D / n_i²).
  - Depletion width W(V) from Poisson + depletion approximation.
  - Saturation current I_S from minority-carrier diffusion (Einstein relation D = μV_T).
  - Shockley equation I = I_S × (exp(V/V_T) - 1).
- **MOSFET threshold**:
  - Flat-band voltage V_FB = φ_MS - Q_ox/C_ox.
  - V_t0 = V_FB + 2φ_F + γ × sqrt(2φ_F).
  - Body effect: V_t(V_SB) = V_t0 + γ × (sqrt(2φ_F + V_SB) - sqrt(2φ_F)).
  - γ = sqrt(2 ε_Si q N_body) / C_ox.

## v0.1.0 scope

The teaching subset of semiconductor physics needed to ground BSIM3v3 in first principles. Sufficient for `mosfet-models`, `spice-engine`, and `standard-cell-library` characterization runs.

Reproduces Sky130 NMOS V_t ≈ 0.42 V from physical parameters (130 nm L, 4 nm T_ox, 10^17 /cm³ body doping, polysilicon-on-pSi work-function difference).

## Out of scope (v0.2.0)

- Quantum corrections (poly depletion, channel quantization).
- Bandgap narrowing in heavily doped regions.
- Gate tunneling currents.
- Aging models (NBTI, PBTI, HCI).
- High-K / metal-gate stacks.

MIT.
