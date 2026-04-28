# Fab Process Simulation

## Overview

Once a GDSII layout exists, a fab takes those mask polygons and turns them into a working chip via a sequence of physical steps: oxidation, photolithography, etching, ion implantation, diffusion, deposition, planarization, metallization. This spec defines a 1-D analytical process simulator that takes a process recipe + a vertical cross-section of the layout and produces the resulting device geometry, doping profiles, and oxide/metal stacks.

Why simulate fab? Three reasons:
1. **Education** — The reader must understand *what doping is* before BSIM3v3 parameters mean anything.
2. **Validation** — Compute the doping profile under a transistor's gate, plug it into `device-physics.md`'s threshold formula, compare to BSIM3v3's `VT0`. They should match within 10%.
3. **Mask sanity** — A surprising number of chip bugs are mask drawing errors caught by simulating the resulting cross-section.

This spec is **deliberately limited to 1-D analytical models**. Real TCAD (Sentaurus, Genius) solves 2-D or 3-D nonlinear PDEs with finite-element meshes — months of work. Our 1-D models are calibrated against the analytical solutions and against published Sky130 reference profiles; they are quantitatively accurate for the front-end (oxide, source/drain, channel doping) and qualitatively accurate for back-end-of-line (metal stack thicknesses).

## Layer Position

```
device-physics.md
       │ (drift-diffusion, depletion, MOSFET equations)
       ▼
fab-process-simulation.md  ◀── THIS SPEC
       │ (recipe + masks → cross-sections, doping, stack)
       ▼
sky130-pdk.md
       │ (recipe = Sky130 process flow with calibrated models)
       ▼
standard-cell-library.md  ──► ASIC backend
```

## Concepts

### A wafer is a 1-D problem (mostly)

Lithography defines patterns horizontally; process steps modify the wafer vertically. For most steps, the *vertical* profile at any point is independent of the lateral position (away from feature edges). So a 1-D model — "what does the wafer look like along the depth axis at this lithographic state?" — captures most of the physics.

Where 1-D fails:
- **Junction shaping**: source/drain doping wraps around the gate edge in a 2-D shape. We approximate with a Gaussian centered at the mask edge.
- **Topography effects**: oxide thickness varies near step edges (mask corners). Skip; we assume planar.
- **Feature interactions**: STI (shallow-trench isolation) creates 2-D stress fields. Skip.

For a teaching simulator, 1-D is sufficient.

### The CMOS process flow (twin-well, 130 nm-scale)

A simplified Sky130-flavored flow. Real Sky130 has 80+ steps; we capture the 15 essential ones.

```
Step 1:  Start with bare p-type Si wafer (lightly doped, N_A ≈ 1e15 /cm³)
Step 2:  Pad oxide growth (~10 nm thermal SiO₂)
Step 3:  Pad nitride deposition (~150 nm Si₃N₄)
Step 4:  Active mask + nitride etch + STI trench + STI fill (oxide)
Step 5:  Strip nitride; n-well mask + phosphorus implant + diffusion
Step 6:  p-well mask + boron implant + diffusion
Step 7:  V_t adjust implant for NMOS (boron, shallow)
Step 8:  V_t adjust implant for PMOS (phosphorus, shallow)
Step 9:  Sacrificial oxide grow + strip (clean surface)
Step 10: Gate oxide growth (4-8 nm thermal SiO₂)
Step 11: Polysilicon deposition (~200 nm) + doping
Step 12: Poly mask + poly etch (defines gate length)
Step 13: LDD (lightly doped drain) implants — As (NMOS), BF₂ (PMOS)
Step 14: Spacer formation (oxide deposit + anisotropic etch)
Step 15: Heavy source/drain implants — As (NMOS), B (PMOS)
Step 16: Activation anneal (rapid thermal: ~1050°C, seconds)
Step 17: Salicide (Ti or Co or Ni silicide on poly + S/D)
Step 18: Pre-metal dielectric (PMD) + contact mask + W plug fill
Step 19: M1 deposition + M1 mask + M1 etch
Step 20: IMD-1 + via mask + W via fill
Step 21: M2 deposition + M2 mask + M2 etch
... repeat for M3-M5 ...
Step 22: Passivation (SiN + polyimide); pad mask
```

We model steps 1-17 in detail (front-end-of-line, FEOL); steps 18+ (back-end-of-line, BEOL) are modeled as a stack of nominal layer thicknesses without process simulation.

