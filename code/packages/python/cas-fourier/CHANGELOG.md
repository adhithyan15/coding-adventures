# Changelog — cas-fourier

## 0.1.0 — 2026-04-27

Initial release.

### Added

- `fourier_transform(f, t, ω)` — forward Fourier transform via table lookup and linearity
- `ifourier_transform(F, ω, t)` — inverse Fourier transform via exact pattern matching
- Forward table entries (physics/engineering convention `F(ω) = ∫ f(t) e^{-iωt} dt`):
  - `δ(t)` → `1`
  - `1` → `2π·δ(ω)`
  - `exp(-a·t)` (causal) → `1/(a + i·ω)`
  - `exp(i·a·t)` → `2π·δ(ω - a)`
  - `sin(ω₀·t)` → `i·π·(δ(ω+ω₀) - δ(ω-ω₀))`
  - `cos(ω₀·t)` → `π·(δ(ω-ω₀) + δ(ω+ω₀))`
  - `exp(-a·t²)` (Gaussian) → `√(π/a)·exp(-ω²/(4a))`
  - `t·exp(-a·t)` (causal ramp) → `1/(a + i·ω)²`
- Scalar linearity: `fourier(c·f) = c·fourier(f)` for `c` independent of `t`
- Sum linearity: `fourier(f + g) = fourier(f) + fourier(g)`
- Inverse table — exact pattern matches against forward table outputs for round-trip consistency
- `FOURIER` and `IFOURIER` IR head symbols in `heads.py`
- `build_fourier_handler_table()` for VM integration
- Full test suite with ≥ 80% coverage (25+ tests)
- README with usage examples and architecture description
