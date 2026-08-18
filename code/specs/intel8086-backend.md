# `intel8086-backend` spec

> **Status:** v0.1.0 — ninth and **final** lane of the 9-architecture
> expansion, 2026-08-17.

## Purpose

Intel 8086 (1978) implementation of the `jit_core::backend::Backend`
trait. Mirror of `mos6502-backend` / `arm1-backend` / `armv7-backend`
(the *minimal viable* shape). The Intel 8086 is the direct architectural
ancestor of every x86 CPU made today — its cheaper, 8-bit-external-bus
sibling the 8088 shipped in the original IBM PC (1981), founding the
"PC-compatible" industry that has dominated general-purpose computing
for over four decades. Segmented memory (`physical = segment×16 +
offset`) was its defining, and later controversial, architectural
choice.

Lowers `Vec<CIRInstr>` (typed, monomorphised) to `Vec<u8>` of Intel 8086
machine code via `intel8086-encoder`.

## Why this crate exists

This is the **ninth and final lane** of a 9-architecture expansion that
replicates the pattern established by the historical-arch backend
migration (see
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](HISTORICAL-ARCH-BACKEND-MIGRATION.md)):
consume typed **CIR** (not dynamically-typed IIR) via the shared
`Backend` trait, so `lang-aot --emit=intel8086` routes through the same
`aot_core::infer` + `aot_core::specialise` + `Backend::compile` pipeline
every other arch backend (including `aarch64-backend` / `x86_64-backend`)
uses. The Intel 8086 never had an `iir-to-intel8086` predecessor to
migrate away from — this crate starts at the correct layer from day
one, same as every other lane in this expansion.

Unlike ARM1 (whose behavioral simulator pre-existed complete in-tree)
or MOS 6502/RV32I (which needed brand-new from-scratch Rust
simulators), the Intel 8086 needed a **new Rust simulator that ports
only a curated core** of an unusually large Python reference
(`code/packages/python/intel-8086-simulator`, ~1670 lines implementing
essentially the full ISA) — see `intel8086-simulator`'s crate-level doc
for the full scoping rationale and the "deferred" list.

## Segmented memory — structural, not deferrable