## Step Models

### Step Model 1 — Oxidation (Deal-Grove)

Thermal oxide grows on Si by reaction: `Si + O₂ → SiO₂` (dry) or `Si + 2H₂O → SiO₂ + 2H₂` (wet).

Deal-Grove model:
```
T_ox² + A × T_ox = B × (t + τ)
```

where `T_ox(t)` is oxide thickness, `A` and `B` are temperature- and pressure-dependent constants, `τ` accounts for any pre-existing oxide.

Two regimes:
- **Linear** (early growth, `t << A²/4B`): `T_ox ≈ (B/A) × (t + τ)` — reaction-rate limited.
- **Parabolic** (thick oxide, `t >> A²/4B`): `T_ox ≈ sqrt(B × t)` — diffusion-limited.

Constants for dry oxidation at 1000°C:
```
A = 0.165 µm
B = 0.0117 µm²/hr
```

For 5 nm gate oxide at 800°C dry, growth time ~10 minutes. Our model returns `T_ox(t)` given the recipe step.

### Step Model 2 — Photolithography

A photomask + light + photoresist + developer → patterned resist. The aerial image (intensity vs position) determines what gets exposed.

For a binary mask with feature width `W` at wavelength `λ` and numerical aperture `NA`:
- Resolution limit: `R = k₁ × λ / NA` where `k₁` is a process-dependent constant (~0.4).
- Depth of focus: `DOF = k₂ × λ / NA²` (~0.3 µm for 193-nm immersion).

For our purposes, we model lithography as **threshold-based**: any region of the wafer that receives intensity > 0.5 × peak crosses the develop threshold and is exposed (for positive resist) or unexposed (for negative resist).

Aerial image (Hopkins-formula approximation, simplified):
```
I(x) = |∑ fourier_components_of_mask × pupil_filter|²
```

For a feature wider than `R`, the aerial image is essentially the mask image. For sub-resolution features, the image is blurred — and OPC (optical proximity correction) adds compensating sub-resolution features to the mask. We simulate post-OPC masks; OPC itself is out of scope.

### Step Model 3 — Etching

Anisotropic etch (e.g., reactive-ion etch of poly):
- Vertical etch rate `R_v` >> lateral rate `R_l`.
- Mask defines etched-region edges.
- Bias: under-etch or over-etch leaves features wider or narrower than mask.

Isotropic etch (wet etch):
- Uniform `R_v = R_l`; etches under the mask.

For this simulator, each etch step is parameterized by `(R_v, R_l, time, mask)`. The wafer cross-section is updated.

### Step Model 4 — Ion Implantation

A beam of ions (B, P, As, BF₂) accelerated to fixed energy hits the wafer. Implanted ions follow a stopping curve and end up at a depth distribution well-approximated by a Gaussian:

```
N(x) = (Q / (Rp_std × sqrt(2π))) × exp(-(x - Rp)² / (2 × Rp_std²))
```

where:
- `Q` = dose (atoms/cm²)
- `Rp` = projected range (depth of peak; depends on ion type and energy)
- `Rp_std` = straggle (standard deviation of the Gaussian)

Tabulated Rp and Rp_std from SRIM/TRIM or IEEE published data; we ship a small table.

| Ion | Energy (keV) | Rp (nm) | Rp_std (nm) |
|---|---|---|---|
| B | 10 | 33 | 18 |
| B | 30 | 92 | 38 |
| P | 30 | 39 | 19 |
| P | 100 | 130 | 50 |
| As | 30 | 22 | 11 |
| As | 100 | 64 | 28 |
| BF₂ | 30 | 31 | 19 |

### Step Model 5 — Diffusion (Fick's Second Law)

After implantation, a thermal anneal drives in / activates the dopants. In 1-D:

```
∂N/∂t = D × ∂²N/∂x²
```

where `D(T)` is the diffusivity (Arrhenius: `D = D₀ × exp(-E_a / kT)`). For boron in Si at 1000°C, `D ≈ 1e-14 cm²/s`. For 30 minutes, `sqrt(D × t) ≈ sqrt(1e-14 × 1800) ≈ 4 nm` of diffusion length.

