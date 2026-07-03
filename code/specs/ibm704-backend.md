# `ibm704-backend` spec

> **Status:** v0.1.0 — L4 of the McCarthy Lisp implementation,
> 2026-06-11.

## Purpose

IBM 704 implementation of the `jit_core::backend::Backend` trait.
Mirror of `ge225-backend` / `intel4004-backend` / `armv7-backend` /
`intel8008-backend` / `riscv-backend`.

Lowers `Vec<CIRInstr>` (typed, monomorphised) to a `Vec<u8>` of
5-byte-per-36-bit-word IBM 704 machine code, via
`ibm704-encoder`.

## Why this crate exists — the CAR/CDR birthplace round-trip

McCarthy and his students Steve Russell, Tim Hart, and Mike Levin
ran the **first Lisp implementation** on an IBM 704 at MIT in
1959.  `CAR` and `CDR` — the two universal Lisp accessors — were
literal IBM 704 instruction-word field mnemonics:

* **CAR** = **C**ontents of the **A**ddress part of a **R**egister
* **CDR** = **C**ontents of the **D**ecrement part of a **R**egister

This backend lets McCarthy Lisp source compile back to that
silicon — the closing half of the LANG VM's two-historical-
languages-on-their-birthplace-machines story (the other half is
Dartmouth BASIC → GE-225, already shipped).

## v0.1.0 scope — minimal viable

Per the McCarthy Lisp v0.1.0 scope decision (no CONS on
historical-arch backends — confirmed decision 3 in
`MCCARTHY-LISP-PLAN.md`), this backend handles only:

| CIR op family | Lowering |
|---------------|----------|
| `const_*` (15-bit unsigned immediate) | `CLA n` — load `n` into the accumulator |
| `ret_*` (value matches the last `const_*` dest) | `HTR 0` — canonical halt; final value lives in AC |
| `ret_void` | `HTR 0` |
| Empty CIR body | `HTR 0` |

Anything else returns `BackendError::UnsupportedOp(op)` from the
inherent `compile()`, or `None` from `Backend::compile`.

The accumulator-tracking pattern mirrors `intel8008-backend`'s
v0.1.0: a single-tracked "current accumulator var" gets loaded
by `CLA n` and read back by `HTR 0` on exit.

## Wire format

Each instruction is one 36-bit IBM 704 word, packed as 5 bytes
(low byte first; high 4 bits of the top byte are always zero).
Per-function byte streams concatenate directly — `lang-aot`
writes them straight to disk as a flat `.bin`.

## Pinned byte sequences

| Program | CIR | Emitted bytes |
|---------|-----|---------------|
| Twig `42` / McCarthy `42` | `const_i64 v=42; ret_i64 v` | `[0x2A, 0x00, 0x00, 0x00, 0x0A, 0x00, 0x00, 0x00, 0x80, 0x08]` |
| `ret_void` only | `ret_void` | `[0x00, 0x00, 0x00, 0x80, 0x08]` |
| Empty CIR | (none) | `[0x00, 0x00, 0x00, 0x80, 0x08]` |
| `(CONS 1 2)` etc. | `… cons …` | (returns `UnsupportedOp` per v0.1.0 scope) |

## Backend trait surface

| Trait method | Behaviour |
|--------------|-----------|
| `name()` | returns `"ibm704"` |
| `compile(ir)` | returns `Some(bytes)` for supported CIR ops; `None` otherwise |
| `compile_function(ctx, ir)` | identical to `compile(ir)` |
| `run(binary, args)` | **panics** with `"ibm704 backend is emit-only…"` — a future `ibm704-simulator` could wire this for actual execution |

## Error variants

| `BackendError` variant | Trigger |
|------------------------|---------|
| `UnsupportedOp(String)` | CIR op outside v0.1.0's coverage |
| `InvalidOperand(String)` | `ret_*` srcs[0] isn't a `Var`, `const_*` missing a dest, etc. |
| `ImmediateOutOfRange(i64)` | `const_*` value outside `[0, 32767]` (15-bit CLA window) |

## Tests (11 byte-pinned unit tests + 2 e2e in lang-aot)

* Empty CIR emits the canonical 5-byte halt.
* Backend name is `"ibm704"`.
* `Backend::run` panics with the documented message.
* Twig `42` canonical produces the 10-byte sequence above.
* `const_i64 0` → `[..00 00 00 00 0A, halt]`.
* `const_bool true` acts as immediate-1.
* `const_i64 32767` (max 15-bit) is accepted.
* `const_i64 32768` reports `ImmediateOutOfRange`.
* Negative consts (`-1`) report `ImmediateOutOfRange` (704 CLA address field is unsigned).
* `ret_void`-only program is just `HTR 0`.
* Unsupported op (`add_i64`) reports `UnsupportedOp`.
* Multi-const ret-first falls through to `UnsupportedOp` (single-tracked accumulator).
* **e2e in lang-aot:** `end_to_end_twig_42_emits_ibm704_bin_via_lang_aot` pins the full 10-byte sequence through the entire IIR → CIR → backend pipeline.
* **e2e in lang-aot:** `end_to_end_mccarthy_42_emits_ibm704_bin_via_lang_aot` — McCarthy Lisp source `42` → identical bytes (the IIR convergence in action).

## Out of scope (future increments)

* Arithmetic, comparison, branches, calls, locals — would mirror what `intel8008-backend`'s richer roadmap covers.
* CONS — needs a static heap area at a fixed address (not in the v0.1.0 scope decision for any historical-arch backend).
* Real execution via a future `ibm704-simulator`.
* Type A instructions (decrement-field, transfer with index increment).
