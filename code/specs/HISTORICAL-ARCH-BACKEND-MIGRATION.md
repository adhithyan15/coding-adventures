# Historical-arch backend migration — `iir-to-*` → `*-encoder` + `*-backend`

**Status:** Phase 1 in progress.
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
| **1** | `ge225-encoder` carve-out | New crate with constants + `encode_*`.  `iir-to-ge225` re-exports from it so there's one source of truth. |
| **2** | `ge225-backend` skeleton + ops | New crate implementing `Backend`.  Covers the same op set `iir-to-ge225` v0.9.0 did, via CIR. |
| **3** | `ge225-backend` wiring + `iir-to-ge225` deprecation | `lang-aot --emit=ge225` routes through `aot_core::link` + `ge225-backend`.  `jit-core` can register it.  `iir-to-ge225` becomes a thin `#[deprecated]` shim. |
| **4** | Intel 4004 migration | `intel4004-encoder` + `intel4004-backend` + wiring + deprecate `iir-to-intel4004`. |
| **5** | ARMv7 migration | `armv7-encoder` + `armv7-backend` + wiring + deprecate `iir-to-armv7`. |
| **6** | Intel 8008 migration | `intel8008-encoder` + `intel8008-backend` + wiring + deprecate `iir-to-intel8008`. |
| **7** | RV32I migration | `riscv-encoder` + `riscv-backend` + wiring + deprecate `iir-to-riscv`. |

Each phase = 1 PR + babysitter cron + auto-merge + next-phase
kickoff.  Same cadence as the A5 cascade.

## What about `Backend::run`?

For the real native backends (`aarch64`, `x86_64`), `Backend::run`
actually executes the binary in-process via the JIT loader.  The
historical-arch targets have no in-process executor — we emit
bytes for downstream simulators (or, in the GE-225 case, just for
posterity).

These crates satisfy `Backend::run` by **panicking** with a clear
"{arch} backend is emit-only; load `{bytes}` into a {arch} simulator
to execute" message.  The function exists to satisfy the trait so
the encoders/backends can be registered with `jit-core`, but
nobody should call it.

A future increment could add an in-process simulator for one of
these arches (e.g. `ge225-simulator` already exists in the
workspace), at which point `run` would forward to it.

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