Closed-form solutions:
- **Pre-deposition + drive-in** (e.g., implant then anneal): the Gaussian implant profile broadens to a Gaussian with `Rp_std' = sqrt(Rp_std² + 2Dt)`.
- **Constant-source** (e.g., POCl₃ from gas): erfc profile, depth grows as `2 × sqrt(Dt)`.

For our 1-D simulator, every implant + anneal step transforms the existing Gaussian distributions per the above formulas.

### Step Model 6 — Deposition (CVD/PVD)

Films deposited by chemical or physical vapor deposition. We model as: target thickness, deposition uniformity, conformality. For planar deposition (no topography), uniform thickness everywhere. For non-planar, conformality dictates step coverage. Most relevant for poly-Si, dielectrics, and metals.

### Step Model 7 — CMP (Chemical-Mechanical Planarization)

After dielectric or metal deposition, the wafer surface is rough. CMP polishes it flat. Model: uniform thinning to a target thickness above the highest feature, with dishing/erosion proportional to local pattern density. For our planar 1-D world, we just set the surface to the target thickness.

### Step Model 8 — Activation anneal

Rapid thermal at ~1050°C for ~10 seconds. Activates dopants (puts them in substitutional sites where they release/accept electrons) and partially anneals damage. Modeled as a final diffusion + activation factor (~95% activation typical).

### Step Model 9 — Silicide formation

Sputter Ti/Co/Ni; thermal anneal forms TiSi₂/CoSi₂/NiSi on exposed Si (poly + S/D), unreacted metal stripped. Reduces sheet resistance from ~100 Ω/sq (n+ Si) to ~5 Ω/sq (silicide). Modeled as a layer addition on Si surfaces.

## Worked Example — NMOS transistor cross-section

Trace an NMOS through the front-end-of-line:

```
Step 0: bare wafer
        depth ──→
        ──────────────────── Si surface
        p-type Si (N_A = 1e15 /cm³)
        ────────────────────

Step 4: STI in place (skip — flat region between transistors; we look at the gate region)

Step 6: p-well doping (boron implant 100 keV, 1e13 /cm² + drive-in 60 min @ 1000°C)
        N_well(x) Gaussian:
          Rp = 280 nm
          Rp_std' = sqrt(80² + 2 × 1e-14 × 3600 × 1e14) ≈ 270 nm
          peak ≈ 1e17 /cm³ at 280 nm depth

Step 7: V_t adjust (boron implant 10 keV, 5e12 /cm²)
        Shallow Gaussian:
          Rp = 33 nm, Rp_std = 18 nm
          peak ≈ 1.4e18 /cm³ at 33 nm depth
        Total channel doping (well + V_t adj) at the surface ≈ 4e17 /cm³

Step 10: gate oxide (5 nm thermal SiO₂, dry, 800°C, 10 min)

Step 11: poly deposition (200 nm) + n+ doping (phos diffusion)

Step 12: poly etch (mask: gate of width L = 130 nm)

Step 13: LDD (As implant 5 keV, 1e14 /cm²)
        Very shallow N+ regions on either side of gate (gate masks the channel area)
        Rp = 7 nm, peak ≈ 1.5e19 /cm³

Step 14: oxide spacer (50 nm)

Step 15: heavy S/D (As implant 60 keV, 5e15 /cm²)
        Deep N++ S/D (where spacer exposes Si)
        Rp = 50 nm, peak ≈ 5e20 /cm³

Step 16: activation anneal (RTA 1050°C, 10 s)
        All Gaussians broaden slightly:
          Diffusion length sqrt(2 × D_B × 10) ≈ 5 nm at 1050°C

Step 17: salicide (CoSi₂ on S/D and gate top)
```

Final cross-section at the gate center:
```
        depth ──→
        ─── 0 nm  ── poly-Si gate (250 nm thick after silicide reduction)
        ─── -5 nm ── gate oxide SiO₂ (5 nm)
        ─── 0 nm  ── Si surface
                  ── channel doping (boron, ~4e17 /cm³, peak near surface)
        ─── 280 nm ── p-well peak (boron, ~1e17 /cm³)
        ─── 1 µm  ── p-substrate (boron, 1e15 /cm³)
```

