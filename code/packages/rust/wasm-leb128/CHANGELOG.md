# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased] — Warrant-derived evidence (LEB01, VER04h)

No change to the public API. One behaviour fix (SEC-006, below): the bounded
decoders reject `max_bits > 64`. (Cargo.toml stays at 0.2.0: this whole entry is
unreleased and the version is bumped at release.)

### Fixed

- `decode_unsigned_bounded` / `decode_signed_bounded` now return `Err` for
  `max_bits > 64`, before any other check. Previously widths 66..=70 silently
  dropped data bits above bit 64 (`FF x9 7F` decoded to `u64::MAX`) and widths
  >= 71 with an eleven-byte input shifted a `u64` by 70 (debug panic, wrong
  value in release: `80 x10 01` gave `(64, 11)`). `max_bits = 0` unchanged.
  Unit tests added first; Warrant document gains the `rejects max_bits > 64u32`
  row, 8 examples and 6 claims (evidence 219 -> 245 entries).

### Added

- `code/specs/LEB01-wasm-leb128.md` (the package had no spec) and
  `code/specs/wasm-leb128.warrant`, the first Warrant v1 document of a real
  package: canonical-encoder models, round trips, every decoder input of length
  <= 3 (16,843,009 per 64-bit decoder), the ten-byte boundary, the bounded
  decoders at every width 0..=64, and the bytes-consumed bound under hostile
  input.
- `evidence.bindings`: the first package binding of host composites
  (`args=bytes,usize[,u32] ret=(u64|i64,usize)
  errors=wasm_leb128/Leb128Error=MALFORMED`) and of the WX-A faults
  `off-by-one-consumed`, `drop-consumed`, `ok-as-err`, `missing-error` plus
  `flip-result-bit`/`drop-last-result-byte` on the encoders.
- `tests/warrant_evidence.rs` (shim) and the generated
  `tests/generated/warrant_evidence.rs`: 31 examples, 36 claims each with an
  exhaustive, a property (4,096 cases) and a fuzz (5,000 iterations) suite, and
  80 negative controls; `evidence/test-manifest.json` is written from a real run.
- `evidence/proof-manifest.json` (from `warrant-ir --test wasm_leb128_proofs`):
  every steps-bound obligation `not-live` (the Rust-subset front end refuses
  `src/lib.rs` at the anonymous lifetime `Formatter<'_>`), heap/depth rows
  `unchecked`.
