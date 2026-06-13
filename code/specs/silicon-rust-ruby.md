# Specification: silicon-rust-ruby

## Overview

`silicon_rust_ruby` is a Ruby gem that exposes the Rust silicon simulation stack
(`device-physics`, `mosfet-models`, `fab-process-simulation`) to Ruby programs
via a zero-dependency native extension.

The native extension is built by the Rust crate `silicon-rust-ruby-native`, which
lives in the workspace at `code/packages/rust/silicon-rust-ruby-native/`.  It
uses `ruby-bridge` (raw `extern "C"` declarations against the Ruby C API — no
Magnus, no rb-sys, no bindgen, no Ruby headers at build time) and follows the
same architecture as `matrix-rust-ruby-native`.

---

## Architecture

```
silicon_rust_ruby (Ruby gem)
├── lib/coding_adventures/silicon_rust_ruby.rb      — façade + namespace alias
├── lib/coding_adventures/silicon_rust_ruby/
│   ├── version.rb                                  — VERSION constant
│   └── native_loader.rb                            — require .{so,bundle,dll}
├── ext/silicon_rust_ruby_native/
│   ├── build_config.rb                             — cargo invocation helpers
│   └── extconf.rb                                  — generates Makefile for gem install
├── test/silicon_rust_ruby_test.rb                  — minitest suite
├── Gemfile / gemspec / Rakefile
└── BUILD / BUILD_windows

silicon-rust-ruby-native (Rust workspace crate)
├── src/lib.rs         — Init_silicon_rust_ruby_native + 26 Ruby methods
├── build.rs           — macOS: -undefined dynamic_lookup; Windows: link libruby
├── Cargo.toml
└── BUILD / BUILD_windows
```

---

## Ruby-visible API