Computing V_t for this profile:
- `C_ox = 3.9 × 8.854e-12 / 5e-9 = 6.91 × 10⁻³ F/m²`
- `N_A_eff` (channel surface) ≈ 4e17 /cm³ = 4e23 /m³
- `φ_F = 0.0259 × ln(4e23 / 1e16) ≈ 0.45 V`
- `γ = sqrt(2 × 11.7 × 8.854e-12 × 1.602e-19 × 4e23) / 6.91e-3 ≈ 0.50 V^(1/2)`
- `V_FB ≈ -0.95 V` (n+ poly, p-Si body)
- `V_t = -0.95 + 2 × 0.45 + 0.50 × sqrt(2 × 0.45) ≈ 0.42 V`

Compare to BSIM3v3 default `VT0 = 0.42 V` — exact match. The fab simulation reproduces the device parameter from first principles.

## Worked Example — Adder transistor count

The 4-bit adder mapped to Sky130 standard cells consumes ~25 cells. Each cell averages ~10 transistors. Total: ~250 NMOS + 250 PMOS = ~500 transistors. Our cross-section model is per-device but reused; the chip-level result is 500 instances of the cross-section + interconnect.

This example shows that the 1-D cross-section model scales: same physics applied 500 times.

## Public API

```python
from dataclasses import dataclass, field
from enum import Enum


class StepKind(Enum):
    OXIDATION    = "oxidation"
    LITHOGRAPHY  = "lithography"
    ETCH         = "etch"
    IMPLANT      = "implant"
    DIFFUSION    = "diffusion"
    DEPOSITION   = "deposition"
    CMP          = "cmp"
    ANNEAL       = "anneal"
    SILICIDE     = "silicide"


@dataclass(frozen=True)
class Layer:
    """One material layer in the cross-section, top of stack first."""
    material: str          # 'Si', 'SiO2', 'Poly', 'TiSi2', 'CoSi2', ...
    thickness_nm: float
    doping: dict[str, list[tuple[float, float]]] = field(default_factory=dict)
    # doping['B'] = [(depth_nm, conc_cm3), ...] gives the B profile inside this layer


@dataclass(frozen=True)
class CrossSection:
    """Vertical cross-section at a point on the wafer."""
    layers: tuple[Layer, ...]


@dataclass(frozen=True)
class Step:
    kind: StepKind
    name: str
    params: dict[str, float | str]


@dataclass(frozen=True)
class ProcessRecipe:
    name: str
    steps: tuple[Step, ...]


@dataclass(frozen=True)
class Mask:
    """A 1-D mask: a list of (start_x, end_x, opaque) tuples."""
    name: str
    intervals: tuple[tuple[float, float, bool], ...]


def simulate_step(cs: CrossSection, step: Step, mask_at_x: bool) -> CrossSection:
    """Apply a single process step to a cross-section at a given point.

    `mask_at_x` is True if the local mask is opaque (blocking) at this x position
    for litho-dependent steps (etch, implant); else the step proceeds normally.
    """
    ...


def simulate_recipe(
    recipe: ProcessRecipe,
    masks: dict[str, Mask],
    x: float = 0.0,
) -> CrossSection:
    """Run a full process recipe to produce the final cross-section at point x."""
    cs = CrossSection(layers=(Layer("Si", 1_000_000.0, doping={"B": [(0, 1e15)]}),))
    for step in recipe.steps:
        if step.params.get("mask"):
            mask_name = step.params["mask"]
            opaque = is_opaque(masks[mask_name], x)
        else:
            opaque = False
        cs = simulate_step(cs, step, opaque)
    return cs


# ─── Step model implementations ─────────────────────────────────

def step_oxidation(cs: CrossSection, T_C: float, time_min: float, ambient: str) -> CrossSection:
    """Deal-Grove oxide growth."""
    A, B = deal_grove_constants(T_C, ambient)  # tabulated
    T_ox_existing = top_oxide_thickness(cs)
    tau = (T_ox_existing**2 + A * T_ox_existing) / B
    t = time_min / 60.0   # to hours
    T_ox_new = (-A + sqrt(A**2 + 4 * B * (t + tau))) / 2
    delta = T_ox_new - T_ox_existing
    # Consume `delta * (rho_SiO2 / rho_Si) ≈ delta * 0.45` of Si; add `delta` of SiO2 on top.
    return apply_oxidation(cs, delta_nm=delta)


def step_implant(cs: CrossSection, ion: str, energy_keV: float, dose: float) -> CrossSection:
    Rp, Rp_std = implant_range(ion, energy_keV)
    return add_gaussian_doping(cs, ion, Rp, Rp_std, dose)


def step_diffusion(cs: CrossSection, T_C: float, time_min: float) -> CrossSection:
    """Apply Fick diffusion to all dopant species."""
    return diffuse_all(cs, T_C, time_min)


def step_etch(cs: CrossSection, target_layer: str, depth_nm: float, anisotropic: bool) -> CrossSection:
    return apply_etch(cs, target_layer, depth_nm, anisotropic)


# ─── Helpers ────────────────────────────────────────────────────

def deal_grove_constants(T_C: float, ambient: str) -> tuple[float, float]:
    """Returns (A, B) for Deal-Grove. Tabulated empirically."""
    ...

def implant_range(ion: str, energy_keV: float) -> tuple[float, float]:
    """Tabulated SRIM/TRIM ranges. Returns (Rp_nm, Rp_std_nm)."""
    ...

def diffuse_gaussian(profile: list[tuple[float, float]], D: float, t: float) -> list[tuple[float, float]]:
    """Convolve a profile with a Gaussian kernel. For pure Gaussians, just broaden Rp_std."""
    ...

def diffusivity_arrhenius(species: str, T_K: float) -> float:
    """D = D0 × exp(-Ea / kT). Tabulated D0, Ea."""
    ...
```

