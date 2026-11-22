## Unreleased - CV2-F1, generator half: no dump-only exception, the 4,096-byte cap at emission

No version bump and no emitted byte changes: the committed ct-compare v2 `--check`, the
accept-blind, synth-v2, omit-example and tuple-forms fixtures and the fixed-forms parity all pass
unchanged. Mirrors the oracle half (#19616).

### Changed

- **`skip_pure_guards` is removed.** `lower_document_with_fixed` is now `lower_partial` plus the
  `ir.fixed` allowance, with every rule applied. The oracle's reshaped `fixed-forms-v1` (the eight
  `c_not_*` claims under a pure guard) reproduces byte for byte with the normal guard rules.
- **The 4,096-byte cap is enforced at emission** (VER04r 2.3, 14). A non-omitted example whose argument
  or pure value holds a `Bytes` over 4,096 bytes (itself, under `ok`, or in a tuple) is refused with
  code `limit`, after the D18/D19 checks. The omit decision is unchanged: with an `omit_example` line
  the example is still `value-over-limit`.
- **A size error in a callee's `admit` refuses.** `Program::admits_call` clears the `size` flag, so the
  D19 check's size error is never `value-over-limit`, even with an `omit_example` line.
- `tests/census.rs` `ORACLE_PINS` is the oracle's census with its lowering refusal (wasm-leb128 now 16
  `ok` + 2 `W0701` + 24 `ir.fixed`) and gains hash-functions. VER04r R13 (d) lists what remains.

### Added

- Tests in `tests/omit_example.rs` for the cap (argument, pure value, under `ok`, the 4,096/4,097
  boundary, omit unchanged) and for the admit size error; the existing "line removed" cases now expect
  the refusal.
