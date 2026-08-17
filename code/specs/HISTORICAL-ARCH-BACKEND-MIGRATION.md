# Historical-arch backend migration — `iir-to-*` → `*-encoder` + `*-backend`

**Status:** ✅ **MIGRATION COMPLETE** (Phases 1–7 done, 2026-06-03).  All five historical-arch lanes (GE-225, Intel 4004, ARMv7, Intel 8008, RV32I) now consume typed CIR via the `Backend` trait.  The architectural correctness win — every arch backend uses the same `aot_core::infer` + `aot_core::specialise` + `Backend::compile` pipeline as `aarch64-backend` and `x86_64-backend` — is delivered.
**Plan:** [`MULTILANG-ARCHITECTURE-BACKENDS.md`](MULTILANG-ARCHITECTURE-BACKENDS.md) (which produced the A1–A5 lanes this migration corrects).

## The architectural mistake the A1–A5 cascades made

The A1–A5 architecture-backend lane (`iir-to-riscv`, `iir-to-intel8008`,
`iir-to-armv7`, `iir-to-intel4004`, `iir-to-ge225`) shipped 5 crates
that all sit at the **wrong layer** in the compiler stack.

They consume **IIR** (interpreter-IR — dynamically typed, with ops
like `add a b` whose argument types are unknown until inference) and
emit machine code directly.  This sounds plausible but skips two
architectural amenities that the existing `aarch64-backend` and
`x86_64-backend` crates already provide:

1. **Type monomorphization.**  The proper input to a real-arch
   backend is **CIR** (compiler-IR), which is the typed,
   specialised form of IIR.  CIR ops carry their type suffix:
   `add_i64`, `cmp_lt_u32`, `neg_i16`.  No backend should have to
   redo type inference.
2. **The `jit_core::backend::Backend` trait.**  A single trait
   contract — `fn compile(&self, ir: &[CIRInstr]) -> Option<Vec<u8>>`
   — plugs a backend into **both** `aot-core` (for AOT executables)
   **and** `jit-core` (for in-process execution).  My `iir-to-*`
   crates were invisible to JIT and needed hand-rolled wiring into
   `lang-aot`.

## The correct pattern (already in use for x86_64 and AArch64)

```text
IIR (interpreter-IR, dynamic-typed: "add a b")
  │
  ▼  aot_core::infer::infer_types
  ▼  aot_core::specialise::aot_specialise
  │
CIR (compiler-IR, monomorphised: "add_i64 a b")
  │
  ▼  Backend::compile(&[CIRInstr]) → Option<Vec<u8>>
  │
  ├──→ aot_core::link → AOT executable bytes      (twig-aot, lang-aot)
  └──→ jit_core::GenericCirJit → JIT execution    (BasicCirJit, OctCirJit, …)
```

Two crates per arch:

- **`{arch}-encoder`** — pure encoding tables and `encode_*`
  helpers.  No IR knowledge.  Mirror of `aarch64-encoder` /
  `x86_64-encoder`.
- **`{arch}-backend`** — implements `Backend`.  Lowers CIR
  to bytes using the encoder.  Mirror of `aarch64-backend` /
  `x86_64-backend`.

The old `iir-to-{arch}` crate retires (becomes a `#[deprecated]`
shim that forwards to the new backend, then eventually disappears).

## Phase plan

GE-225 establishes the pattern (3 careful phases); the other 4
arches are mechanical applications (1 phase each).

