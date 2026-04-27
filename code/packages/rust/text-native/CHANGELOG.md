# Changelog

## [0.1.0] — initial release

### Added
- `NativeResolver`, `NativeMetrics`, `NativeShaper`, `NativeHandle` type aliases dispatched at compile time via `cfg(target_vendor)`.
- On Apple targets: aliases resolve to the `CoreTextResolver` / `CoreTextMetrics` / `CoreTextShaper` / `CoreTextHandle` types from `text-native-coretext` (TXT03a).
- On non-Apple targets: aliases resolve to `UnimplementedNativeBackend`, a stub that implements `FontResolver` and returns `FontResolutionError::LoadFailed` with a message identifying the missing backend. Cross-platform binaries can still compile and either detect the gap at runtime or select the device-independent text path at build time.
- Re-exports the `text_interfaces` crate so downstream callers can pull in the trait vocabulary from a single import.
- Tests: version constant; on Apple, compile-time assertion that `NativeResolver` is `CoreTextResolver`; on non-Apple, runtime assertion that `resolve()` returns `LoadFailed`.

### Design
- Per-target `dependencies` block in `Cargo.toml` — `text-native-coretext` is only pulled in on Apple targets, so non-Apple builds remain lean.
- Trait surface is identical regardless of backend — the TXT00 contract is stable across OSes.
