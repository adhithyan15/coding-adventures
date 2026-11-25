# Changelog

## 0.1.0

- Add paired baseline/candidate collection with injected clocks and checksum validation.
- Add exact ratio analysis, validated JSON replay, and the indexed-array driver.
- Add shared-fixture, independent-oracle, generated-property, and seeded fuzz coverage.
- Package a local shared-fixture copy and reject unbalanced replayed pair orders.
- Embed the selected Rust compiler version at build time so installed benchmark
  binaries do not need `rustc` on `PATH` at runtime.
- Accept any nonblank workload producer identifier during single-report replay while keeping the array CLI workload unchanged.