| Phase | Scope | Output |
|-------|-------|--------|
| ✅ **1** | `ge225-encoder` carve-out | **MERGED** (PR #4954).  New crate with constants + `encode_*`.  `iir-to-ge225` re-exports from it. |
| ✅ **2** | `ge225-backend` skeleton + ops | **MERGED** (PR #4956).  Implements `Backend`.  Same op set as `iir-to-ge225` v0.9.0, via CIR. |
| ✅ **3** | `ge225-backend` wiring + `iir-to-ge225` deprecation | **this PR** — `lang-aot --emit=ge225` routes through `aot_core::infer` + `aot_core::specialise` + `ge225_backend::compile`.  `iir-to-ge225` marked `#[deprecated]` at the API level; existing callers keep working with warnings.  All 5 GE-225 + 3 BASIC e2e tests produce byte-for-byte-identical output. |
| ✅ **4** | Intel 4004 migration | **this PR** — `intel4004-encoder` + `intel4004-backend` + lang-aot wiring + `iir-to-intel4004` marked `#[deprecated]`.  Byte-for-byte parity verified by the existing Intel 4004 e2e smoke test. |
| ✅ **5** | ARMv7 migration | **this PR** — `armv7-encoder` + `armv7-backend` (minimal viable: `const_*` + `ret_*` only) + lang-aot wiring + `iir-to-armv7` marked `#[deprecated]`.  Byte-for-byte parity for the trivial `MOV r0, #N; BX LR` ROM verified by the e2e smoke test.  Full op coverage (add/sub/cmp/branches/calls) is intentionally NOT ported — future increments can add to `armv7-backend` as needed. |
| ✅ **6** | Intel 8008 migration | **this PR** — `intel8008-encoder` + `intel8008-backend` (minimal viable: `const_*` + `ret_*`) + lang-aot wiring + `iir-to-intel8008` deprecated.  Byte-for-byte parity for `MVI A, 42; HLT` ROM verified. |
| ✅ **7** | RV32I migration (FINAL lane) | **this PR** — `riscv-encoder` + `riscv-backend` (minimal viable: `const_*` + `ret_*`) + lang-aot wiring + `iir-to-riscv` deprecated.  Byte-for-byte parity for the canonical `addi t0, x0, n; addi a0, t0, 0; jalr x0, x1, 0` sequence verified by unit tests, e2e RV32I smoke test still ends with the canonical `[0x67, 0x80, 0x00, 0x00]` (`jalr x0, x1, 0`) tail.  Full op coverage (add/sub/cmp/branches/calls/ecall print_i64) intentionally NOT ported — future increments can add to `riscv-backend` as needed.  This was the **original mistake from A1+** that started the IIR-level pattern this entire migration corrects. |

## ✅ Migration complete

All seven phases are merged.  The historical-arch lane now matches
the `aarch64-backend` / `x86_64-backend` shape exactly:

```text
IIR  ─▶  aot_core::infer  ─▶  aot_core::specialise  ─▶  CIR
                                                         │
                                                         ▼  Backend::compile
                                                       bytes
```

Per-arch end state:

| Arch | Encoder crate | Backend crate | Deprecated IIR crate |
|------|---------------|---------------|----------------------|
| GE-225      | `ge225-encoder`      | `ge225-backend`      | `iir-to-ge225` v0.10.0 |
| Intel 4004  | `intel4004-encoder`  | `intel4004-backend`  | `iir-to-intel4004` v0.4.0 |
| ARMv7       | `armv7-encoder`      | `armv7-backend`      | `iir-to-armv7` v0.5.0 |
| Intel 8008  | `intel8008-encoder`  | `intel8008-backend`  | `iir-to-intel8008` v0.4.0 |
| RV32I       | `riscv-encoder`      | `riscv-backend`      | `iir-to-riscv` v0.4.0 |

`lang-aot` v0.12.0 routes every `--emit=<arch>` flag through the
new `Backend`-trait path.  The deprecated `iir-to-*` crates stayed
in the workspace for a time (with `#[deprecated]` attributes on
their public APIs) so existing downstream test invariants kept
regressing against the old byte sequences — a belt-and-braces
guarantee that the new path produced compatible output.

**Update (2026-08-17): the five deprecated `iir-to-*` crates have
been deleted.** Nothing outside each crate itself ever depended on
them (verified: no other `Cargo.toml` `path = "../iir-to-*"`
dependency, no non-comment `iir_to_*::` import elsewhere in the
workspace) — every reference left in `lang-aot` and elsewhere was
prose/doc-comments documenting byte-format provenance, which are
left in place as historical context. The corresponding
`code/specs/iir-to-{riscv,intel4004,intel8008,armv7,ge225}.md`
spec files are kept as a historical record of the original
(superseded) design, each now bannered as removed.

Each phase = 1 PR + babysitter cron + auto-merge + next-phase
kickoff.  Same cadence as the A5 cascade.

## New architectures — the 9-architecture expansion

With the five-arch migration complete, later lanes add **brand-new**
`{arch}-encoder` + `{arch}-backend` pairs for architectures that
never had an `iir-to-{arch}` predecessor to migrate away from — they
start at the correct `Backend`-trait-over-CIR layer from day one,
following exactly the pattern this document established.

**ARM1 (ARMv1)** — `arm1-encoder` + `arm1-backend`, minimal viable
(`const_*`/`ret_*` only, single-register `R0` allocator, same
trivial-ROM scope as `armv7-backend`).  Unlike a from-scratch lane,
ARM1's behavioral simulator (`arm1-simulator`, 2270 lines) already
existed complete in-tree, so this lane only needed the
encoder/backend split on top of it — no new simulator work.  One
architecture-specific design decision: ARM1/ARMv1 (1985) predates
the `BX`/link-register-return convention `armv7-backend` (its direct
architectural descendant, ARMv7-A) uses, so `ret_*`/`ret_void` lower
to `arm1-simulator`'s existing pseudo-halt instruction
(`SWI #0x123456`, intercepted by `execute_swi` to set
`halted() == true`) rather than a return-from-subroutine instruction
— see `code/specs/arm1-backend.md` for the full rationale.  Byte-for-
byte parity for the canonical `MOV R0, #42; SWI #0x123456` sequence
(`[0x2A, 0x00, 0xA0, 0xE3, 0x56, 0x34, 0x12, 0xEF]`, little-endian)
is verified both as a hand-derived byte array and by actually
executing the emitted bytes in `arm1-simulator` and asserting
`R0 == 42`.

**MOS 6502** — `mos6502-encoder` + `mos6502-backend`, minimal viable
(`const_*`/`ret_*` only, single-accumulator allocator, same trivial-ROM
scope as `arm1-backend`/`armv7-backend`).  Fifth lane of the expansion.
Unlike ARM1 (whose behavioral simulator pre-existed complete in-tree),
the MOS 6502 needed a brand-new Rust simulator (`mos6502-simulator`,
~1870 lines across 6 modules) — a full 151-opcode / 13-addressing-mode
port of the pre-existing Python simulator
(`code/packages/python/mos6502-simulator`, Layer 07j), including BCD
decimal-mode `ADC`/`SBC` and the documented indirect-`JMP` page-wrap
silicon bug, ported faithfully rather than skipped.  One
architecture-specific design decision (or rather, the *absence* of one):
unlike ARM1/ARMv1, which has no real halt instruction and required
`arm1-backend` to invent a pseudo-halt (`SWI #0x123456`), the MOS 6502
already has a genuine one-byte instruction, `BRK` (opcode `0x00`), that
the *pre-existing* Python simulator's own module docstring documents as
the established HALT convention: *"BRK (opcode 0x00) is treated as
HALT... matches the convention used throughout the simulator stack (HLT
for 8080, TRAP for IBM 704, etc.)"*.  `ret_*`/`ret_void` in
`mos6502-backend` therefore lower to `BRK` directly — mirroring this
repo's own established, pre-existing semantics for the ISA rather than
inventing a new KIL/JAM-illegal-opcode or self-jump-spin-loop
convention — see `code/specs/mos6502-backend.md` and
`code/specs/mos6502-encoder.md` for the full rationale and the
alternatives considered and rejected.  Byte-for-byte parity for the
canonical `LDA #42; BRK` sequence (`[0xA9, 0x2A, 0x00]`) is verified
both as a hand-derived byte array and by actually executing the
emitted bytes in `mos6502-simulator` and asserting `A == 42` and
`halted() == true`.

**Motorola 68000** — `m68k-encoder` + `m68k-backend`, minimal viable
(`const_*`/`ret_*` only, single-register `D0` allocator, same
trivial-ROM scope as `mos6502-backend`/`arm1-backend`).  Eighth lane of
the expansion.  Like MOS 6502 (and unlike ARM1, whose behavioral
simulator pre-existed complete in-tree), the 68000 needed a brand-new
Rust simulator alongside the encoder/backend pair — `m68k-simulator`, a
substantial-but-bounded port of the pre-existing Python simulator
(`code/packages/python/motorola-68000-simulator`, Layer 07n) covering
`MOVE`/`MOVEA`/`MOVEQ`, `ADD`/`SUB`/`AND`/`OR`/`EOR`/`CMP`,
`ADDQ`/`SUBQ`, `Scc`/`DBcc`, `BRA`/`BSR`/all 14 `Bcc`, register-form
shift/rotate, `CLR`/`NEG`/`NOT`/`TST`/`SWAP`/`EXT`/`LEA`/`JSR`/`JMP`,
and `NOP`/`RTS`/`RTR`/`STOP`/`TRAP`/`LINK`/`UNLK` — a genuinely useful
subset rather than a narrow stub, though (unlike MOS 6502's full
151-opcode port) not the complete ISA: the line-0 immediate group
(`ORI`/`ANDI`/`SUBI`/`ADDI`/`EORI`/`CMPI`), bit ops, `DIVU`/`DIVS`,
`MULU`/`MULS`, `ADDX`/`SUBX`/`NEGX`, and 3 of the 11 addressing-mode
variants (the indexed and PC-relative forms, which need a second
extension word to decode) are deferred — see `code/specs/
m68k-backend.md` for the full ported-vs-deferred accounting.  One
architecture-specific design decision: the 68000 (unlike ARM1) has
*two* genuinely real halting instructions — `STOP #imm` and
`TRAP #15` — and the *pre-existing* Python simulator's own `state.py`
documents both as valid ("halted: True after STOP or TRAP #15
executes"); `ret_*`/`ret_void` in `m68k-backend` lower to `TRAP #15`,
not `STOP #imm`, because the Python original's own test suite settles
which one is the established idiom — its `_stop()` helper (used 100+
times) is `TRAP #15`, while `STOP #imm` appears exactly once, in a
module-level doctest — see `code/specs/m68k-backend.md` and
`code/specs/m68k-encoder.md` for the full rationale.  Byte-for-byte
parity for the canonical `MOVE.L #42, D0; TRAP #15` sequence
(`[0x20, 0x3C, 0x00, 0x00, 0x00, 0x2A, 0x4E, 0x4F]`, big-endian — the
68000's native byte order, unlike every little-endian target above) is
verified both as a hand-derived byte array and by actually executing
the emitted bytes in `m68k-simulator` and asserting `D0 == 42` and
`halted == true`.  This lane also applies, from the start, the
termination-check fix a prior lane (Intel 8051) needed a security-review
round to discover: `compile_to_bytes` tracks an explicit
`terminated: bool` rather than comparing the trailing emitted byte
against the halt sentinel's value (which would be unsound here too —
`TRAP #15`'s low byte, `0x4F`, is also reachable as the low byte of a
`MOVE.L #imm, D0` immediate).

Other lanes in this expansion (e.g. MIPS R2000, first lane) may land
in parallel PRs and are not enumerated here to avoid merge
conflicts with concurrently-landing work — see each lane's own
`code/specs/{arch}-encoder.md` / `code/specs/{arch}-backend.md` for
its specifics.

## What about `Backend::run` — and JIT in general?

For the real native backends (`aarch64`, `x86_64`), `Backend::run`
actually executes the binary in-process via the JIT loader.  The
historical-arch targets have no in-process executor — we emit
bytes for downstream simulators (or, in the GE-225 case, just for
posterity).

**JIT support for the historical arches is explicitly best-effort,
and "no working JIT" is an acceptable outcome for any individual
arch.**  The migration's primary goal is correct **AOT** lowering
through `aot-core` + the `Backend` trait; the JIT side is a free
side benefit that we take *if it's cheap*.

Concretely, each `{arch}-backend` crate satisfies the `Backend`
trait as follows:

- `name()` — returns `"{arch}"`.
- `compile()` / `compile_function()` — does the real work,
  returns `Some(bytes)` for supported CIR ops, `None` otherwise
  (which AOT treats as a per-function compile failure and JIT
  treats as "stay on interpreter tier").
- `run()` — **panics** with `"{arch} backend is emit-only; load
  bytes into a {arch} simulator to execute"`.  The function exists
  to satisfy the trait so the backend can plug into the same
  registry as `aarch64-backend` / `x86_64-backend`, but no caller
  should reach it.

If a future increment wants real JIT execution for one of these
arches, it can:

1. Wire `Backend::run` to forward to an in-tree simulator —
   `ge225-simulator`, `intel4004-simulator`, `intel8008-simulator`,
   `arm-simulator`, and `riscv-simulator` all already exist in
   the workspace.
2. Or skip `jit-core` registration entirely for that arch and just
   keep it on the AOT path.

Either is fine.  **Don't gate the migration on getting JIT working
for all five historical arches** — the architectural correctness
win is the AOT-side move from IIR to CIR, which is delivered as
soon as the AOT path is wired.

## What about `Backend::compile` returning `None`?

Per the trait docs, `None` means "compilation failed; fall back to
interpreter".  For historical-arch backends:

- **AOT path**: a `None` causes `aot_core` to report a compile
  failure for that function (same as any backend).  The
  user-visible behaviour is identical to today's "UnsupportedOp"
  error from `iir-to-{arch}`.
- **JIT path**: the function stays on the interpreter tier — same
  graceful fallback every other backend gets.

Far cleaner than the bespoke `IIR{Arch}Error::UnsupportedOp`
variants my `iir-to-*` crates invented.

## Migration order rationale

GE-225 goes first because the bytes are fresh in my head and the
trivial-case ROM sizes are still pinned in my recent commits.
Intel 4004 second because its allocator pattern mirrors GE-225's
17-slot pool.  Then ARMv7 (most complex of the historical lane),
Intel 8008 (Oct's native — touched by many call sites), and RV32I
last (largest, and the original mistake from A1+ that started this
whole pattern).

## Post-migration: 9-architecture expansion (2026-08)

Having proven the CIR-via-`Backend`-trait pattern across the five
original arches, a further expansion is bringing more historical/
production ISAs onto the same `{arch}-encoder` + `{arch}-backend`
split — this time starting at the correct layer from day one (no
`iir-to-{arch}` predecessor to deprecate, unlike the five phases
above).

- **Intel 8080** — `intel8080-encoder` + `intel8080-backend` (minimal
  viable: `const_*` + `ret_*` only, same scope decision the original
  five phases made) + a new-from-scratch `intel8080-simulator`
  (Rust port of the existing Python behavioral simulator) + `lang-aot`
  wiring (`--emit=intel8080`).  Byte-for-byte parity for the trivial
  `MVI A, 42; HLT` = `[0x3E, 0x2A, 0x76]` ROM verified against the new
  simulator by actually executing the emitted bytes, not just
  asserting the byte array.  The 8080 (1974) is the Intel 8008's
  direct architectural successor — same 8-bit accumulator model, same
  `HLT` opcode (`0x76`) — so `intel8080-backend` reuses
  `intel8008-backend`'s exact shape, unlike ARM1's SWI pseudo-halt or
  MIPS R2000's `JR $ra`, which needed bespoke return-mechanism
  handling.

Further lanes in this expansion each get their own entry here as they
land.

## Non-goals

- No new functional coverage — every migration preserves the byte
  sequences the IIR-level crate emitted.  Existing trivial-ROM
  byte traces stay pinned (just via CIR inputs now).
- No in-process simulator implementations.
- No changes to `aarch64-backend` / `x86_64-backend` — they're
  already correct.
- No changes to `iir-to-llvm` / `iir-to-wasm` / `iir-to-jvm` /
  `iir-to-clr` / `iir-to-beam` — those targets stay typed at the
  IR level (LLVM IR, WASM, JVM bytecode, CIL, BEAM) and are
  correctly hooked at IIR.  Only the **real native bytes** path
  needs to move to CIR.
