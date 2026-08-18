# sparc-v8-backend

SPARC V8 backend for `jit-core` / `aot-core`.  Sixth lane of the
9-architecture expansion following the pattern documented in
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).

**v0.1.0 scope**: minimal viable — covers `const_*` (13-bit signed imm
into `%o0` via `ADD %g0, imm, %o0`) + `ret_*` (`ta 0`, trap-always
HALT). Enough to compile the canonical Twig `42` program to
`lang-aot --emit=sparc-v8` byte-for-byte, verified against the in-tree
`sparc-v8-simulator`.

## Why `%o0`, not a `%g` register, for the return value

`%o0` is the real SPARC ABI's integer return-value register. It is a
*windowed* register, but this backend never emits `SAVE`/`RESTORE`, so
the Current Window Pointer never moves for the lifetime of a compiled
program — `%o0` always resolves to the same fixed physical register.
See `src/lib.rs`'s crate-level doc comment for the full derivation and
the `sparc-v8-encoder` doc comment it cross-references.

## Why `ret_*` lowers to `ta 0`, not `RESTORE` + `JMPL`

A real SPARC subroutine returns via `RESTORE` (undo the register
window) then `JMPL %i7+8, %g0` — both require a live caller context
(`%i7` set by a preceding `CALL`) that the minimal-viable `const_*`/
`ret_*` scope never establishes. `sparc-v8-simulator` already defines
`ta 0` (trap always, software trap #0) as its HALT convention — see
`src/lib.rs` for the full rationale.

Full op coverage (add/sub/cmp/branches/calls) is **not** ported in
this PR — future increments can add them, along with a real register
allocator that emits `SAVE`/`RESTORE` at function boundaries. The
`sparc-v8-simulator` crate underneath already implements the full
windowed-register-file machinery needed for that; only this backend's
CIR-to-word lowering needs to grow.

See [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
