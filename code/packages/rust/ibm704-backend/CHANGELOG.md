# Changelog — ibm704-backend

## v0.2.0 — 2026-08-27 — executable IBM 704 output

* Emits canonical big-endian IBM 704 transport.
* Places constants in an addressable literal pool instead of treating CLA's
  address field as an immediate.
* Rejects instruction plus literal output beyond the 32K-word address space.
* Bounds caller-controlled CIR before allocation and supports absolute
  per-function relocation for safe multi-function module emission.
* Canonical `42` is now `CLA 2; HTR 0; +42` (15 bytes).

## v0.1.0 — 2026-06-11 — initial release (L4)

Phase L4 of the McCarthy Lisp implementation — the historical-arch
backend for the silicon Lisp was born on.

Minimal-viable Backend trait impl over CIR.  Covers `const_*` +
`ret_*` + `ret_void` — enough to keep the lang-aot IBM 704 e2e
smoke test passing byte-for-byte:

* Twig `42` → `[CLA 42; HTR 0]`
  = `[0xA_0000_002A, 0x8_8000_0000]` as 36-bit words
  = `[0x2A, 0x00, 0x00, 0x00, 0x0A, 0x00, 0x00, 0x00, 0x80, 0x08]`
  as 5-byte-per-word little-endian packing on disk (10 bytes).
* McCarthy `42` → same 10-byte sequence (Twig and McCarthy
  Lisp lower a bare-integer program to the same CIR).
* No CONS support (v0.1.0 scope decision for every
  historical-arch backend).

11 byte-pinned unit tests.

Per the GUIDING CONSTRAINT of the historical-arch migration,
`Backend::run` panics with `"ibm704 backend is emit-only…"` — a
future increment can wire it to a yet-to-exist `ibm704-simulator`.