The 8086's defining feature — `physical_address = (segment_register<<4)
+ offset` — is **not** a scoping choice like the memory-operand
addressing modes this lane defers. Even the trivial `const 42; ret`
program this backend compiles has its first opcode byte fetched via
`CS:IP` segmented addressing when loaded into `intel8086-simulator`.
This backend's own output is unaffected by segmentation (it emits a
flat byte stream, same as every other lane), but the *simulator* that
executes those bytes for verification purposes cannot use a flat-memory
shortcut the way `mos6502-simulator`/`arm1-simulator`/`riscv-simulator`
do — see `code/specs/07m-intel-8086-simulator.md` and
`intel8086-simulator/src/simulator.rs`'s module doc (`phys_addr`) for
the exact formula and its 20-bit wraparound behaviour.

## Current scope — minimal viable

| CIR op family | Lowering |
|----------------|----------|
| `const_*` (unsigned 16-bit literal, `[0, 65535]`) | `MOV AX, #imm16` |
| `ret_*` | `HLT` (only if returning the most recently `const_*`'d variable) |
| `ret_void` | `HLT` |
| Empty CIR body | `HLT` |
| Anything else | `UnsupportedOp` from `compile()`; `None` from the `Backend::compile` trait method |

There is **no real register allocator** — a trivial "last const var"
scheme tracks which single variable the most recent `const_*` wrote
into the accumulator (`AX` — the 8086's primary 16-bit accumulator and
return-value register); `ret_*` only succeeds if it returns exactly
that variable. Programs needing more than one live value fall through
to `UnsupportedOp`. AOT treats `None` as a per-function compile
failure; JIT keeps execution on the interpreter tier.

Full op coverage (arithmetic, register-to-register moves, control flow
— a mature backend's worth) is **intentionally not wired into this
backend** in this PR, even though `intel8086-simulator` already
implements a curated core of those mnemonics. Future increments can
extend `intel8086-backend::compile_to_bytes` to emit `ADD`/`SUB`/`MOV
reg,reg` using the encoder helpers `intel8086-encoder` already
re-exports — the simulator-side work to execute them is already done.

## Why `ret_*` lowers to real `HLT`, not a pseudo-halt

See `code/specs/intel8086-encoder.md`'s "Why `HLT`, not a pseudo-halt or
repurposed opcode?" section for the full three-way comparison against
ARM1's invented `SWI` pseudo-halt and MOS 6502's repurposed `BRK`. The
short version: `HLT` is a genuine, single-byte, no-operand hardware
instruction whose sole documented purpose is halting the fetch-decode-
execute loop — the least-invented halt-related decision anywhere in
this 9-architecture expansion, ported directly from the Python
reference's `if op == 0xF4: self._halted = True`.

## The `terminated: bool` pattern — and the bug class it avoids

**A real bug was found and fixed in four prior lanes of this campaign:
Intel 8051, Intel 8080, MOS 6502, and Zilog Z80.** In each case, the
backend's defensive "is the program already terminated?" check compared
the trailing byte(s) of the emitted buffer against the architecture's
halt-opcode byte value (or, in a worse variant, checked
`bytes.is_empty()`). Both forms are unsound:

- **Trailing-byte-value comparison** breaks because a legitimate
  `const_*` immediate's *own encoded bytes* can numerically collide
  with the halt opcode. For this lane specifically: `HALT_BYTE` is
  `0xF4`, and `MOV AX,#imm16` encodes as `[0xB8, imm_lo, imm_hi]`. An
  immediate like `0xF400` therefore encodes as `[0xB8, 0x00, 0xF4]` —
  trailing byte `0xF4`, byte-identical to `HLT`, despite this program
  never having executed a real halt instruction. A naive check would
  conclude "already terminated" and skip appending the real `HLT`,
  silently shipping a program with **no genuine halt instruction** at
  all — the CPU would fetch whatever garbage byte follows in memory as
  the next opcode.
- **`is_empty()`** breaks for a different reason: any `const_*` at all
  makes the output buffer non-empty long before a real terminator is
  ever emitted, so `is_empty()` can never correctly answer "has a
  terminator been emitted yet?" once the compile loop is underway.

`intel8086-backend` avoids the entire bug class structurally: it tracks
an explicit `terminated: bool` local, never inspecting trailing byte
values at all.

```text
terminated = false
for instr in cir:
    match instr.op:
        "ret_*" | "ret_void"  => emit HLT; terminated = true
        "const_*"             => emit MOV AX,#imm; terminated = false
        other                 => UnsupportedOp
if not terminated:
    emit HLT
```

- Starts `false`.
- Set `true` **only** by a genuine `ret_*`/`ret_void` arm pushing a real
  `HLT`.
- Reset to `false` by every subsequent `const_*` (or any other non-
  terminating instruction) — the crux of the pattern: a byte-value
  check has no equivalent "reset" step, which is exactly how the bug
  class this avoids slips in.
- The final defensive append checks the flag, not the buffer's trailing
  byte.

`tests/test_backend.rs`'s
`const_whose_encoded_high_byte_collides_with_halt_opcode_still_gets_real_terminator`
is a dedicated regression test proving a `const_i64 v=0xF400` program
with **no** `ret` at all still gets a real `HLT` appended — a naive
trailing-byte-comparison implementation would fail this exact test (it
would see the buffer already ending in `0xF4` and wrongly skip the
terminator).

## Wire format

Multi-byte immediates are little-endian *within* each instruction
(matching the 8086's native byte order), but there is no fixed
instruction-word width to flatten across the whole output — unlike
`arm1-backend`/`mips-r2000-backend`'s 32-bit-word targets, the
encoder's `Vec<u8>` bytes are already the final wire format.
Per-function byte streams concatenate directly; `lang-aot` writes them
straight to disk as a flat `.bin`.

## Pinned byte sequence

| Program | CIR | Emitted bytes |
|---------|-----|----------------|
| IIR `42` | `const_i64 v=42; ret_i64 v` | `[0xB8, 0x2A, 0x00, 0xF4]` |
| `ret_void` only | `ret_void` | `[0xF4]` |
| Empty CIR | (none) | `[0xF4]` |

`MOV AX,#42` = `[0xB8, 0x2A, 0x00]`; `HLT` = `[0xF4]`.

## Backend trait surface

| Trait method | Behaviour |
|---------------|-----------|
| `name()` | returns `"intel8086"` |
| `compile(ir)` | returns `Some(bytes)` for supported CIR ops; `None` otherwise |
| `compile_function(ctx, ir)` | ignores `FunctionContext` (no parameter marshalling in v0.1.0); delegates to `compile` |
| `run(binary, args)` | **panics** with `"intel8086 backend is emit-only; load bytes into intel8086-simulator to execute"` — emit-only per the migration spec |

## Error variants

| `BackendError` variant | Trigger |
|--------------------------|---------|
| `UnsupportedOp(String)` | CIR operation outside `const_*`/`ret_*` |
| `InvalidOperand(String)` | Malformed CIR operands or missing `dest` |
| `UndefinedVariable(String)` | Reserved for a future register allocator (unused in v0.1.0's single-var scheme, where the "not the current AX var" case surfaces as `UnsupportedOp` instead) |
| `ImmediateOutOfRange(i64)` | A `const_*` literal falls outside `[0, 65535]` — `MOV reg16,#imm16`'s unsigned 16-bit immediate field (`AX` is 16 bits wide) |

## Tests

19 unit/integration tests in `tests/test_backend.rs` (mirroring
`mos6502-backend`'s/`arm1-backend`'s test shape) pin the canonical byte
sequence and edge cases (zero, 16-bit range boundaries — negative and
`>65535` — bool, multi-var fallthrough, unsupported op, empty CIR,
`ret_void`, `Backend::run` panics, `Backend::compile` vs the free
`compile` function agree).

Two tests additionally load the compiled bytes into
`intel8086-simulator` and genuinely execute them (through non-zero-`CS`
segmented addressing, not a flat-memory shortcut) — byte-for-byte
parity is necessary but not sufficient; the emitted bytes must actually
execute correctly (and actually halt) in the new simulator:

* `canonical_const_42_then_ret_actually_executes_to_ax_equals_42` —
  the `const 42; ret` program, asserting `AX == 42` and `halted ==
  true` after execution at `CS=0x0010`.
* `const_whose_encoded_high_byte_collides_with_halt_opcode_still_gets_real_terminator`
  — the `terminated: bool` regression test described above, which also
  executes the emitted bytes and confirms `AX == 0xF400` and
  `halted == true` (i.e. the emitted program genuinely halts, rather
  than running off the end of a too-short byte buffer).

## Backlog

1. [ ] Real register allocator using the 8086's other general-purpose
   registers (`BX`/`CX`/`DX`) and the stack, removing the single-var
   limitation.
2. [ ] Arithmetic/logical CIR ops (`add`/`sub`/`and`/`or`/`xor`/`cmp`)
   via the accumulator-immediate and register-to-register ALU
   instructions `intel8086-simulator` already implements — only the
   backend-side lowering + a wider `intel8086-encoder` re-export list
   are missing.
3. [ ] Memory-operand support (loads/stores through `[BX+SI]` and
   friends) — this needs the effective-address computation
   `intel8086-simulator`'s `decode.rs` explicitly defers, so this item
   is gated on a simulator-side increment first.
4. [ ] Comparisons and conditional branches. Unlike ARM1's per-
   instruction condition-code field, the 8086 needs an explicit
   `CMP`-then-conditional-jump pairing (closer to
   `mips-r2000-backend`'s branch story than ARM1's).
5. [ ] Direct calls (`CALL`/`RET` pairing) and a stack frame — once
   this lands, `ret_*` could switch from `HLT` to `RET` for called
   functions (the `HLT` would remain for the outermost program-exit
   case, matching how other lanes' backlogs plan to keep their halt
   convention for program exit even after adding real calls).
6. [ ] `Backend::run` wired to `intel8086-simulator` for JIT execution
   (best-effort per the migration spec — "no working JIT" is an
   acceptable outcome for a historical-arch target).
