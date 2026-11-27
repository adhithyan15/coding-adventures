# Changelog

## 0.1.0

- Reject more than 10,000 pairs in public analysis before ratio allocation.
- Add paired baseline/candidate collection with injected clocks and checksum validation.
- Add exact ratio analysis, validated JSON replay, and the indexed-array driver.
- Add shared-fixture, independent-oracle, generated-property, and seeded fuzz coverage.
- Package a local shared-fixture copy and reject unbalanced replayed pair orders.
- Report `"unknown"` compiler metadata unless a caller supplies a bounded
  `--rustc-version` label; remove the compiler-querying Cargo build script so
  library consumers do not trigger that extra subprocess.
- Accept any nonblank workload producer identifier during single-report replay while keeping the array CLI workload unchanged.
