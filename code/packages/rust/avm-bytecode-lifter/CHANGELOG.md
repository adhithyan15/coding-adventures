# Changelog

All notable changes to `avm-bytecode-lifter` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-16

### Added
- Initial implementation of `SWF04: avm-bytecode-lifter`.
- Binary AVM1 Action Lifter (`Avm1Lifter`) decoding arithmetic, stack, member, function, branch, and timeline actions.
- Mandatory CFG validation verifying branch targets land strictly on instruction boundaries (`InvalidBranchTarget`).
- Mandatory abstract stack simulation rejecting stack underflow (`StackUnderflow`) and stack overflow (`StackOverflow`).
- Binary AVM2 ABC Lifter (`Avm2Lifter`) decoding constant pools, method signatures, scripts, method bodies, and ABC opcodes with CFG/stack verification.
- CWE-1321 Prototype Pollution defense with isolated prototype registries and frozen native built-in prototypes.
- Deterministic VM execution helper (`AvmRunner`) executing lifted `IIRModule`s on `vm-core::VMCore` under instruction budget (`max_steps <= 2_000_000`).
- Conformance test suite (`tests/conformance.rs`), counting allocator microbenchmarks (`tests/microbenchmarks.rs`), and runtime capability conformance gate (`tests/runtime_capability.rs`).
- Formal verification artifacts: `avm-bytecode-lifter.warrant`, `required_capabilities.json`, `assurance-profile.json`, `effect_certificate.json`, and 26-CWE `assurance_report.json`.
