## 0.45.0 — 2026-06-11 — **McCarthy Lisp on the silicon Lisp was born on** — L4 + L5: `--emit=ibm704`

The closing half of the **CAR/CDR birthplace round-trip**.  `CAR` and
`CDR` — the two universal Lisp accessors McCarthy introduced in 1958 —
were *literally* IBM 704 instruction-word field mnemonics
(**C**ontents of the **A**ddress / **D**ecrement part of **R**egister).
The IBM 704 (1954) is the vacuum-tube mainframe McCarthy and his MIT
students Steve Russell, Tim Hart, and Mike Levin first ran Lisp on, in
1959.  This release lets McCarthy Lisp source compile back to that
silicon — the symmetric counterpart of the **Dartmouth BASIC → GE-225**
round-trip the historical-arch migration already established.

### What changed

* New `EmitMode::Ibm704Bin` variant + CLI `--emit=ibm704` (aliases
  `ibm-704`, `704`).
* New API `lang_aot::compile_file_to_ibm704_bin(src, out, language)`
  that routes source through the standard `aot_core::infer` +
  `aot_core::specialise` + `Backend::compile` pipeline against the new
  `ibm704-backend` v0.1.0 crate.
* New error variant `LangAotError::Ibm704BackendError(String)` carrying
  the backend's `BackendError` Display.
* New dev-deps: pulls `ibm704-backend` v0.1.0 + `ibm704-encoder` v0.1.0
  from the workspace.

### Wire format

Each 36-bit IBM 704 word is packed as **5 bytes**, low byte first.
The top 4 bits of the high byte are always zero — the 40-bit byte
window has 4 padding bits, which `ibm704-encoder::pack_word`
zeroes by construction.  Same convention `ge225-encoder` uses
(20-bit words → 3 bytes) extended to 36 bits.

### Pinned byte sequences

* **Twig `42`** → `[CLA 42; HTR 0]` =
  `[0xA_0000_002A, 0x8_8000_0000]` = the canonical 10 bytes
  `[0x2A, 0x00, 0x00, 0x00, 0x0A, 0x00, 0x00, 0x00, 0x80, 0x08]`.
* **McCarthy `42`** → byte-for-byte identical — the IIR
  convergence in action.  One source language, one
  `Language::McCarthyLisp` arm, one shared IIR, one shared
  backend, one machine-code sequence.

### v0.1.0 scope — minimal viable

Per the McCarthy Lisp v0.1.0 scope decision (confirmed in
`MCCARTHY-LISP-PLAN.md`), the IBM 704 backend handles only
`const_*` + `ret_*` + `ret_void`.  CONS-using programs are out
of scope for *every* historical-arch backend in v0.1.0; that
hasn't changed, and the IBM 704 follows the same convention as
GE-225 / Intel 4004 / Intel 8008 / ARMv7 / RV32I.

### Tests

Two new e2e tests in `tests/end_to_end_smoke.rs`:
* `end_to_end_twig_42_emits_ibm704_bin_via_lang_aot` — pins the 10
  bytes byte-for-byte.
* `end_to_end_mccarthy_42_emits_ibm704_bin_via_lang_aot` — pins the
  same 10 bytes, demonstrating the IIR convergence.

### Specs

New: `code/specs/ibm704-encoder.md`, `code/specs/ibm704-backend.md`.
`code/specs/MCCARTHY-LISP-PLAN.md` updated to mark L4 + L5 as ✓.

