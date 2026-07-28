# mosfet-models

MOSFET I-V models with a uniform interface for the SPICE engine. v0.1.0 ships **Level-1** (Shockley square-law); EKV and BSIM3v3 subsets follow in v0.2.0.

See [`code/specs/mosfet-models.md`](../../../specs/mosfet-models.md).

## Quick start

```python
from mosfet_models import MOSFET, Level1Model, Level1Params, MosfetType

nmos = MOSFET(
    type=MosfetType.NMOS,
    model=Level1Model(Level1Params(VT0=0.42, KP=220e-6, W=1e-6, L=130e-9)),
)

# Operating point
r = nmos.dc(V_GS=1.8, V_DS=1.8)
print(r.Id, r.gm, r.gds, r.region)
# Id = ~280 µA, region = saturation
```

## v0.1.0 scope

- `Level1Params`: Level-1 DC, geometry, capacitance, and noise parameters with
  sane defaults for 130 nm-style NMOS, including zero-default `LD` lateral
  diffusion through `L_eff = L - 2*LD`, `PB` / `MJ` / `FC` bulk-junction
  depletion shaping, Berkeley-default `TOX` gate oxide thickness for intrinsic
  Meyer capacitance through `Cox = epsilon_ox / TOX`, zero-default `RD` / `RS`
  external drain/source resistance, zero-default `RSH` sheet resistance,
  one-default `NRD` / `NRS` drain/source diffusion square counts, zero-default
  drain/source diffusion areas `AD` / `AS`, drain perimeter `PD`, and
  bottom/sidewall junction capacitance densities `CJ` / `CJSW` contributing
  `CJ * AD + CJSW * PD` to `CBD` and `CJ * AS` to `CBS`, and
  `KF` / `AF` flicker noise.
- `evaluate_level1(params, V_GS, V_DS, V_BS, T)`: returns `MosResult` with Id, gm, gds, gmb, Cgs/Cgd/Cgb/Cbs/Cbd, region.
- Region detection: cutoff (subthreshold-aware), triode, saturation.
- Body effect via gamma * (sqrt(PHI-V_BS) - sqrt(PHI)) shift.
- Channel-length modulation factor.
- Subthreshold-current model (toggle via `subthreshold_enable`).
- `MOSFET` wrapper: NMOS or PMOS, with sign flipping for PMOS.

## Out of scope (v0.2.0)

- EKV (smooth all-region model).
- BSIM3v3 subset for Sky130 cell characterization.
- Velocity saturation for sub-100 nm devices.
- Non-quasi-static dynamic model.
- Aging models (NBTI, PBTI, HCI).

MIT.
