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

## Current scope — WORD02 arithmetic

| CIR op family | Lowering |
|----------------|----------|
| `const_u8`, `const_bool` | `MOV AX, #imm16`, zero-extended (`AH = 0`) |
| `const_u16` | `MOV AX, #imm16` |
| `add`/`sub`/`and`/`or`/`xor` on `u8` and `u16` | register-to-register ALU operations through `AX`/`BX` |
| `not_u8`, `not_u16` | width mask followed by register `XOR` |
| matching `ret_u8`, `ret_bool`, `ret_u16` | `HLT` (only if returning the current value at the matching width) |
| `ret_void` | `HLT` |
| Empty CIR body | `HLT` |
| Anything else | `UnsupportedOp` from `compile()`; `None` from the `Backend::compile` trait method |

The bounded allocator supports two same-width live values in `AX`/`BX`; `CX`
is scratch for reversed subtraction and byte-result masking. Binary operations
consume both operands and leave their result in `AX`. Byte and bool values are
zero-extended so `AL` contains the result and `AH == 0`; word values occupy all
of `AX`. More than two live values and mixed-width live sets fall through to
`UnsupportedOp`. AOT treats `None` as a per-function compile failure; JIT keeps
execution on the interpreter tier. Comparisons and control flow remain outside
this increment.

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
| Word `0x1234` | `const_u16 v=0x1234; ret_u16 v` | `[0xB8, 0x34, 0x12, 0xF4]` |
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
| `UndefinedVariable(String)` | A typed return has no current value |
| `ImmediateOutOfRange(i64)` | A literal falls outside its selected unsigned width (`u8` or `u16`) |

## WORD01 execution proof

`tests/test_backend.rs::word_u16_result_executes_in_ax` runs
`MOV AX,0x1234; HLT` in `intel8086-simulator` and asserts the full word result.
A companion test proves a `u8` boundary produces `AX == 0x00ff`, while other
regressions reject 256 as a `u8` and reject a `ret_u8` for a current `u16`
value.

## WORD02 execution proof

The backend tests execute `u8(0xff) + u8(2) == 1`,
`u16(0xffff) + u16(2) == 1`, and
`u16(0x1234) ^ u16(0x00ff) == 0x12cb` in `intel8086-simulator` through
non-zero-`CS` segmented fetch. Companion tests cover subtraction and every
bitwise operation at both widths.

## Tests

23 unit/integration tests in `tests/test_backend.rs` (mirroring
`mos6502-backend`'s/`arm1-backend`'s test shape) pin the canonical byte
sequence and edge cases (zero, 16-bit range boundaries — negative and
`>65535` — bool, bounded allocation, unsupported op, empty CIR,
`ret_void`, `Backend::run` panics, `Backend::compile` vs the free
`compile` function agree).

Several tests additionally load the compiled bytes into
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

1. [ ] Expand the bounded two-value allocator with stack spills when later
   Word programs require it.
2. [ ] Comparisons and conditional branches using `CMP` plus conditional
   jumps.
3. [ ] Memory-operand support (loads/stores through `[BX+SI]` and
   friends) — this needs the effective-address computation
   `intel8086-simulator`'s `decode.rs` explicitly defers, so this item
   is gated on a simulator-side increment first.
4. [ ] Direct calls (`CALL`/`RET` pairing) and a stack frame — once
   this lands, `ret_*` could switch from `HLT` to `RET` for called
   functions (the `HLT` would remain for the outermost program-exit
   case, matching how other lanes' backlogs plan to keep their halt
   convention for program exit even after adding real calls).
5. [ ] `Backend::run` wired to `intel8086-simulator` for JIT execution
   (best-effort per the migration spec — "no working JIT" is an
   acceptable outcome for a historical-arch target).
