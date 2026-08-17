# arm1-backend

ARM1 (ARMv1) backend for `jit-core` / `aot-core`.  Second lane of the
9-architecture expansion following the pattern documented in
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).

**v0.1.0 scope**: minimal viable — covers `const_*` (unrotated 8-bit
imm into `R0`) + `ret_*` (pseudo-halt `SWI #0x123456`). Enough to
compile the canonical Twig `42` program to `lang-aot --emit=arm1`
byte-for-byte, verified against the in-tree `arm1-simulator`.

ARM1/ARMv1 (1985) predates the `BX`/link-register-return convention
`armv7-backend` (its architectural descendant) uses, so `ret_*` lowers
to `arm1-simulator`'s pseudo-halt instruction instead of `BX LR` — see
the crate-level doc comment in `src/lib.rs` for the full rationale.

Full op coverage (add/sub/cmp/branches/calls) is **not** ported in
this PR — future increments can add them.  Per the GUIDING
CONSTRAINT, the architectural fix (IIR → CIR via `Backend` trait) is
delivered as soon as the AOT path is wired, regardless of op-set
parity.

See [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
