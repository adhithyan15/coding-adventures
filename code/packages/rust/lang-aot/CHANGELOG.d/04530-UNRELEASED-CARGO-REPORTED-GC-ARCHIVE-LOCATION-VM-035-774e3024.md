## Unreleased — Cargo-reported GC archive location (VM-035)

Resolve the GC static archive from Cargo's compiler-artifact messages so custom
`CARGO_TARGET_DIR` and Cargo-configured layouts work. Reject missing, ambiguous
or nonexistent archives; never substitute a stale default artifact or DLL
import library. Normal BUILD runs parser checks and a real isolated Cargo build
under a temporary directory containing spaces.