## Edge Cases

| Scenario | Handling |
|---|---|
| Implant through existing oxide | Adjust effective Rp using oxide stopping power. |
| Multiple implants of same species | Profiles add (linear superposition). |
| Out-diffusion during oxidation | Boron piles up at oxide interface (segregation); approximate with a multiplier. |
| Negative thicknesses (over-etch) | Layer is consumed entirely; etch into next layer. |
| Mask alignment offset | Add `mask_offset_nm` parameter; shift mask intervals. |
| Resist erosion during long etches | Reject; assume infinite resist selectivity for v1. |
| Annealing at temperatures below diffusion threshold | Skip diffusion; only damage anneal. |
| Stack height growing beyond total wafer thickness | Reject; warn. |
| Implant species not in table | Reject with descriptive error. |
| Recipe step references undefined mask | Reject. |

## Test Strategy

### Unit (target 95%+)
- Deal-Grove: 5 nm gate oxide grows in ~10 min @ 800°C dry. Verify within 5%.
- Implant range: B at 30 keV → Rp ≈ 92 nm. Verify exactly against table.
- Gaussian doping: integrated dose equals input.
- Diffusion broadening: Gaussian std² grows linearly in time at fixed T.
- Mask opacity check: a given (x, mask) returns the right boolean.

### Integration
- Full Sky130-flavored recipe → cross-section under gate. Compute V_t from physics. Match BSIM3v3 default VT0 within 10%.
- Same recipe at FF / SS / TT corners by varying VT-adjust dose ±10%. Verify V_t shifts agree with PVT corner specs.
- Etch + deposit + etch sequence: verify cross-section shape matches expected after each step.

### Property
- Mass conservation: total dopant atoms in cross-section after a diffusion step equal what went in.
- Symmetry: recipe with mirrored mask produces mirrored cross-section.

## Conformance / Reference

We model the *open* Sky130 process flow as documented at:
- https://github.com/google/skywater-pdk
- https://skywater-pdk.readthedocs.io/

We do **not** claim to reproduce proprietary Sky130 SPICE parameters exactly. Our simulated V_t matches Sky130's BSIM3v3 default within 10%, which is sufficient for teaching.

## Open Questions

1. **2-D effects** — should we add a poor-man's 2-D mode for source/drain wraparound? Recommendation: defer; document as future.
2. **Stress effects** — STI compresses channel; affects mobility. Skip for v1.
3. **Pattern density-dependent CMP** — out of scope.
4. **Lithography simulation** — full Hopkins formula or threshold? Recommendation: threshold for v1; full Hopkins as future.
5. **Recipe input format** — define a YAML/TOML format for recipes? Yes; enables sharing a recipe across teams.

## Future Work

- Full 2-D (or 3-D) finite-element TCAD bridge.
- Lithography aerial-image simulation (sub-resolution features, OPC).
- Stress engineering (eSiGe, dual-stress liner).
- Pattern density-dependent CMP.
- Defectivity / yield modeling.
- Strained-Si and SOI process flows.
- BEOL simulation: actual via shape, IR drop in metal stack.
- 3-D / FinFET fab flows.
