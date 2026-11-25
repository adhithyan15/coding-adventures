## 0.2.0 — LGE-RUST-HARDEN

**Breaking:** `CounterState::new` now panics for widths above `MAX_COUNTER_WIDTH` (64), and `counter` panics on a `reset` other than 0 or 1.

- `mux_n` is now iterative (a tournament of 2:1 MUXes, one level per select bit) instead of recursive, so the report's CWE-674 attestation ("zero recursive function calls; call graph strictly acyclic") is true. Results are identical; the exhaustive Warrant corpus is unchanged.
- `counter` validates `reset` as a bit: 2 or 255 now panics (`reset must be 0 or 1, got N`) instead of being treated as "no reset".
- `CounterState::new` rejects widths above the new `MAX_COUNTER_WIDTH` (64).
- `validate_bits` builds the element name only on failure.
- `#![forbid(unsafe_code)]` at the crate root.
- Adopted the generated `src/capabilities.rs` (pure token), which `--verify-cert` requires; BUILD, BUILD_linux and BUILD_windows now run `--check-module` and `--verify-cert effect_certificate.json` every build.
- Regenerated `effect_certificate.json`, `evidence/*-manifest.json` and `assurance_report.json` with the canonical tools. The memory-class (CWE-416/787/125/415/476/190) reason now says "no FFI in the package's own sources (the analysis closure is the package)" rather than "no FFI reachability".
- `ca-assurance-report`: the memory-safety reason text is corrected at its source; `--check` still accepts the legacy sentence so other packages' reports pass until regenerated.
- Added a differential `mux_n` test against the plain index formula (n = 2, 4, 32, 64) and unit tests pinning the `ca-assurance-report` legacy memory-reason shim.
