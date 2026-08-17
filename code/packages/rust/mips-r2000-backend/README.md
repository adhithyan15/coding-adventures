# mips-r2000-backend

MIPS R2000 backend for `jit-core` / `aot-core`.  First lane of the
9-architecture expansion following the pattern documented in
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).

**v0.1.0 scope**: minimal viable — covers `const_*` (16-bit signed imm
into `$v0`) + `ret_*` (`JR $ra`).  Enough to compile the canonical Twig
`42` program to `lang-aot --emit=mips-r2000` byte-for-byte, verified
against the in-tree `mips-r2000-simulator`.

Full op coverage (add/sub/cmp/branches/calls) is **not** ported in this
PR — future increments can add them.  Per the GUIDING CONSTRAINT, the
architectural fix (IIR → CIR via `Backend` trait) is delivered as soon as
the AOT path is wired, regardless of op-set parity.

See [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
