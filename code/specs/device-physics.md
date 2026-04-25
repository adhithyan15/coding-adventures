# Device Physics

## Overview

Every "magic constant" in a transistor model has a physics origin. The threshold voltage `V_t` is not a number we measure once and tabulate — it is `V_FB + 2φ_F + γ√(2φ_F + V_SB)`, where each of those terms comes from a stack of physical reasoning: Fermi levels, depletion regions, surface potentials, oxide capacitance. This spec derives those equations from first principles so that `mosfet-models.md` can use them to compute `I_d(V_GS, V_DS, V_BS)`, and so the rest of the stack (SPICE, standard cells, fab-process simulation) has a coherent foundation.

This is the bottom-most analog spec in the silicon stack. It depends on nothing — only on the laws of electromagnetism and statistical mechanics, which we treat as given. Everything **above** it depends on the equations derived here:

- `mosfet-models.md` uses these equations to model individual transistors.
- `spice-engine.md` solves circuit-level equations whose elements come from the device models.
- `standard-cell-library.md` characterizes cells using SPICE simulations driven by these models.
- `fab-process-simulation.md` produces the *parameters* (doping, oxide thickness, channel length) that feed back into the device equations.

### Generality

Although MOSFETs are the focus (because every digital cell we'll build is CMOS), the underlying machinery — drift, diffusion, recombination-generation, Poisson's equation — describes every solid-state device. The same derivation framework, with different boundary conditions, gives BJTs, JFETs, diodes, photodetectors, and solar cells. We derive the bipolar diode equation as a warm-up before attacking the MOSFET, both because the diode is everywhere in CMOS (every junction is a diode) and because the techniques are the same.

## Layer Position

```
       (no spec depends on this from below)
                      │
                      ▼
              ┌─────────────────────┐
              │  device-physics.md  │  ◀── THIS SPEC
              │                     │
              │  outputs: V_t(W,L,doping,T_ox)│
              │           I_diode(V,T)│
              │           depletion widths│
              │           mobility models│
              └─────────────────────┘
                      │
       ┌──────────────┼──────────────┐
       ▼              ▼              ▼
 mosfet-models.md  fab-process-   (informs spice-engine
                   simulation.md   convergence aids)
```

## Concepts

### What we treat as given

We start from these constants and laws as axioms; deriving them is the job of an undergraduate physics curriculum, not this spec.

| Constant | Symbol | Value | Where it appears |
|---|---|---|---|
| Boltzmann constant | k | 1.380649 × 10⁻²³ J/K | Thermal voltage `V_T = kT/q` |
| Electron charge | q | 1.602177 × 10⁻¹⁹ C | Everywhere |
| Permittivity of free space | ε₀ | 8.854188 × 10⁻¹² F/m | Capacitance |
| Permittivity of silicon | ε_Si | 11.7 × ε₀ | Depletion capacitance |
| Permittivity of SiO₂ | ε_ox | 3.9 × ε₀ | Gate oxide capacitance |
| Silicon bandgap (300 K) | E_g | 1.12 eV | Intrinsic carrier concentration |
| Intrinsic carrier conc. (300 K) | n_i | 1.0 × 10¹⁰ cm⁻³ | Equilibrium |
| Effective DOS, conduction band | N_C | 2.8 × 10¹⁹ cm⁻³ | Fermi level placement |
| Effective DOS, valence band | N_V | 1.04 × 10¹⁹ cm⁻³ | Fermi level placement |
| Electron mobility (low field, lightly doped Si, 300 K) | μ_n | ~1350 cm²/V·s | Drift current |
| Hole mobility (low field, lightly doped Si, 300 K) | μ_p | ~480 cm²/V·s | Drift current |

We also assume:
- **Maxwell's equations** in the quasi-static limit (devices are slow compared to light; magnetic effects negligible).
- **Boltzmann statistics** for non-degenerate semiconductors (carrier concentrations < N_C, N_V).
- **Charge neutrality** in the bulk far from junctions / interfaces.
- **The depletion approximation** — at junctions, the transition between neutral and depleted regions is sharp.

### Why electrons and holes are both currents

Silicon is a covalent crystal. Pure silicon at 0 K has every valence electron bonded; no free carriers; infinite resistance. At room temperature, thermal energy promotes a small number of electrons into the conduction band, leaving "holes" — vacant bonds that adjacent electrons can hop into. The hop is electrically equivalent to a positive charge moving the *opposite direction* of the actual electron motion.

So we treat holes as if they were positive particles with mobility μ_p ≈ 480 cm²/V·s. They are not; they are the absence of electrons. But the math works out the same, and tracking holes makes the bookkeeping much simpler than tracking N − 1 electrons in a sea of N − 1 places.

```
Pure silicon (intrinsic), 300 K:
  Conduction band:  ●  ●        ●            ←  ~10¹⁰ free electrons / cm³
                    │  │        │
                    ▼  ▼        ▼   (recombine occasionally)
  Valence band:    ○  ○         ○            ←  ~10¹⁰ holes / cm³ (same number)

  In equilibrium: n × p = n_i²  (mass-action law)
```

### Doping

Add a few parts per billion of phosphorus (5 valence electrons): each P atom contributes one extra electron to the conduction band. Now `n ≈ N_D >> n_i` — the silicon is **n-type**. The hole concentration drops to maintain `n × p = n_i²`, so `p ≈ n_i² / N_D`.

Add boron (3 valence electrons): each B atom accepts one electron from the valence band, leaving a hole. Now `p ≈ N_A >> n_i` — **p-type**.

This is how we control conductivity by 10+ orders of magnitude with parts-per-million precision: doping. Every transistor's behavior is determined by *where* the doping changes from p to n (the metallurgical junctions) and *how heavy* it is in each region.

### The Fermi level

The Fermi level E_F is the energy at which the probability of a state being occupied is exactly 1/2. In the conduction band, the carrier concentration depends on how far E_F sits below E_C (the conduction band edge):

```
n = N_C × exp(-(E_C − E_F) / kT)
p = N_V × exp(-(E_F − E_V) / kT)
```

In intrinsic (undoped) silicon, E_F sits near mid-gap (E_F = E_i ≈ (E_C + E_V) / 2). Doping shifts it: n-type pushes E_F up toward E_C; p-type pushes it down toward E_V.

The shift is captured by the **Fermi potential**:

```
φ_F = (kT/q) × ln(N_A / n_i)   for p-type (φ_F > 0)
φ_F = (kT/q) × ln(N_D / n_i)   for n-type (use negative sign convention)
```

`φ_F` shows up everywhere — it sets diode built-in voltages, MOSFET thresholds, and the size of the depletion region. At room temperature, kT/q ≈ 0.0259 V (the **thermal voltage** V_T), so for `N_A = 10¹⁷ cm⁻³`:

```
φ_F = 0.0259 × ln(10¹⁷ / 10¹⁰) = 0.0259 × ln(10⁷) = 0.0259 × 16.12 ≈ 0.418 V
```

### Drift current

An electric field E applies a force qE to a carrier. Carriers don't accelerate forever — they scatter off lattice vibrations (phonons) and impurities. The result is an average drift velocity:

```
v_drift = μ × E    (for low fields)
```

Mobility μ depends on the material, doping (heavier doping → more impurity scattering → lower μ), temperature, and the field itself (very high fields saturate v_drift). Drift current is then:

```
J_n_drift = q × n × μ_n × E
J_p_drift = q × p × μ_p × E
```

### Diffusion current

Even without a field, carriers diffuse from high concentration to low. Fick's law:

```
J_n_diff = + q × D_n × dn/dx
J_p_diff = − q × D_p × dp/dx
```

Sign convention: electrons diffuse *down* the gradient and carry negative charge, so J_n is *positive* in the direction of decreasing n. Holes are positive carriers; J_p is negative in the direction of decreasing p (the carriers go down the gradient, the conventional current goes up).

D_n and D_p are diffusion constants; they relate to mobility by the **Einstein relation**:

```
D = μ × kT/q = μ × V_T
```

This is one of the deepest results in semiconductor physics: drift and diffusion are two faces of the same scattering process.

### The drift-diffusion equation (DDE)

Total current density combines both:

```
J_n = q × μ_n × n × E + q × D_n × dn/dx
J_p = q × μ_p × p × E − q × D_p × dp/dx
```

These, together with **Poisson's equation** (relating field to net charge):

```
dE/dx = (q / ε_Si) × (p − n + N_D − N_A)
```

and the **continuity equations** (charge conservation with generation/recombination):

```
∂n/∂t = (1/q) × dJ_n/dx + (G_n − R_n)
∂p/∂t = (-1/q) × dJ_p/dx + (G_p − R_p)
```

form the **drift-diffusion model** — a coupled set of nonlinear PDEs that describe every classical semiconductor device. Every formula in `mosfet-models.md` is a special-case solution of these.

We will not solve them in full generality (that is TCAD; see `fab-process-simulation.md` for the corresponding limit). Instead, we apply the **depletion approximation** and **gradual-channel approximation** to derive closed-form results.

## Worked Derivation 1 — The PN Junction Diode

Every junction in a CMOS process is a diode. The source/drain of a MOSFET to its body is a diode. Wells to substrate are diodes. Latching, isolation, and ESD protection all rely on these diodes. We derive `I_diode(V)` from drift-diffusion + depletion approximation.

### Setup

```
   p-region (N_A)        |        n-region (N_D)
   E_F closer to E_V     |        E_F closer to E_C
                         |
   holes majority        |        electrons majority
   electrons minority    |        holes minority
                         ▲
                  metallurgical junction (x = 0)
```

In equilibrium (no applied bias), the Fermi level must be flat (constant E_F across the device — that's what equilibrium means). For E_F to align in p-type and n-type material whose intrinsic Fermi levels sit at different energies, the bands must *bend* near the junction, creating a built-in field.

### Built-in voltage

```
φ_bi = φ_Fp + |φ_Fn| = V_T × ln(N_A × N_D / n_i²)
```

For `N_A = N_D = 10¹⁷ cm⁻³` at 300 K: `φ_bi ≈ 0.836 V`.

### Depletion region

In the depletion approximation, the junction has a region of width W stripped of mobile carriers. Charge neutrality across the junction requires:

```
N_A × x_p = N_D × x_n        (where x_p + x_n = W)
```

Solving Poisson's equation with these boundary conditions gives:

```
W(V) = sqrt( (2 × ε_Si / q) × ((N_A + N_D)/(N_A × N_D)) × (φ_bi − V) )
```

where V is the **applied voltage** (positive when forward-biased). Forward bias *narrows* the depletion region; reverse bias *widens* it.

### Diode current

When the diode is forward-biased by V, the energy barrier shrinks by V, and the minority-carrier concentration at the edge of each depletion region rises by `exp(V / V_T)`. These excess minority carriers diffuse into the neutral region, where they recombine. The total current that supplies this diffusion is:

```
I = I_S × (exp(V / V_T) − 1)
```

with the **saturation current** I_S given by:

```
I_S = q × A × n_i² × ((D_n / (L_n × N_A)) + (D_p / (L_p × N_D)))
```

where:
- A = junction area
- L_n, L_p = minority-carrier diffusion lengths (`L = sqrt(D × τ)`, τ = recombination lifetime)

This is the **Shockley diode equation**. Reverse bias gives I ≈ −I_S (a tiny leakage). Forward bias gives the classic exponential I-V.

### Reverse breakdown

At sufficient reverse bias, two effects can break down the diode:

1. **Avalanche**: carriers accelerated by the high field generate new electron-hole pairs by impact ionization. Cascading. Knee around 5-50 V depending on doping.
2. **Zener (band-to-band tunneling)**: in heavily doped junctions, electrons tunnel from valence to conduction band through the thin depletion region. Knee below ~5 V.

We model breakdown as a single threshold `BV` (breakdown voltage) past which the current rises sharply. SPICE Level-1 uses an exponential break model.

## Worked Derivation 2 — MOSFET Threshold Voltage

The MOSFET is the workhorse of CMOS. We derive the threshold voltage `V_t` — the gate voltage at which an inversion layer forms under the gate, allowing current to flow from source to drain.

### Setup (NMOS)

```
       Gate (poly-Si or metal)
            │
   ┌────────┴──────────┐
   │  Gate oxide (SiO₂, thickness T_ox)│
   ├────────────────────┤
   │  p-type body (N_A)  │
   │                    │
   │  n+ source         n+ drain
   └────────────────────┘
```

Apply a positive gate voltage V_GS. The gate-oxide-body stack is a capacitor; positive charge on the gate pushes holes (the majority carriers in the p-body) away from the oxide interface. As V_GS rises:

1. **Accumulation** (V_GS < V_FB): holes are pulled toward the interface. (Doesn't happen for NMOS in normal operation.)
2. **Depletion** (V_FB < V_GS < V_t): holes are pushed away; a depletion region forms under the gate.
3. **Inversion** (V_GS ≥ V_t): the surface potential becomes positive enough that electrons (the minority carriers in p-body) accumulate at the surface, forming a thin "inversion layer" — the channel.

### Flat-band voltage V_FB

V_FB is the gate voltage at which the bands in the body are flat (no band bending). It captures the work-function difference between the gate and the body, plus any oxide trapped charge:

```
V_FB = φ_MS − Q_ox / C_ox
```

where:
- φ_MS = (work function of gate) − (work function of body) — depends on gate material and body doping.
- Q_ox = oxide-trapped charge per unit area (manufacturing artifact; we treat as a constant).
- C_ox = ε_ox / T_ox = oxide capacitance per unit area.

For poly-Si gate over p-type Si: `φ_MS ≈ −1 V` typically.

### Surface potential at threshold

The body's bulk is at potential `−φ_F` relative to E_i (since p-type). Threshold occurs when the surface is inverted to the same magnitude on the *other* side: `+φ_F` (the surface is now as n-type as the bulk is p-type). So the surface potential at threshold is:

```
ψ_s,inv = 2 × φ_F
```

This is the famous "strong inversion" criterion.

### Depletion charge at threshold

The depletion region under the gate has thickness:

```
W_dep,max = sqrt( (4 × ε_Si × φ_F) / (q × N_A) )
```

The total depletion charge per unit area is:

```
|Q_B| = q × N_A × W_dep,max = sqrt( 4 × ε_Si × q × N_A × φ_F )
```

### Putting it together: V_t

The gate voltage must supply (a) the flat-band offset, (b) the surface potential, (c) the charge to support the depletion region:

```
V_t = V_FB + 2 × φ_F + |Q_B| / C_ox
    = V_FB + 2 × φ_F + sqrt(4 × ε_Si × q × N_A × (2 × φ_F)) / C_ox
    = V_FB + 2 × φ_F + γ × sqrt(2 × φ_F)
```

where:

```
γ = sqrt(2 × ε_Si × q × N_A) / C_ox
```

is the **body-effect coefficient**. This is the threshold formula. With body bias V_SB:

```
V_t = V_FB + 2 × φ_F + γ × sqrt(2 × φ_F + V_SB)
```

The body-effect raises threshold when the source is reverse-biased relative to the body — a real phenomenon in stacked transistors and used deliberately in dynamic threshold control.

### Numerical sanity check

For N_A = 10¹⁷ cm⁻³, T_ox = 5 nm, poly-Si gate (φ_MS ≈ −0.95 V), Q_ox negligible, T = 300 K:

- `φ_F = 0.0259 × ln(10¹⁷ / 10¹⁰) = 0.418 V`
- `2 × φ_F = 0.836 V`
- `C_ox = 3.9 × 8.854e-12 / 5e-9 ≈ 6.91 mF/m² ≈ 6.91 × 10⁻⁷ F/cm²`
- `γ = sqrt(2 × 11.7 × 8.854e-12 × 1.602e-19 × 10²³) / 6.91e-3 ≈ 0.27 V^(1/2)`
- `V_t = -0.95 + 0.836 + 0.27 × sqrt(0.836) ≈ 0.13 V`

That's a low V_t — typical of a modern thin-oxide process. Older / longer-channel processes had V_t closer to 0.7 V.

## Worked Derivation 3 — MOSFET I-V (Square Law)

Once the channel is inverted (V_GS > V_t), apply V_DS to drive current from drain to source. We derive the classical "square law" current using the **gradual-channel approximation**: the channel is long enough that vertical (gate-driven) and horizontal (drain-driven) fields decouple.

### Charge in the inversion layer

At a position x along the channel (0 = source, L = drain), the channel-to-gate voltage is `V_GS − V(x)` where V(x) is the channel potential at x (V(0)=0, V(L)=V_DS). The inversion charge per unit area is:

```
|Q_n(x)| = C_ox × (V_GS − V(x) − V_t)
```

(only valid for V_GS − V(x) > V_t, i.e., the channel is inverted at x).

### Current along the channel

Current density J = drift current of electrons:

```
I_D = W × |Q_n(x)| × μ_n × dV/dx
```

This must be constant along x (current conservation). Separating variables:

```
I_D × dx = W × μ_n × C_ox × (V_GS − V_t − V(x)) × dV
```

Integrating x from 0 to L and V from 0 to V_DS:

```
I_D × L = W × μ_n × C_ox × [ (V_GS − V_t) × V_DS − V_DS² / 2 ]
```

Hence:

```
I_D = (μ_n × C_ox × W / L) × [ (V_GS − V_t) × V_DS − V_DS² / 2 ]    (triode region)
```

This holds while the channel is inverted everywhere — i.e., at the drain end, `V_GS − V_DS > V_t`, or `V_DS < V_GS − V_t`.

### Saturation

When V_DS = V_GS − V_t, the inversion charge at the drain end goes to zero — the channel "pinches off." Increasing V_DS further extends the pinch-off point slightly toward the source, but I_D becomes (to first order) independent of V_DS:

```
I_D = (1/2) × (μ_n × C_ox × W / L) × (V_GS − V_t)²    (saturation)
```

This is the **square law** — the iconic MOSFET equation. Including channel-length modulation (the slight rise of I_D in saturation as V_DS increases) gives:

```
I_D = (1/2) × (μ_n × C_ox × W / L) × (V_GS − V_t)² × (1 + λ × V_DS)
```

where λ is an empirical fitting parameter (the SPICE Level-1 LAMBDA).

### Operating regions summary

| Condition | Region | Current |
|---|---|---|
| V_GS ≤ V_t | Cutoff | I_D ≈ 0 (subthreshold leakage; see below) |
| V_GS > V_t and V_DS < V_GS − V_t | Triode (linear) | I_D = β × [(V_GS − V_t) × V_DS − V_DS²/2] |
| V_GS > V_t and V_DS ≥ V_GS − V_t | Saturation | I_D = (β/2) × (V_GS − V_t)² × (1 + λ × V_DS) |

where β = μ_n × C_ox × W / L is the **transconductance parameter** (units: A/V²).

## Worked Derivation 4 — Subthreshold Conduction

The square-law model says I_D = 0 below threshold. The truth: there's a small but non-zero current that decreases exponentially with V_GS. This subthreshold leakage matters for low-power design and is the dominant leakage source in modern processes.

In subthreshold, the surface is depleted but not yet strongly inverted. The minority-carrier concentration at the surface depends exponentially on surface potential, which depends linearly on V_GS through the body coefficient. Result:

```
I_D ≈ I_S0 × (W/L) × exp((V_GS − V_t) / (n × V_T)) × (1 − exp(−V_DS / V_T))
```

where:
- n is the **subthreshold slope factor**, typically 1.0 to 1.5. Bigger n = worse switching.
- The (1 − exp(−V_DS / V_T)) factor saturates to 1 when V_DS > ~3 × V_T (~75 mV).

The **subthreshold slope** S = n × V_T × ln(10) gives mV per decade of current. Best possible is `n = 1`, giving S ≈ 60 mV/dec at room T. Real processes are 70-100 mV/dec.

## Worked Derivation 5 — Mobility Degradation

Real mobility is not constant. Two effects matter:

### Vertical-field degradation

A high gate field (large V_GS) confines carriers to a thin inversion layer where they scatter more frequently off the oxide-semiconductor interface. Empirically:

```
μ_eff = μ₀ / (1 + θ × (V_GS − V_t))
```

where θ is the **vertical-field mobility coefficient** (~0.1 V⁻¹ typically). This makes `I_D` grow slower than `(V_GS − V_t)²` in real devices.

### Velocity saturation

At high lateral fields (large V_DS / L), drift velocity saturates to ~10⁷ cm/s for electrons in silicon. The square-law breaks down:

```
v_drift = μ × E / (1 + E / E_crit)    (Caughey-Thomas)
```

where E_crit ≈ 1.5 × 10⁴ V/cm for electrons. In short-channel devices (L < 100 nm), this is the dominant departure from square law and produces the linear-in-V_GS saturation current characteristic of deep-submicron transistors:

```
I_D,sat ≈ W × C_ox × (V_GS − V_t) × v_sat    (velocity-saturated regime)
```

## Concept: Why CMOS works

A CMOS inverter has one PMOS and one NMOS in series between V_DD and GND, gates tied:

```
                      V_DD
                       │
                  ┌────┴────┐
                  │ PMOS    │   ── source at V_DD
            G ────┤         │
                  │   D     │
                  └────┬────┘
                       │ ── output
                  ┌────┴────┐
                  │ NMOS    │   ── source at GND
            G ────┤         │
                  │   D     │
                  └────┬────┘
                       │
                      GND
```

When the input is at V_DD: NMOS on, PMOS off → output pulled to GND. When the input is at GND: NMOS off, PMOS on → output pulled to V_DD. In static state, *one* transistor is always cut off, so the static current is just the subthreshold leakage of the off device — typically pA per gate at 1 V V_DD.

Dynamic power comes from charging/discharging the load capacitance during transitions: `P_dyn = α × C_L × V_DD² × f`. This is why power scales with `V_DD²`: every halving of supply voltage cuts dynamic power by 4×.

## Worked Derivation 6 — Oxide Capacitance and Gate Tunneling

`C_ox = ε_ox / T_ox` is straightforward parallel-plate capacitance. As T_ox shrinks below ~3 nm, electrons can quantum-mechanically tunnel through the oxide ("gate leakage"). The tunneling current density follows Fowler-Nordheim or direct-tunneling formulas:

```
J_tunnel ≈ A × E_ox² × exp(−B / E_ox)    (Fowler-Nordheim, high field)
```

where E_ox = V_GS / T_ox. For T_ox < 2 nm, gate leakage rivals subthreshold leakage as a source of static power. The industry response: **high-K dielectrics** (HfO₂, ε_K ~ 25, replaces SiO₂, ε_ox = 3.9). Higher K means thicker physical T_ox for the same C_ox, so less tunneling.

In our (Sky130-shaped) PDK, oxide is thick enough that we can neglect gate tunneling. Document this assumption explicitly.

## Concept: Temperature dependence

Most parameters drift with T:

| Parameter | Temperature dependence |
|---|---|
| n_i | n_i(T) = n_i(300) × (T/300)^(3/2) × exp(−(E_g/2k) × (1/T − 1/300)) — strongly increases with T |
| V_T = kT/q | Linear in T |
| μ | μ(T) ≈ μ(300) × (T/300)^(−2.4) — decreases with T (more phonon scattering) |
| φ_F | (kT/q) × ln(N/n_i) — depends on T through both factors; typically slightly decreases |
| V_t | Decreases ~1-3 mV/K (φ_F decrease dominates over thinner depletion) |
| I_S (diode) | Strongly increases with T — diode currents roughly double per 10 K |

For digital circuits, the practical impact: hotter chips have lower V_t (faster switching, but higher leakage). Worst-case timing is usually at high T (slow), and worst-case leakage is at high T (high). PVT corners (process-voltage-temperature) capture this; standard cell library characterization (`standard-cell-library.md`) sweeps T across 3+ corners.

## Public API (Python)

```python
from dataclasses import dataclass
from math import sqrt, exp, log


# ═══════════════════════════════════════════════════════════════
# Physical constants
# ═══════════════════════════════════════════════════════════════

K_BOLTZMANN  = 1.380649e-23      # J/K
Q_ELECTRON   = 1.602177e-19      # C
EPS0         = 8.854188e-12      # F/m
EPS_SI       = 11.7 * EPS0
EPS_OX       = 3.9 * EPS0
N_I_300K     = 1.0e16            # /m³ (1.0e10 /cm³)
N_C          = 2.8e25            # /m³ (effective DOS conduction band)
N_V          = 1.04e25           # /m³ (effective DOS valence band)
EG_SI_300K   = 1.12              # eV
MU_N_300K    = 1350e-4           # m²/V·s (lightly doped Si)
MU_P_300K    = 480e-4            # m²/V·s


# ═══════════════════════════════════════════════════════════════
# Helpers
# ═══════════════════════════════════════════════════════════════

def thermal_voltage(T: float = 300.0) -> float:
    """V_T = kT/q. Returns volts."""
    return K_BOLTZMANN * T / Q_ELECTRON


def intrinsic_concentration(T: float = 300.0) -> float:
    """n_i(T). Returns /m³."""
    if T == 300.0:
        return N_I_300K
    factor = (T / 300.0) ** 1.5
    bandgap_term = exp(-(EG_SI_300K / (2.0 * thermal_voltage(T))) *
                       (1.0 - T / 300.0))
    # Approximate; full model requires bandgap T-dependence.
    return N_I_300K * factor * bandgap_term


def fermi_potential(N: float, *, kind: str, T: float = 300.0) -> float:
    """Fermi potential φ_F. kind ∈ {'p','n'}; N in /m³.
    Returns positive value for p-type, negative for n-type."""
    n_i = intrinsic_concentration(T)
    magnitude = thermal_voltage(T) * log(N / n_i)
    return +magnitude if kind == "p" else -magnitude


# ═══════════════════════════════════════════════════════════════
# PN junction
# ═══════════════════════════════════════════════════════════════

@dataclass(frozen=True)
class PNJunction:
    N_A: float          # acceptor doping in p-side (/m³)
    N_D: float          # donor doping in n-side  (/m³)
    A: float            # area (m²)
    T: float = 300.0    # temperature (K)
    tau_n: float = 1e-6 # electron lifetime (s)
    tau_p: float = 1e-6 # hole lifetime (s)

    def built_in_voltage(self) -> float:
        n_i = intrinsic_concentration(self.T)
        return thermal_voltage(self.T) * log((self.N_A * self.N_D) / (n_i ** 2))

    def depletion_width(self, V_applied: float = 0.0) -> float:
        phi_bi = self.built_in_voltage()
        return sqrt(
            (2.0 * EPS_SI / Q_ELECTRON) *
            ((self.N_A + self.N_D) / (self.N_A * self.N_D)) *
            (phi_bi - V_applied)
        )

    def saturation_current(self) -> float:
        # I_S = q*A*n_i² * (D_n/(L_n*N_A) + D_p/(L_p*N_D))
        n_i = intrinsic_concentration(self.T)
        V_T = thermal_voltage(self.T)
        D_n = MU_N_300K * V_T
        D_p = MU_P_300K * V_T
        L_n = sqrt(D_n * self.tau_n)
        L_p = sqrt(D_p * self.tau_p)
        return (Q_ELECTRON * self.A * n_i ** 2 *
                (D_n / (L_n * self.N_A) + D_p / (L_p * self.N_D)))

    def current(self, V: float) -> float:
        """Shockley diode equation."""
        V_T = thermal_voltage(self.T)
        return self.saturation_current() * (exp(V / V_T) - 1.0)


# ═══════════════════════════════════════════════════════════════
# MOSFET parameters (NMOS or PMOS, computed from physical params)
# ═══════════════════════════════════════════════════════════════

@dataclass(frozen=True)
class MOSFETParams:
    type: str           # 'NMOS' or 'PMOS'
    L: float            # channel length (m)
    W: float            # channel width (m)
    T_ox: float         # gate-oxide thickness (m)
    N_body: float       # body doping (/m³); N_A for NMOS, N_D for PMOS
    phi_MS: float       # gate-body work-function difference (V)
    Q_ox: float = 0.0   # oxide trapped charge per area (C/m²)
    T: float = 300.0    # temperature (K)

    @property
    def C_ox(self) -> float:
        return EPS_OX / self.T_ox

    @property
    def V_FB(self) -> float:
        return self.phi_MS - self.Q_ox / self.C_ox

    @property
    def phi_F(self) -> float:
        body_kind = "p" if self.type == "NMOS" else "n"
        return abs(fermi_potential(self.N_body, kind=body_kind, T=self.T))

    @property
    def gamma(self) -> float:
        return sqrt(2.0 * EPS_SI * Q_ELECTRON * self.N_body) / self.C_ox

    def threshold_voltage(self, V_SB: float = 0.0) -> float:
        V_t0 = self.V_FB + 2.0 * self.phi_F + self.gamma * sqrt(2.0 * self.phi_F)
        # Body-effect correction:
        return V_t0 + self.gamma * (sqrt(2.0 * self.phi_F + V_SB) -
                                    sqrt(2.0 * self.phi_F))
```

## Edge Cases

| Scenario | Handling |
|---|---|
| Zero applied voltage on diode | I = 0 (Shockley exp(0)−1 = 0). |
| Reverse bias > breakdown voltage | Out of model scope — caller must clamp; Level-1 SPICE has a soft-breakdown extension. |
| V_GS = V_t exactly | Square law gives I_D = 0; subthreshold model gives ≈ I_S0 × W/L. There's a "moderate inversion" smoothing region that BSIM models; we use a hard switch at V_t in Level-1. |
| Negative N_body (pathological) | Validate; reject. |
| T near 0 K | n_i → 0; `log(N/n_i) → ∞`. Reject T below 100 K — model invalid. |
| Very heavy doping (degenerate semiconductor, N > 10²⁰ /cm³) | Boltzmann statistics break down. Document as a model limitation. |
| Velocity-saturated regime | Square-law overpredicts I_D,sat. Use the velocity-saturation formula for L < 100 nm. |
| Body-source forward biased | V_SB < 0 — the body-source diode conducts. Out of normal NMOS operation; flag as a warning. |

## Test Strategy

### Unit
- `intrinsic_concentration(300) == 1.0e16` (m⁻³ basis).
- `thermal_voltage(300) ≈ 0.02585 V` (within 0.1%).
- `fermi_potential(1e23, kind="p") ≈ 0.418 V` (matches numerical sanity check above).
- `PNJunction(N_A=1e23, N_D=1e23).built_in_voltage() ≈ 0.836 V`.
- `PNJunction(...).current(0.6) ≈ exp(0.6/0.0259) × I_S` (within 1%).
- `MOSFETParams(NMOS, L=180n, W=1u, T_ox=4n, N_body=1e23, phi_MS=-0.95).threshold_voltage() ≈ 0.13 V`.
- Body effect: `V_t(V_SB=2) > V_t(V_SB=0)`.

### Property
- Symmetry: NMOS and PMOS with equivalent physical parameters give equal-magnitude V_t.
- Monotonicity: V_t increases with N_body (heavier doping → harder to invert).
- Monotonicity: V_t decreases with C_ox (thinner oxide → easier to invert).
- Diode current is monotonically increasing in V (forward) and asymptotes to −I_S in reverse.
- Diode current is exponential within 5% over 0.2 V to 0.7 V forward bias.

### Integration
- Use these functions to compute parameters for a 130 nm-style transistor; feed into `mosfet-models.md` Level-1 model; SPICE-simulate a CMOS inverter; verify switching threshold ≈ V_DD / 2.
- Verify that the existing `transistors` package (which currently uses behavioral models) can be extended with this physics — the API surface here matches what `mosfet-models.md` will need.

## Conformance & Caveats

| Topic | Coverage |
|---|---|
| **Boltzmann statistics regime** (non-degenerate doping ≤ 10¹⁹ /cm³) | Full |
| **Degenerate semiconductors** (N > 10²⁰ /cm³) | Out of scope; flagged. |
| **Quantum effects** (poly depletion, channel quantization) | Out of scope; documented as future work. |
| **Bandgap narrowing** (heavy doping) | Out of scope. |
| **Trap-assisted tunneling, GIDL, BTBT** | Out of scope. |
| **Hot carriers, NBTI/PBTI aging** | Out of scope; documented as future. |
| **Gate tunneling** | Modeled qualitatively; Sky130 process is thick enough to neglect quantitatively. |
| **High-K / metal-gate stacks** | Constants are for SiO₂; high-K requires re-parameterization. |

## Open Questions

1. **Should we represent the four-terminal MOSFET (G, D, S, B) or merge B with S as is common in digital simulation?** — Default: four-terminal so body-effect is correctly modeled. Sky130 PDK has explicit body terminals; we follow.

2. **How to handle the PMOS sign convention?** — Recommend: store the *physical* equations (signed) but also expose a `magnitude_threshold()` helper that flips PMOS sign for SPICE compatibility.

3. **Do we want a separate "moderate inversion" smoothing function for V_GS ≈ V_t?** — Level-1 doesn't smooth; convergence-friendly extensions exist. Defer to `mosfet-models.md`.

4. **Are mobility tables (μ vs doping) precomputed or always derived?** — Recommend: tables built into the module from canonical references (e.g., Caughey-Thomas with Si parameters); user can override for non-Si.

## Future Work

- Bandgap narrowing model for heavy doping.
- Quantum corrections (polysilicon depletion, surface quantization).
- Aging models (NBTI/PBTI) for reliability simulation.
- High-K / metal-gate parameter sets.
- Compact non-quasi-static (NQS) charge model for high-frequency.
- Self-heating thermal coupling (for power MOSFETs / high-current digital).
- TCAD-style PDE solver for cases where analytical models break down — bridge to `fab-process-simulation.md`.
