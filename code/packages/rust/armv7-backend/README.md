# armv7-backend

ARMv7-A (A32) backend for `jit-core` / `aot-core`.  Mirror of
`ge225-backend` / `intel4004-backend` / `aarch64-backend`.

**v0.1.0 scope** (Phase 5 of the historical-arch migration):
minimal viable — covers `const_*` (8-bit imm into r0) + `ret_*`
(BX LR).  Enough to pass the existing lang-aot ARMv7 e2e smoke
test byte-for-byte.

Full op coverage that the deprecated `iir-to-armv7` v0.4.6 had
(add/sub/and/or/xor/adc/sbb/cmp/branches/calls) is **not** ported
in this PR — future increments can add them.  Per the GUIDING
CONSTRAINT, the architectural fix (IIR → CIR via Backend trait)
is delivered as soon as the AOT path is wired, regardless of
op-set parity.

See [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
