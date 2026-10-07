# Changelog

## 1.0.0 — 2026-10-07

- Pin normalization values below and at `1e-12`, with exact origin comparison
  so a returned sub-threshold input cannot pass by output tolerance alone.
- Pin degenerate SVG arc line evaluation and ordered endpoint bounds for
  zero, negative near-zero, near-coincident, and coincident inputs.
