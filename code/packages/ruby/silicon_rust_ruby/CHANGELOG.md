# Changelog — silicon_rust_ruby

## [0.1.0] — 2026-06-13

### Added

Initial release.  Ruby gem exposing the Rust silicon simulation stack to Ruby.

**26 module functions on `SiliconRustRuby`:**

*Physical constants (9):*
`k_boltzmann`, `q_electron`, `eps0`, `eps_si`, `eps_ox`, `ni_at_300k`,
`eg_si_at_300k`, `mu_n_300k`, `mu_p_300k`

*`device-physics` (8):*
`thermal_voltage`, `intrinsic_concentration`, `fermi_potential`,
`pn_junction_built_in_voltage`, `pn_junction_depletion_width`,
`pn_junction_saturation_current`, `pn_junction_current`,
`mosfet_threshold_voltage`

*`mosfet-models` (2):*
`evaluate_level1` (12 args), `evaluate_level1_defaults` (4 args)

*`fab-process-simulation` (7):*
`deal_grove_oxidation` (2 or 4 args), `deposit`, `etch`, `implant`,
`diffuse` (2 or 3 args), `implant_range`, `diffusivity_cm2_per_s`

**`CodingAdventures::SiliconRustRuby`** namespace alias — all 26 methods
are delegated from the namespaced module to the top-level `SiliconRustRuby`.

**Hash return values** — `evaluate_level1` returns a Ruby Hash with symbol
keys `:id`, `:gm`, `:gds`, `:gmb`, `:cgs`, `:cgd`, `:cgb`, `:cbs`, `:cbd`,
`:region`.  `implant_range` returns `{ rp: Float, straggle: Float }`.

**Wire format injection guard** — `deposit` rejects material names containing
`|` or `:` to prevent cross-section wire-format corruption.

**33 minitest tests** covering constants, PN junction physics, MOSFET threshold
voltage, Level-1 evaluation, process simulation, error cases, and the
namespace alias.

### Dependencies (development)

- `minitest ~> 5.0`
- `rake ~> 13.0`
