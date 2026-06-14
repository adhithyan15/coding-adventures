# Changelog — iir-codegen-adapters

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-05-11

### Added

- `IIRBackendArtifact` enum wrapping `BEAMModule`, `WasmModule`, `JvmClassFile`,
  and `CILProgramArtifact` — the four IIR backend output types.
- Typed accessor methods: `as_beam()`, `as_wasm()`, `as_jvm()`, `as_clr()`.
- `IIRBackendArtifact::backend_name()` — returns the backend identifier string.
- `IIRBackendArtifact::Display` — shows backend name and a size metric.
- `IIRAdapterError` enum with `UnknownBackend`, `ValidationFailed`, and
  `LoweringFailed` variants; implements `std::error::Error` + `Display`.
- `build_iir_codegen_registry()` — populates a `CodeGeneratorRegistry` with all
  four IIR generators (`IIRBeamCodeGenerator`, `IIRWasmCodeGenerator`,
  `IIRJvmCodeGenerator`, `IIRClrCodeGenerator`).
- `compile_iir(module, backend)` — single-call dispatch to any registered IIR
  backend by name; validates before generating.
- `list_iir_backends()` — returns the four registered backend name strings.
- Full test suite: 45+ unit tests across four source modules and one integration
  test file; all tests pass.
