## 0.370.0 - 2026-10-05 - Runtime-real captured globals

- Preserve formatter provenance for ordinary real scalars captured by nested
  procedures through the existing E6 typed-global path. The shared slot remains
  a concrete f64, so this requires no closure or thunk ABI change.
