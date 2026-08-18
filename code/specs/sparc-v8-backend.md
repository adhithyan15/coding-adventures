# `sparc-v8-backend` spec

> **Status:** v0.1.0 — sixth lane of the 9-architecture expansion,
> 2026-08-17.

## Purpose

SPARC V8 (1987) implementation of the `jit_core::backend::Backend`
trait.  Mirror of `mips-r2000-backend` / `arm1-backend` / `armv7-backend`
/ `intel8008-backend` (the *minimal viable* shape).  SPARC V8 was the
first **open** RISC instruction-set standard — designed by Sun
Microsystems (1987) and later powering Sun SPARCstation workstations
and Solaris servers for two decades.

Lowers `Vec<CIRInstr>` (typed, monomorphised) to a big-endian `Vec<u8>`
of SPARC V8 machine code via `sparc-v8-encoder`.

## Why this crate exists

This is the sixth lane of a 9-architecture expansion that replicates
the pattern established by the historical-arch backend migration (see
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](HISTORICAL-ARCH-BACKEND-MIGRATION.md)):
consume typed **CIR** (not dynamically-typed IIR) via the shared
`Backend` trait, so `lang-aot --emit=sparc-v8` routes through the same
`aot_core::infer` + `aot_core::specialise` + `Backend::compile`
pipeline every other arch backend (including `aarch64-backend` /
`x86_64-backend`) uses.  SPARC V8 never had an `iir-to-sparc-v8`
predecessor to migrate away from — this crate starts at the correct
layer from day one, same as `mips-r2000-backend`/`arm1-backend`.

Unlike `arm1-backend` (whose behavioral simulator already existed
in-tree), this lane needed a brand-new `sparc-v8-simulator` — the
Python reference existed (`code/packages/python/sparc-v8-simulator`,
Layer 07r, ~1000 lines) but had no Rust port.  See that crate's own
spec/README for the simulator's full instruction coverage.

## Current scope — minimal viable