All functions are defined as **module functions** on `SiliconRustRuby` (also
accessible as `CodingAdventures::SiliconRustRuby` via a delegating alias in
the gem's `.rb` façade).

Ruby naming uses `snake_case` throughout.  Argument types are documented as
Ruby types.

### Physical constants (no arguments)

| Method | Return value | Unit |
|--------|-------------|------|
| `k_boltzmann` | `Float` | J/K |
| `q_electron` | `Float` | C |
| `eps0` | `Float` | F/m |
| `eps_si` | `Float` | F/m |
| `eps_ox` | `Float` | F/m |
| `ni_at_300k` | `Float` | /m³ |
| `eg_si_at_300k` | `Float` | eV |
| `mu_n_300k` | `Float` | m²/V·s |
| `mu_p_300k` | `Float` | m²/V·s |

### device-physics

```ruby
SiliconRustRuby.thermal_voltage(t_kelvin)                             # → Float [V]
SiliconRustRuby.intrinsic_concentration(t_kelvin)                     # → Float [/m³]
SiliconRustRuby.fermi_potential(n_doping, kind, t_kelvin)             # → Float [V]
SiliconRustRuby.pn_junction_built_in_voltage(na, nd, t)               # → Float [V]
SiliconRustRuby.pn_junction_depletion_width(na, nd, t, v_applied)     # → Float [m]
SiliconRustRuby.pn_junction_saturation_current(na, nd, a, t, tau_n, tau_p)  # → Float [A]
SiliconRustRuby.pn_junction_current(na, nd, a, t, tau_n, tau_p, v)   # → Float [A]
SiliconRustRuby.mosfet_threshold_voltage(device_type, l, w, t_ox, n_body,
                                         phi_ms, q_ox, t, v_sb)       # → Float [V]
```

`kind` and `device_type` are `String` (`"NMOS"` or `"PMOS"`).

### mosfet-models

```ruby
SiliconRustRuby.evaluate_level1(vt0, kp, lambda, gamma, phi, w, l,
                                n_sub, v_gs, v_ds, v_bs, t)  # → Hash
SiliconRustRuby.evaluate_level1_defaults(v_gs, v_ds, v_bs, t)        # → Hash
```

The returned `Hash` has symbol keys:

```ruby
{
  id:     Float,   # drain current [A]
  gm:     Float,   # transconductance [S]
  gds:    Float,   # drain-source conductance [S]
  gmb:    Float,   # body-transconductance [S]
  cgs:    Float,   # gate-source capacitance [F]
  cgd:    Float,   # gate-drain capacitance [F]
  cgb:    Float,   # gate-body capacitance [F]
  cbs:    Float,   # body-source capacitance [F]
  cbd:    Float,   # body-drain capacitance [F]
  region: String,  # "cutoff" | "subthreshold" | "triode" | "saturation"
}
```

### fab-process-simulation

```ruby
SiliconRustRuby.deal_grove_oxidation(cs_str, time_min)               # → String
SiliconRustRuby.deal_grove_oxidation(cs_str, time_min, a_um, b_um2_per_hr) # → String (custom A/B)
SiliconRustRuby.deposit(cs_str, material, thickness_nm)               # → String
SiliconRustRuby.etch(cs_str, target_material, depth_nm)               # → String
SiliconRustRuby.implant(cs_str, species, energy_kev, dose_cm2)        # → String
SiliconRustRuby.diffuse(cs_str, time_min)                             # → String
SiliconRustRuby.diffuse(cs_str, time_min, temperature_c)              # → String (custom T)
SiliconRustRuby.implant_range(species, energy_kev)                    # → Hash {rp:, straggle:}
SiliconRustRuby.diffusivity_cm2_per_s(species, temperature_c)         # → Float [cm²/s]
```

`deal_grove_oxidation` and `diffuse` accept optional arguments and are registered
with `argc = -1` (variadic Ruby C convention).

**Cross-section wire format** — `cs_str` is a pipe-separated `material:thickness_nm`
string, ordered top-to-bottom.  An empty cross-section is `""`.

```ruby
""                               # bare (no layers)
"Si:500.0"                       # 500 nm silicon substrate
"SiO2:4.8|Si:500.0"             # gate oxide on silicon
"Poly:50.0|SiO2:4.8|Si:500.0"  # poly gate on oxide on silicon
```

Material names must not contain `|` or `:` — `deposit` enforces this to prevent
wire-format injection.

---

## Rust crate: silicon-rust-ruby-native

### Entry point

Ruby calls `Init_silicon_rust_ruby_native` when it `dlopen()`s the `.so`.
The function:
1. Defines top-level module `SiliconRustRuby`
2. Registers all 26 functions via `rb_define_module_function`

### Helpers (pure Rust)

```rust
fn validate_material_name(material: &str) -> Result<(), String>
fn cs_to_wire(cs: &fps::CrossSection) -> String
fn cs_from_wire(s: &str) -> fps::CrossSection
```

These are used by the N-API and Python bindings with identical logic.

### Platform behaviour

| Platform | Shared library | Ruby symbols |
|----------|---------------|-------------|
| Linux    | `.so`         | Resolved at `dlopen()` by ELF dynamic loader |
| macOS    | `.bundle`/`.dylib` | `-undefined dynamic_lookup` via `build.rs` |
| Windows  | `.dll`        | Linked against `libruby.dll` via `build.rs` |

---

## Testing

Tests are written in Ruby using minitest and live in `test/silicon_rust_ruby_test.rb`.
They verify the FFI boundary ("did the Ruby call round-trip correctly?"), not the
underlying Rust math (which is covered by the crate unit tests).

Minimum coverage:
- All 9 constant accessors return numeric values
- `thermal_voltage(300)` returns approximately 0.025852
- `pn_junction_built_in_voltage` returns a positive Float
- `mosfet_threshold_voltage` returns a Float
- `evaluate_level1_defaults` returns a Hash with all 10 keys including `:region`
- `evaluate_level1` saturation path returns `{ region: "saturation" }`
- `deposit` builds a wire string with the new layer on top
- `deal_grove_oxidation` returns a String starting with `"SiO2:"`
- `etch` removes a layer
- `implant_range` returns a Hash with `:rp` and `:straggle` keys
- `diffusivity_cm2_per_s` returns a positive Float
- `deposit` raises `RuntimeError` when material name contains `|`
- `deal_grove_oxidation` raises `RuntimeError` for non-positive `time_min`
- Non-String arguments raise `TypeError` or `RuntimeError`

---

## File layout

```
code/packages/rust/silicon-rust-ruby-native/
  Cargo.toml
  build.rs
  src/lib.rs
  BUILD
  BUILD_windows
  README.md
  CHANGELOG.md

code/packages/ruby/silicon_rust_ruby/
  coding_adventures_silicon_rust_ruby.gemspec
  Gemfile
  Rakefile
  BUILD
  BUILD_windows
  README.md
  CHANGELOG.md
  ext/silicon_rust_ruby_native/
    build_config.rb
    extconf.rb
  lib/
    coding_adventures/
      silicon_rust_ruby.rb
      silicon_rust_ruby/
        version.rb
        native_loader.rb
  test/
    silicon_rust_ruby_test.rb
```

---

## Dependencies

**Rust crate:**
- `device-physics` (workspace path)
- `mosfet-models` (workspace path)
- `fab-process-simulation` (workspace path)
- `ruby-bridge` (workspace path)

**Ruby gem (development):**
- `minitest ~> 5.0`
- `rake ~> 13.0`
