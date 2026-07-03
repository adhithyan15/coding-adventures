# silicon_rust_ruby

Ruby gem that exposes the Rust silicon simulation stack — `device-physics`,
`mosfet-models`, and `fab-process-simulation` — to Ruby programs via a
zero-dependency native extension.

The native extension is built by the workspace Rust crate
`silicon-rust-ruby-native`, which uses `ruby-bridge` (raw `extern "C"` Ruby
C API — no Magnus, no rb-sys, no bindgen, no Ruby headers at build time).

## How it fits in the stack

```
silicon_rust_ruby (this gem)
  ↓ require
silicon_rust_ruby_native.{so,bundle,dll}
  ↓ Rust function calls
device-physics   mosfet-models   fab-process-simulation
```

## Installation

```bash
cd code/packages/ruby/silicon_rust_ruby
bundle install
bundle exec rake compile
bundle exec rake test
```

## Usage

```ruby
require "coding_adventures/silicon_rust_ruby"

# Physical constants
SiliconRustRuby.k_boltzmann   # => 1.380649e-23 J/K
SiliconRustRuby.thermal_voltage(300) # => 0.025852 V

# PN junction physics
vbi = SiliconRustRuby.pn_junction_built_in_voltage(1e23, 1e22, 300)
w   = SiliconRustRuby.pn_junction_depletion_width(1e23, 1e22, 300, 0.0)
is_ = SiliconRustRuby.pn_junction_saturation_current(1e23, 1e22, 1e-8, 300, 1e-6, 1e-6)
i   = SiliconRustRuby.pn_junction_current(1e23, 1e22, 1e-8, 300, 1e-6, 1e-6, 0.6)

# MOSFET threshold voltage
vt = SiliconRustRuby.mosfet_threshold_voltage("NMOS", 130e-9, 1e-6, 2e-9, 1e24, -0.05, 0, 300, 0)

# Level-1 MOSFET DC operating point (default 130 nm NMOS params)
r = SiliconRustRuby.evaluate_level1_defaults(1.8, 1.8, 0.0, 300.15)
r[:region]  # => "saturation"
r[:id]      # => drain current [A]

# Full Level-1 with explicit parameters
r2 = SiliconRustRuby.evaluate_level1(
  0.42,   # vt0 [V]
  220e-6, # kp [A/V²]
  0.05,   # lambda [1/V]
  0.27,   # gamma [√V]
  0.84,   # phi [V]
  1e-6,   # w [m]
  130e-9, # l [m]
  1.4,    # n_sub
  1.8,    # v_gs [V]
  1.8,    # v_ds [V]
  0.0,    # v_bs [V]
  300.15  # t [K]
)

# Process simulation
cs = SiliconRustRuby.deposit("", "Si", 500.0)      # Si substrate
cs = SiliconRustRuby.deal_grove_oxidation(cs, 5.0)  # grow gate oxide
cs = SiliconRustRuby.deposit(cs, "Poly", 50.0)      # deposit poly gate
# cs => "Poly:50.0|SiO2:...|Si:500.0"

cs = SiliconRustRuby.implant(cs, "B", 30.0, 1e13)   # boron implant
cs = SiliconRustRuby.diffuse(cs, 30.0, 1000.0)      # 30-min anneal

range = SiliconRustRuby.implant_range("B", 30.0)     # => { rp: 92.0, straggle: 38.0 }
d     = SiliconRustRuby.diffusivity_cm2_per_s("B", 1000.0)  # => 1e-14

# Namespaced alias works too
CodingAdventures::SiliconRustRuby.thermal_voltage(300)
```

## Cross-section wire format

A `CrossSection` is serialised as a pipe-separated list of
`material:thickness_nm` pairs, ordered top-to-bottom:

```
""                               # empty cross-section
"Si:500.0"                       # bare silicon substrate, 500 nm
"SiO2:4.8|Si:500.0"             # gate oxide on silicon
"Poly:50.0|SiO2:4.8|Si:500.0"  # poly gate on gate oxide on silicon
```

Material names must not contain `|` or `:` — `deposit` enforces this.

## API reference

### Physical constants

All take no arguments and return `Float`.

| Method | Value | Unit |
|--------|-------|------|
| `k_boltzmann` | 1.380649×10⁻²³ | J/K |
| `q_electron` | 1.602176634×10⁻¹⁹ | C |
| `eps0` | 8.8541878×10⁻¹² | F/m |
| `eps_si` | 1.0359×10⁻¹⁰ | F/m |
| `eps_ox` | 3.4531×10⁻¹¹ | F/m |
| `ni_at_300k` | 1×10¹⁶ | /m³ |
| `eg_si_at_300k` | 1.12 | eV |
| `mu_n_300k` | 0.1350 | m²/V·s |
| `mu_p_300k` | 0.0480 | m²/V·s |

### device-physics

```
thermal_voltage(t_kelvin)                              → Float [V]
intrinsic_concentration(t_kelvin)                      → Float [/m³]
fermi_potential(n_doping, kind, t_kelvin)              → Float [V]  # kind: "p" or "n"
pn_junction_built_in_voltage(na, nd, t)                → Float [V]
pn_junction_depletion_width(na, nd, t, v_applied)      → Float [m]
pn_junction_saturation_current(na, nd, a, t, tau_n, tau_p) → Float [A]
pn_junction_current(na, nd, a, t, tau_n, tau_p, v)    → Float [A]
mosfet_threshold_voltage(device_type, l, w, t_ox, n_body,
                         phi_ms, q_ox, t, v_sb)        → Float [V]
```

`device_type` is `"NMOS"` or `"PMOS"`.

### mosfet-models

```
evaluate_level1(vt0, kp, lambda, gamma, phi, w, l,
                n_sub, v_gs, v_ds, v_bs, t) → Hash
evaluate_level1_defaults(v_gs, v_ds, v_bs, t) → Hash

Hash keys: :id, :gm, :gds, :gmb, :cgs, :cgd, :cgb, :cbs, :cbd [Float]
           :region [String: "cutoff"|"subthreshold"|"triode"|"saturation"]
```

### fab-process-simulation

```
deal_grove_oxidation(cs_str, time_min [, a_um, b_um2_per_hr]) → String
deposit(cs_str, material, thickness_nm)                         → String
etch(cs_str, target_material, depth_nm)                         → String
implant(cs_str, species, energy_kev, dose_cm2)                  → String
diffuse(cs_str, time_min [, temperature_c])                     → String
implant_range(species, energy_kev)     → Hash { rp: Float, straggle: Float }
diffusivity_cm2_per_s(species, temperature_c) → Float
```

## Platform notes

| Platform | Shared library | Linking |
|----------|---------------|---------|
| Linux | `.so` | ELF resolves at `dlopen()` |
| macOS | `.bundle`/`.dylib` | `-undefined dynamic_lookup` via `build.rs` |
| Windows | `.dll` | `build.rs` links `libruby` via rbconfig |

## Testing

```bash
bundle exec rake test
```

33 minitest tests covering constants, physics, process simulation, hash
results, error cases, and namespace alias.
