# Changelog — maxima-iir-compiler

## v0.1.0 — 2026-08-18 — initial release (macsyma-iir-vm.md, Wave 5 item 1)

The second frontend onto `interpreter_ir` (IIR) in this rollout, and the
first Wave 5 item — a direct re-export of `macsyma-iir-compiler`'s public
API (`compile`, `compile_source`, `MacsymaIirError`) under Maxima's own
name, mirroring `maxima-to-semantic-ir`'s identical relationship to
`macsyma-to-semantic-ir` on the Semantic-IR side.

* No shim, no Maxima-specific CST, no `maxima-vm` — every compiled
  `IIRModule` runs unchanged on `macsyma-vm`.
* 5 tests: re-export symmetry (`compile`/`compile_source`), a
  representative accepted program running end-to-end through
  `macsyma-vm`, the `;`/`$` display/suppress terminator pair, one
  rejected-construct check, and a malformed-input error-not-panic check.
