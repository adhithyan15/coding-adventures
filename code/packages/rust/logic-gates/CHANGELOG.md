# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased] — Warrant-derived evidence for the stateless slice (LG01, VER04h)

Layer 1 of the Intel 4004 verification chain (VER27 section 4.15, backlog LV-006).
`sequential.rs` is untouched and is the next slice. (Cargo.toml stays at 0.1.0:
this whole entry is unreleased and the version is bumped at release.)

### Fixed

- **SEC-007: a value that is not a bit is rejected in every build profile.**
  `validate_bit` was a `debug_assert!`, so a release build accepted it silently:
  `tri_state(2, 1)` returned `Some(2)`, `priority_encoder` read a 2 as inactive,
  and `encoder` counted active inputs in a `u8` (257 active inputs wrapped to 1
  and passed the one-hot check). It is now `assert!` everywhere, the count is a
  `usize`, and `nand_gate`, `nor_gate` and `xnor_gate` validate their inputs like
  the other gates (they relied on the transistor model alone). The valid domain
  is unchanged. `tests/bit_domain.rs` failed three tests under `cargo test
  --release` before the fix.
- **SEC-008: `decoder` and `demux` have at most `MAX_DECODER_WIDTH` = 16 inputs.**
  The output size `1usize << n` was unbounded: 17..=40 inputs attempted a 128 KiB
  to 1 TiB allocation, and 64 inputs wrapped the shift to 1 in release and
  returned one line. More than 16 now panics before any allocation.
  `tests/decoder_width.rs` (written first).

### Added

- `pub const MAX_DECODER_WIDTH: usize = 16`.
- The stateless operations are re-exported at the crate root
  (`logic_gates::and_gate`), so the Warrant binding can name them. `sequential`
  is not re-exported. No existing path changes.
- `code/specs/LG01-logic-gates-contract.md` and `code/specs/logic-gates.warrant`:
  22 operations, 88 claims, 3,225,774 exhaustive + 544,349 bounded-set (3,770,123 total) cases, 83 examples; the claim
  names are published as stable identifiers for layer-contract linking (LV-005).
- `evidence.bindings`, `tests/warrant_evidence.rs` (shim) and the generated
  `tests/generated/warrant_evidence.rs`: 88 claims each with an exhaustive, a
  property (4,096 cases) and a fuzz (5,000 iterations) suite, and 204 negative
  controls; `evidence/test-manifest.json` is written from a real run.
- `evidence/proof-manifest.json` (from `warrant-ir --test logic_gates_proofs`):
  every steps-bound obligation `not-live` (the Rust-subset front end refuses
  `gates.rs` and `combinational.rs`), depth rows `unchecked`.
- `effect_certificate.json` (`ca-capability-analyzer --emit-cert`: no effects,
  label `proved`) and `assurance_report.json` (`--profile boolean-logic`, verdict
  `report-only`; CWE-208 "not applicable: no secret operands", CWE-400 an
  unapproved exemption quoting the front end's blocker).
- VER08b runtime capability conformance gate (`tests/runtime_capability.rs`,
  `harness = false`): every public stateless operation runs inside a
  seccomp/Landlock jail derived from `required_capabilities.json`
  (`capabilities: []`), watched by an independent `strace -f`, with the 18
  negative controls and a Docker ring. Linux only (NOT-LIVE elsewhere).
  `BUILD_linux` runs it with `RCG_STRICT=1`; the plain `BUILD` (macOS) does not;
  `BUILD_windows` does not run it.
- `tests/bit_domain.rs`, `tests/decoder_width.rs`: the rejection of non-bits,
  bad shapes and over-wide decoders, in both profiles.
- `BUILD`/`BUILD_linux`/`BUILD_windows`: the hand-written tests, the doc tests and
  the evidence target run separately (a bare `cargo test -p logic-gates` would run
  the two-minute generated target alongside); the release-profile runs of the bit
  domain and of the whole generated harness; the capability check; the drift
  check; the manifest run; 51 sabotage runs (each fault against each operation it
  is bound to); the proof target; `ca-assurance-report --check`.
- `Cargo.toml`: dev-dependencies `coding_adventures_test_bootstrap`,
  `coding_adventures_sha256` and `capability-runtime-conformance`, and the
  `warrant_evidence` and `runtime_capability` test targets (`harness = false`).


### Fixed

- Rust 1.100 clippy (`-D warnings`) drift in `src/sequential.rs`: `needless_range_loop`: the counter increment iterates `state.value` directly; behaviour unchanged.

## [0.4.0] - 2026-04-12

### Added

- `xor_n(inputs: &[u8]) -> u8` — N-input XOR gate (parity reducer). Chains
  2-input XOR gates left-to-right via `fold`, returning 1 when an odd number
  of inputs are 1 (odd parity) and 0 for even parity. Used by the Intel 8008
  gate-level simulator to compute the Parity flag: `P = NOT(xor_n(result_bits))`.

## [0.3.0] - 2026-03-29

### Changed

- `xnor_gate` now delegates to `CMOSXnor::new(None).evaluate_digital(a, b)`
  instead of composing `CMOSXor` and `CMOSInverter` inline. Behaviour is
  identical; the call site is simpler and the transistor count (8) is now
  self-documenting through `CMOSXnor::TRANSISTOR_COUNT`.

## [0.2.0] - 2026-03-28

### Changed

- **Transistor-backed gate implementations**: All seven primitive gates
  (`not_gate`, `and_gate`, `or_gate`, `xor_gate`, `nand_gate`, `nor_gate`,
  `xnor_gate`) now delegate their digital evaluation to CMOS gate models from
  the `transistors` crate. Each call instantiates a default-parameter CMOS gate
  and calls `evaluate_digital(...)`, routing the computation through transistor
  physics simulation.
- **New dependency**: `transistors = { path = "../transistors" }` added to
  `Cargo.toml`. Both packages are workspace members so no registry changes are
  needed.
- **XNOR composition**: `xnor_gate` chains `CMOSXor` and `CMOSInverter`:
  `NOT(XOR(a, b))`.

## [0.1.0] - 2026-03-18

### Added
- Seven fundamental logic gates: AND, OR, NOT, XOR, NAND, NOR, XNOR
- NAND-derived gates proving functional completeness: nand_not, nand_and, nand_or, nand_xor
- Multi-input gates: and_n, or_n
- Sequential logic: SR latch, D latch, D flip-flop
- Register (N-bit parallel storage)
- Shift register (serial-to-parallel conversion, left/right)
- Counter (binary counting with overflow)
- Comprehensive doc comments with truth tables, circuit diagrams, and hardware explanations
- Inline unit tests and integration test files
- Ported from Python logic-gates package
