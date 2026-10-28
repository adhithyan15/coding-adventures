### Added bounded Cortex-M23 gate simulator

- Added an educational Armv8-M Baseline Cortex-M23 gate companion with
  524,898 authoritative stable-Q D flip-flops, independent gate-derived
  decode/data/flag/control paths, and a fixed Security-disabled profile.
- Added complete-state functional differentials, typed transactional lifecycle
  checks, exact stable-Q mutation locks, and compile-valid hostile structural
  fixtures that reject direct-Q overwrites after legitimate DFF calls.
- Added explicit out-of-range one-hot decoder qualification and measured
  93.64% package-source line coverage (1,827/1,951) across 35 release tests.
