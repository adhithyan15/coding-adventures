# Changelog — gpu-core (Perl)

## [Unreleased]

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries) instead of the previous skip or bare-`perl` recipe.

## 0.01 — 2026-03-31

Initial release.

- `FPRegisterFile` — configurable floating-point register file
- `LocalMemory` — scratchpad memory with bounds-checked load/store
- `GenericISA` — full teaching instruction set
- `GPUCore` — fetch-execute loop with cycle-accurate tracing
- Example programs: `saxpy_program`, `dot_product_program`