- `effect_certificate.json` (`ca-capability-analyzer --emit-cert`: no effects,
  label `proved`) and `assurance_report.json` (`--profile varint-codec`, verdict
  `report-only`; CWE-208 "not applicable: no secret operands", CWE-400 an
  unapproved exemption quoting the front end's blocker).
- `BUILD`/`BUILD_windows`: unit tests and doc tests run separately from the
  evidence target; the capability check; the drift check; the manifest run; 20
  sabotage runs (each fault against each operation it is bound to); the proof
  target; `ca-assurance-report --check`.
- `Cargo.toml`: dev-dependencies `coding_adventures_test_bootstrap` and
  `coding_adventures_sha256`, and the `warrant_evidence` test target
  (`harness = false`).

- VER08b runtime capability conformance gate (`tests/runtime_capability.rs`,
  `harness = false`, dev-dependency `capability-runtime-conformance`): all six
  public operations, including the `max_bits > 64` rejection, run inside a
  seccomp/Landlock jail derived from `required_capabilities.json`
  (`capabilities: []`), watched by an independent `strace -f`, with the 18
  negative controls and a Docker ring. Linux only (NOT-LIVE elsewhere). `BUILD_linux`
  runs it with `RCG_STRICT=1` (CI installs `strace` on Linux for this); the plain
  `BUILD`, which macOS uses, runs it without, so a macOS NOT-LIVE gate does not
  fail the build; `BUILD_windows` does not run it. `RCG_REQUIRE_DOCKER` is not set.

### Changed

- Two unit tests printed an error with `println!`; they now assert its text
  instead (`decode_unsigned_unterminated` checks for "unterminated",
  `error_display` pins the exact rendering). The capability analyzer reads test
  code, and the `println!` made this pure library need `stdout:write`, which
  blocks the effect certificate and would have opened CWE-117. Test-only; no
  export changes.

## [0.2.0] - 2026-08-31 — LEB128/malformed-binary hardening

Real bug, found while vendoring the official WebAssembly testsuite's
`binary.wast`/`binary-leb128.wast`/`binary_leb128_64.wast` (dense
`assert_malformed` corpora specifically targeting LEB128 encoding rules)
into `wasm-conformance`: `decode_unsigned`/`decode_signed` accumulated
every byte's data bits into a native `u64`/`i64` with no check that the
value actually FIT in 64 bits. A 10-byte encoding (the max this crate
already allowed) whose final byte's padding bits carried a value one bit
past `u64::MAX` (unsigned) or inconsistent with the encoded sign (signed)
had those extra bits silently shifted away by ordinary `u64` arithmetic
instead of being rejected — a genuinely malformed binary (per the WASM
spec's own `uN`/`sN` grammar) decoded successfully to a wrong, wrapped
value.

### Added

- `decode_unsigned_bounded(data, offset, max_bits)` / `decode_signed_bounded(data, offset, max_bits)`
  — the width-aware siblings of `decode_unsigned`/`decode_signed`, needed
  by any caller decoding a field narrower than 64 bits (the overwhelming
  majority of the WASM binary format: every `u32` count/index/length).
  Enforce, for ANY `max_bits` (not hardcoded to 32 or 64):
  - **Overlong**: at most `ceil(max_bits / 7)` bytes — a continuation flag
    on the byte the width's budget says must be the last one is malformed
    ("integer representation too long" in the spec's own wording), even
    though every individual byte is well-formed LEB128 and the byte
    count would be fine for a WIDER field.
  - **Out of range**: on the terminal byte, the bits above `max_bits` must
    be all zero (unsigned) or repeat the value's own sign bit (signed) —
    otherwise the encoding claims a value that doesn't fit in `max_bits`
    at all ("integer too large"), independent of the overlong check.
  - Non-minimal (padded, but in-budget) encodings remain explicitly legal
    — the spec permits them, and this crate always has.
- `decode_bits_core`: the shared private implementation both the new
  bounded functions AND `decode_unsigned`/`decode_signed` (now defined as
  `decode_{unsigned,signed}_bounded(.., 64)`) go through — a `u64`/`i64`
  is itself just a 64-bit-wide `uN`/`sN`, so the exact same padding-bits
  rule that fixes the bounded case ALSO closes the native 64-bit
  overflow bug described above, with one implementation instead of two.
- 15 new unit tests, one per malformed-encoding class this fix closes
  (overlong/out-of-range at both a 32-bit and the native 64-bit width,
  truncated streams through the bounded entry point, the 7-bit-width edge
  case, and confirmation that non-minimal-but-in-budget encodings are
  still accepted).

### Notes

- `decode_unsigned`/`decode_signed`'s public signatures and return
  contracts are unchanged — this is a pure bug fix plus additive API,
  not a breaking change. Every existing round-trip test (`u64::MAX`,
  `i64::MIN`/`MAX`, non-minimal encodings) still passes.

## [0.1.0] - 2026-03-23

### Added

- Initial package scaffolding generated by scaffold-generator

## [0.2.0] - 2026-03-23

### Added

- Full implementation of LEB128 unsigned and signed encoding/decoding
- `decode_unsigned(data, offset)` — decodes an unsigned LEB128 integer,
  returns `(value, bytes_consumed)` or a `Leb128Error`
- `decode_signed(data, offset)` — decodes a signed LEB128 integer with
  sign-extension, returns `(value, bytes_consumed)` or a `Leb128Error`
- `encode_unsigned(value: u64)` — encodes an unsigned integer as LEB128 bytes
- `encode_signed(value: i64)` — encodes a signed integer as LEB128 bytes using
  two's complement with proper termination condition
- `Leb128Error` struct implementing `Display`, `Debug`, `PartialEq`, and
  `std::error::Error`
- 27 unit tests covering all required test cases plus edge cases
- 5 doc-tests embedded in rustdoc comments
- Literate programming style: all functions documented with algorithm
  walkthroughs, visual traces, and ASCII diagrams

### Notes

- The commonly cited example `[0xE5, 0x88, 0x26]` for 624485 is incorrect;
  the correct encoding is `[0xE5, 0x8E, 0x26]` (0x88 decodes to 623717)
