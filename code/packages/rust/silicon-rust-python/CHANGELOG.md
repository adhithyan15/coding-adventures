# Changelog — silicon-rust-python

## [0.1.0] — 2026-06-13

### Added

- `silicon-rust-python` crate: Python C extension (cdylib) for the Rust
  silicon simulation stack.

- **26 Python functions** across three domains:
  - 9 physical-constant getters (`k_boltzmann`, `q_electron`, `eps0`,
    `eps_si`, `eps_ox`, `n_i_300k`, `eg_si_300k`, `mu_n_300k`, `mu_p_300k`)
  - 8 device-physics functions (`thermal_voltage`, `intrinsic_concentration`,
    `fermi_potential`, `pn_junction_built_in_voltage`,
    `pn_junction_depletion_width`, `pn_junction_saturation_current`,
    `pn_junction_current`, `mosfet_threshold_voltage`)
  - 2 MOSFET Level-1 evaluators (`evaluate_level1`, `evaluate_level1_defaults`)
  - 7 fab-process-simulation functions (`deal_grove_oxidation`, `deposit`,
    `etch`, `implant`, `diffuse`, `implant_range`, `diffusivity_cm2_per_s`)

- **CrossSection wire format** (v0.1): pipe-separated `material:thickness_nm`
  pairs for lossless round-trip of layer stacks.  Doping profiles not yet
  serialised (documented limitation).

- **`build.rs`**: platform-specific linker setup (macOS `-undefined
  dynamic_lookup`, Windows `python3.lib` probe via `sysconfig`) mirroring
  the pattern established by `matrix-rust-python`.

- **15 unit tests** (pure Rust, no Python interpreter required): wire format
  round-trips, device physics spot checks, Level-1 region classification,
  process step correctness, SRIM table lookup.

- `BUILD` / `BUILD_windows`: build-tool integration.
- `README.md`: full API reference, wire format documentation, end-to-end
  CMOS inverter process example.