| CIR op family | Lowering |
|----------------|----------|
| `const_*` (13-bit signed literal, `[-4096, 4095]`) | `ADD %g0, imm, %o0` |
| `ret_*` | `ta 0` (only if returning the most recently `const_*`'d variable) |
| `ret_void` | `ta 0` |
| Empty CIR body | `ta 0` |
| Anything else | `UnsupportedOp` from `compile()`; `None` from the `Backend::compile` trait method |

There is **no real register allocator** — a trivial "last const var"
scheme tracks which single variable the most recent `const_*` wrote
into `%o0`; `ret_*` only succeeds if it returns exactly that variable.
Programs needing more than one live value fall through to
`UnsupportedOp`.  AOT treats `None` as a per-function compile failure;
JIT keeps execution on the interpreter tier.

Full op coverage (arithmetic, comparisons, branches, calls) that a
mature backend would carry is **intentionally not ported** in this PR
— future increments can extend `compile_single_function` using
`SETHI` for wider constants, condition-code-setting ALU ops (`*cc`
mnemonics) plus `Bicc` for comparisons/branches, and `SAVE`/`RESTORE`
for real function calls with proper register-window rotation (the
underlying `sparc-v8-simulator` already implements all of this — only
this backend's CIR-to-word lowering needs to grow).

## Register-window scoping decision

SPARC V8's defining structural feature is **overlapping register
windows**: 32 logical registers (`%g0`-`%g7` globals, `%o0`-`%o7`
outs, `%l0`-`%l7` locals, `%i0`-`%i7` ins) mapped from a larger
physical register file via a Current Window Pointer (CWP).  `SAVE`
(procedure entry) rotates CWP backward; `RESTORE` (procedure exit)
rotates it forward.

This backend's v0.1.0 CIR lowering **never emits `SAVE`/`RESTORE`**
and only ever touches `%g0` (hardwired zero, CWP-independent by
definition) and `%o0` (windowed, but always physical index 8 as long
as CWP stays at its power-on value of 0 — which it does, for the
entire lifetime of a program that never executes `SAVE`/`RESTORE`).
This sidesteps the register-window complexity entirely for the
trivial `const`/`ret` case, per the task's explicit scoping guidance.

**Crucially, this is a scoping decision in `sparc-v8-backend` only —
not a gap in `sparc-v8-simulator`.**  The simulator crate one layer
down implements the full windowed-register-file machinery completely
and correctly (`SAVE`/`RESTORE`, `virt_to_phys`, overflow detection),
cross-checked bit-for-bit against the in-tree gate-level SPARC V8 port
(`sparc-v8-gatelevel::register_file::virt_to_phys`).  A future
increment adding real function calls to this backend would emit
`SAVE` at function entry / `RESTORE` at `ret_*` and allocate from the
windowed register pool — the simulator underneath is already ready
for that.

## Why `%o0`, not a `%g` register, for the return value

`%o0` is the real SPARC ABI's integer return-value register — see
`sparc-v8-encoder`'s spec (and its crate-level doc comment) for the
full citation and derivation.  Choosing `%o0` over a `%g` register
keeps the lowering architecturally authentic without reintroducing any
window-rotation risk, since CWP never moves in this backend's v0.1.0
programs.

## Why `ret_*` lowers to `ta 0`, not `RESTORE` + `JMPL`

A real SPARC subroutine returns via `RESTORE` (undo the register
window) then `JMPL %i7+8, %g0` (return to the caller) — both require a
live caller context (`%i7` set by a preceding `CALL`) the
minimal-viable `const_*`/`ret_*` scope never establishes: the trivial
ROM just needs to compute a value and stop, with no caller in the
picture.

`sparc-v8-simulator` already defines exactly the right primitive for
this: `ta 0` (trap always, software trap #0), which its executor
intercepts to set `halted() == true` and stop the
fetch-decode-execute loop, leaving the computed value in `%o0` for the
caller to read via `regs.read(8)`.  This is a simulator-level halt
convention matching the existing Python reference's `HALT_WORD`
(`code/packages/python/sparc-v8-simulator/src/sparc_v8_simulator/state.py`)
— not invented for this backend — and plays the same role
`arm1-backend`'s pseudo-halt `SWI #0x123456` plays for ARM1 (which
also predates a clean caller-less return convention).

## Wire format

Each instruction is a 32-bit SPARC V8 word, flattened to
**big-endian** bytes — SPARC's byte order (matching MIPS R2000, and
unlike every other little-endian target in this lane so far).
Per-function byte streams can be concatenated directly; `lang-aot`
writes them straight to disk as a flat `.bin`.

## Pinned byte sequence

| Program | CIR | Emitted bytes |
|---------|-----|----------------|
| Twig `42` | `const_i64 v=42; ret_i64 v` | `[0x90, 0x00, 0x20, 0x2A, 0x91, 0xD0, 0x20, 0x00]` |
| `ret_void` only | `ret_void` | `[0x91, 0xD0, 0x20, 0x00]` |
| Empty CIR | (none) | `[0x91, 0xD0, 0x20, 0x00]` |

`ADD %g0, 42, %o0` = `0x9000_202A`; `ta 0` = `0x91D0_2000`.

## Backend trait surface

| Trait method | Behaviour |
|---------------|-----------|
| `name()` | returns `"sparc-v8"` |
| `compile(ir)` | returns `Some(bytes)` for supported CIR ops; `None` otherwise |
| `compile_function(ctx, ir)` | ignores `FunctionContext` (no parameter marshalling in v0.1.0); delegates to `compile` |
| `run(binary, args)` | **panics** with `"sparc-v8 backend is emit-only; load bytes into sparc-v8-simulator to execute"` — emit-only per the migration spec |

## Error variants

| `BackendError` variant | Trigger |
|--------------------------|---------|
| `UnsupportedOp(String)` | CIR operation outside `const_*`/`ret_*` |
| `InvalidOperand(String)` | Malformed CIR operands or missing `dest` |
| `UndefinedVariable(String)` | Reserved for a future register allocator (unused in v0.1.0's single-var scheme, where the "not the current `%o0` var" case surfaces as `UnsupportedOp` instead) |
| `ImmediateOutOfRange(i64)` | A `const_*` literal falls outside `[-4096, 4095]` — `ADD rd, rs1, simm13`'s sign-extended 13-bit immediate field; wider values need a `SETHI`+`ADD`/`OR` pair, out of scope for v0.1.0 |

## Tests

14 unit/integration tests in `tests/test_backend.rs` (mirroring
`mips-r2000-backend`'s/`arm1-backend`'s test shape) pin the canonical
byte sequence and edge cases (zero, 13-bit range boundaries, bool,
multi-var fallthrough, unsupported op, empty CIR, `ret_void`,
`Backend::run` panics, `Backend::compile` vs the free `compile`
function agree).

One test additionally loads the compiled bytes into
`sparc-v8-simulator`, runs it, and asserts `%o0 == 42` and
`halted() == true` after execution — byte-for-byte parity is
necessary but not sufficient; the emitted bytes must actually execute
correctly (and actually halt) in the existing simulator.

## Backlog

1. [ ] Real register allocator over SPARC V8's other `%o`/`%l`
   registers, removing the single-var limitation.
2. [ ] `SETHI`+`ADD`/`OR` `const_*` lowering (widens the
   `[-4096, 4095]` literal range to the full 32-bit space).
3. [ ] Arithmetic/bitwise CIR ops (`add`/`sub`/`and`/`or`/`xor`) via
   the already-implemented (but not yet re-exported) `encode_alu_reg`/
   `encode_alu_imm` in `sparc_v8_simulator::encoding`.
4. [ ] Comparisons and conditional branches, using the `*cc`-suffixed
   ALU ops to set PSR condition codes and `Bicc` to branch on them.
5. [ ] Direct calls (`CALL`/`SAVE`/`RESTORE`/`JMPL` pairing) and a
   proper stack frame — once this lands, `ret_*` could switch from
   `ta 0` to the historically authentic `RESTORE`+`JMPL` return idiom
   for called functions (the `ta 0` halt would remain for the
   outermost program-exit case).  `sparc-v8-simulator`'s
   register-window machinery is already ready for this.
6. [ ] `Backend::run` wired to `sparc-v8-simulator` for JIT execution
   (best-effort per the migration spec — "no working JIT" is an
   acceptable outcome for a historical-arch target).
