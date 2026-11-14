# Changelog — silicon-rust-ruby-native

## Unreleased - complete the Windows import-library symbol list

- `ext/**/build.rs` generates the MSVC import library for Ruby's DLL from a hand-written list of
  `rb_*` symbols. The list is now a superset of every symbol `ruby-bridge` declares (28 functions
  and 4 data symbols), so a symbol the bridge uses can never be missing at link time. Before, a
  crate whose list lagged the bridge failed to link on Windows with `rust-lld: undefined symbol:
  rb_str_new` / `rb_hash_new` / ... (`ruby/graph_native` in the scheduled full build; the same cause
  as the earlier `bitset_native`, `tree_set_native` and `board_vm_native` fixes). Every added name
  is exported by libruby. Validated by `cargo check` on Linux and by checking each name against
  libruby's exports; the Windows link itself is proven by the next daily Windows full build.

## [0.1.0] — 2026-06-13

### Added

Initial release.  Ruby native extension for the silicon simulation stack.

**26 exported functions on the `SiliconRustRuby` module:**

*Physical constants (9):*
`k_boltzmann`, `q_electron`, `eps0`, `eps_si`, `eps_ox`, `ni_at_300k`,
`eg_si_at_300k`, `mu_n_300k`, `mu_p_300k`

*`device-physics` (8):*
`thermal_voltage`, `intrinsic_concentration`, `fermi_potential`,
`pn_junction_built_in_voltage`, `pn_junction_depletion_width`,
`pn_junction_saturation_current`, `pn_junction_current`,
`mosfet_threshold_voltage`

*`mosfet-models` (2):*
`evaluate_level1`, `evaluate_level1_defaults`

*`fab-process-simulation` (7):*
`deal_grove_oxidation`, `deposit`, `etch`, `implant`, `diffuse`,
`implant_range`, `diffusivity_cm2_per_s`

**Architecture** — Zero-dependency approach using `ruby-bridge` (raw
`extern "C"` Ruby C API declarations).  No Magnus, no rb-sys, no bindgen,
no Ruby headers at build time.

**Wire format** — `CrossSection` travels as a pipe-separated
`material:thickness_nm` string across the FFI boundary (same format as
`silicon-rust-python` and `silicon-rust-napi`).  Material names are
validated against wire-format delimiter injection (`|`, `:`).

**Hash results** — `evaluate_level1` / `evaluate_level1_defaults` return
a Ruby Hash with symbol keys (`:id`, `:gm`, `:gds`, `:gmb`, `:cgs`, `:cgd`,
`:cgb`, `:cbs`, `:cbd`, `:region`).  `implant_range` returns `{ rp:, straggle: }`.

**Variadic functions** — `deal_grove_oxidation` (2 or 4 args) and `diffuse`
(2 or 3 args) accept optional parameters via `argc = -1`.

**Platform linking** — `build.rs` emits `-undefined dynamic_lookup` on
macOS, links against `libruby` on Windows; nothing needed on Linux.

### Dependencies

- `device-physics 0.1.0`
- `mosfet-models 0.1.0`
- `fab-process-simulation 0.1.0`
- `ruby-bridge 0.1.0`
