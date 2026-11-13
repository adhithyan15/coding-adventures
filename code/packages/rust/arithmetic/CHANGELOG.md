# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased] — Warrant-derived evidence (AR01, VER04h)

Layer 2 of the Intel 4004 verification chain (VER27 section 4.15, backlog LV-006,
LP-011), on top of `logic-gates` (layer 1). (Cargo.toml stays at 0.1.0: this whole
entry is unreleased and the version is bumped at release.)

### Added

- **`code/specs/arithmetic.warrant` and `AR01-arithmetic-contract.md`**: the
  executable specification (Warrant v1) and its prose contract. 5 operations, 53
  examples, 63 claims, 2,655,437 cases run in the generator's "exhaustive" mode (859,848 exhaustive + 1,279,161 sampled + 516,428 bounded-set; see AR01 section on the case count): every input of the half and full
  adder; both ripple-carry adders for every input at 1, 2, 3, 4 and 8 bits (the 4-bit
  adder with a carry-in is the 512 cases of VER27 4.15, with carry out and signed
  overflow); the six ALU operations with all four flags at 4 and 8 bits (Add and Sub
  also at 1 to 3); 16 bits **sampled** (a 377-value boundary domain, squared) and
  named `_sampled`; rejection of a non-bit at each of eight positions, of unequal
  lengths and of an empty operand. Oracles are Warrant's own integers, never the
  crate (a subtraction is `a - b` with carry = no borrow, not `a + NOT(b) + 1`).
- **`evidence.bindings`, `tests/generated/warrant_evidence.rs`,
  `tests/warrant_evidence.rs`**: the generated harness (examples, exhaustive,
  property and fuzz suites, 52 negative controls), `evidence/test-manifest.json`
  and `evidence/proof-manifest.json` (the proof front end refuses `adders.rs` and
  `alu.rs`, so every steps-bound obligation is `not-live`; backlog AR-001),
  `effect_certificate.json` and `assurance_report.json` (`--profile
  bit-arithmetic`, report-only).  The report records `"profile": "bit-arithmetic"`
  (required since the lzw merge) and BUILD, BUILD_linux and BUILD_windows pass
  `--profile bit-arithmetic` to `--check`.
- **`tests/bit_domain.rs`**: every operation rejects a non-bit at every position of
  every operand and in the carry-in, unequal lengths and an empty operand, in the
  debug and the release profile (BUILD runs it in both): the rejection is inherited
  from the `logic-gates` guard (its SEC-007), so a regression there must fail this
  crate's build. Also pins that `AluOp::Not` ignores `b`, and that a 100,000-bit
  addition allocates its own size and no more.
- **`tests/runtime_capability.rs` (VER08b)**: every public operation runs inside a
  seccomp/Landlock jail derived from `required_capabilities.json` (no capabilities),
  watched by an independent `strace`: enforcement pass, observation pass, 18
  negative controls caught. `BUILD_linux` runs it with `RCG_STRICT=1`.
- **`BUILD`, `BUILD_linux`, `BUILD_windows`** replace the one-line Starlark /
  `cargo test` BUILD: the hand-written tests, the release-profile bit-domain and
  evidence runs, the capability analyzer, the drift check of the generated file, 13
  sabotage lines that must print `not ok`, the proof manifest and the report check.
- `lib.rs` re-exports `half_adder`, `full_adder`, `ripple_carry_adder`,
  `ripple_carry_adder_with_carry`, `RippleCarryResult`, `alu`, `AluOp` and
  `AluResult` at the crate root, where the Warrant binding resolves an `export=`
  (the same items; no call site changes).

### Changed

- Documentation only in `adders.rs` and `alu.rs`: a `# Panics` section on every
  public operation (a non-bit, unequal lengths, an empty operand, in every build
  profile; `Not` never reads `b`), and the meaning of `AluResult.carry` after a `Sub`
  (no borrow: `a >= b`). No behaviour changed.

### Findings

- The domain checks (VER27 section 7.4) found **no defect in this crate**: every
  non-bit is rejected in both profiles because `logic-gates` already rejects it
  (SEC-007 there), no operation sizes anything from a caller's width, and the
  exhaustive claims found no disagreement with the integer definitions. No SEC
  row; one lesson (a rejection inherited from a lower guard is pinned in the upper
  crate's release build). Recorded as properties: the rejection is
  inherited (pinned in release here), and `Not` ignores `b` by design.

## [0.1.0] - 2026-03-18

### Added
- Half adder (XOR + AND)
- Full adder (two half adders + OR)
- Ripple-carry adder for N-bit addition with overflow detection
- Ripple-carry adder with explicit carry-in (for subtraction support)
- ALU with six operations: ADD, SUB, AND, OR, XOR, NOT
- ALU status flags: zero, carry, negative, overflow
- Two's complement subtraction (A + NOT(B) + 1)
- Signed overflow detection
- Comprehensive doc comments with circuit diagrams and truth tables
- Inline unit tests and integration test files
- Ported from Python arithmetic package
