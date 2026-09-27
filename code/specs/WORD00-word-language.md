# WORD00 — Word cross-target language contract

> **Status:** design selected for VM-072. This document does not claim a
> frontend or a non-trivial Z80/8086 backend implementation.

## Purpose

Word is the next gradual language rung after Nib and Oct. It is one language
with two deliberately incompatible machine-code targets: the Zilog Z80 and
Intel 8086. The teaching goal is to compile and execute the same fixed-width
source program on an 8-bit accumulator machine and a 16-bit segmented machine,
then inspect why the emitted programs differ.

The name describes the new concept introduced by the rung, not a
target-dependent type. Word source never has an ambiguous `word` integer type;
it spells widths as `u8` and `u16`.

## Decision: one language, not two

The two-target language is selected over consecutive Z80-only and 8086-only
languages.

- Nib already teaches a 4-bit accumulator and Oct teaches an 8-bit
  accumulator. A second source language that differs from Oct mainly by using a
  Z80 would add another lexer/parser/type-checker surface before it adds a new
  source concept.
- The Z80 has 16-bit register pairs and a 16-bit address space even though its
  ALU is primarily 8-bit. The 8086 has native 16-bit general registers. A fixed
  `u16` source value therefore creates an honest comparison: synthesized or
  pair-based work on Z80, native word work on 8086.
- One frontend gives every conformance program the same syntax, types, and
  expected result. Target-specific code generation remains separate, and no
  machine-code compatibility is implied.
- Architecture-only facilities remain namespaced extensions. They cannot
  silently leak into the portable subset or make a one-target program look
  portable.

This decision can be revisited only if an executed proof shows that the
portable contract requires a semantic compromise, not merely because one
backend needs more instructions.

## Evidence from the current repository

The design starts from executed components but does not confuse them with a
complete language pipeline.

| Component | Current evidence | Gap before Word |
|---|---|---|
| shared AOT/CIR typing | `u8`, `u16`, `bool`, and control-flow operations are allowlisted | historical backends do not lower the required operations |
| `z80-backend` | `const_*` to `LD A,n`; `ret_*` to `HALT`; executed in `z80-simulator` | literals stop at 255, only one live variable, no arithmetic/memory/branch/call/I/O lowering |
| Z80 ISA support | simulator covers register pairs, ALU, jumps, calls, stack, memory, and I/O; its internal encoder has helpers for several of these | the public encoder/backend surface is still minimal |
| `intel8086-backend` | unsigned 16-bit constant to `MOV AX,imm16`; `ret_*` to `HLT`; executed through segmented fetch | only one live variable and no arithmetic/memory/branch/call/I/O lowering |
| 8086 ISA support | the behavioral simulator has broad instruction coverage; the stable encoder already exports register moves and accumulator add/sub helpers | backend allocation and the remaining encoder-facing lowering contract are absent |
| Oct | a complete hardware-shaped 8008 language pipeline | its carry, rotation, port, register, and binary assumptions are 8008-specific; its machine code is not reusable |

The existing constant-return smoke tests prove that both targets can execute a
byte stream from the shared AOT route. They do not prove Word.

## Portable v1 surface

### Types

Word v1 has four source types:

- `u8`: unsigned 8-bit value, range 0 through 255;
- `u16`: unsigned 16-bit value, range 0 through 65,535;
- `bool`: exactly `false` or `true`, represented at target boundaries as zero
  or one;
- `void`: function with no returned value.

There are no signed integers, floating point, strings, heap values, pointers,
casts, or target-sized integers in v1. Integer literals are checked against the
declared destination type before lowering.

### Expressions and statements

The portable surface contains:

- explicit declarations and assignment;
- wrapping `+`, `-`, `&`, `|`, `^`, and `~` on matched integer widths;
- `==`, `!=`, `<`, `<=`, `>`, and `>=`, returning `bool`;
- `if`/`else`, `while`, `break`, and `return`;
- non-recursive direct functions with explicitly typed parameters and return
  values;
- fixed-size byte arrays whose length is known at compile time;
- `in8(port: u8) -> u8` and `out8(port: u8, value: u8)` as the shared portable
  I/O boundary.

Arithmetic wraps modulo 256 for `u8` and modulo 65,536 for `u16`. The frontend
must preserve the source width in IIR/CIR (`u8` or `u16`); it must not widen an
operation to `i64` and rely on a target accident to truncate it later.

Multiplication, division, dynamic allocation, recursion, indirect calls,
interrupt handlers, concurrency, and variable-length arrays are outside v1.

### Memory model

Portable Word exposes a 16-bit logical byte address space for fixed arrays and
array indices. The first implementation may allocate only statically known
objects.

- On Z80, the logical address is the architectural 16-bit address.
- On 8086, portable data access is near access through `DS:offset`; the host
  establishes `DS`, and the source-visible address is only the 16-bit offset.

Far pointers, segment arithmetic, segment overrides, and physical 20-bit
addresses are 8086-specific extensions, not portable Word values. This keeps
segmentation visible in emitted-code lessons without changing the meaning of a
portable source address.

### Architecture-specific extensions

Target-only operations use explicit namespaces and are rejected for the other
target. Examples reserved for later contracts include Z80 alternate-register,
block, and interrupt-mode operations, and 8086 segment/far-pointer and string
prefix operations. Neither namespace is part of the v1 completion gate.

The shared `in8`/`out8` functions are intentionally portable because both
targets have an 8-bit port operation with a 0-through-255 port boundary. Wider
or indirect port forms remain target-specific.

## Target ABI

The initial ABI is intentionally small and observable.

| Boundary | Z80 | Intel 8086 |
|---|---|---|
| `u8` result | `A` | `AL` (with `AH = 0` at the boundary) |
| `u16` result | `HL` | `AX` |
| `bool` result | `A`, normalized to 0/1 | `AL`, normalized to 0/1 and `AH = 0` |
| outermost exit | `HALT` | `HLT` |

Direct-call parameter placement and spill frames are deferred until WORD04.
They must be specified before any frontend function call is accepted. A called
function will use the architectural `CALL`/`RET`; only the outermost entry
function uses the halt convention.

## Delivery ladder

Each step is one bounded work item and must execute its emitted bytes in both
behavioral simulators. Byte assertions alone are insufficient.

### WORD01 — fixed-width result ABI

Teach the two Rust backends the exact `u8`/`u16` constant-and-return boundary.
The discriminating proof is `const_u16 0x1234; ret_u16`: Z80 must finish with
`HL == 0x1234`; 8086 must finish with `AX == 0x1234`. Keep the single-live-value
restriction. Do not add arithmetic, a frontend, or pretend the byte streams are
compatible.

### WORD02 — two-live-value arithmetic

Add bounded allocation plus wrapping add/subtract and bitwise operations for
`u8` and `u16`. Execute at least these results on both targets:

- `u8(0xff) + u8(2) == 1`;
- `u16(0xffff) + u16(2) == 1`;
- `u16(0x1234) ^ u16(0x00ff) == 0x12cb`.

The Z80 proof must exercise a real 16-bit pair path rather than comparing only
the low byte. The 8086 proof must use 16-bit registers rather than a host-side
calculation.

### WORD03 — comparisons and structured control

Lower normalized comparisons, labels, conditional/unconditional branches,
and loops. Execute a loop whose result depends on both a taken and untaken
branch. Resolve branch addresses after final instruction sizing; do not encode
source instruction indices as byte offsets.

### WORD04 — static memory, portable I/O, and direct calls

Add fixed byte arrays, checked `u16` indices, `in8`/`out8`, direct calls, and
the target call-frame contracts. Execute a store/load round trip and an I/O
echo on both simulators. Bounds failures must be compile-time errors when
provable and deterministic runtime traps otherwise.

### WORD05 — frontend and same-source acceptance

Only after WORD01 through WORD04 land, add the grammar, lexer, parser, type
checker, IIR compiler, formatter, CLI route, and language documentation. The
same checked source programs must compile through the shared AOT/CIR path and
execute on both simulators.

## Completion gate

Word v1 is complete only when all of the following are true:

1. One source file can target Z80 and 8086 without source edits.
2. The executed result agrees for width wraparound, comparison/branching, a
   loop, static memory, a direct call, and 8-bit port I/O.
3. Every emitted artifact is run by the target behavioral simulator; expected
   bytes may supplement but never replace execution assertions.
4. A target-only intrinsic is rejected under the other target with an
   actionable diagnostic.
5. Z80 `u16` proofs inspect both bytes of the register pair, and 8086 memory
   proofs execute through segmented addressing.
6. The frontend refuses unsupported features instead of falling back to a
   different backend or silently widening a value.
7. Documentation distinguishes source compatibility from machine-code
   compatibility.

Gate-level execution is a later acceptance layer. It is not required to claim
the behavioral Word v1 pipeline, and simulator completeness alone is not
evidence that a compiler backend is complete.

## Non-goals of this design PR

This contract does not add a grammar, reserve file extensions, create frontend
packages, expand either backend, or declare any Word matrix cell supported. It
selects the shared semantics, target boundary, delivery order, and proof
requirements so implementation can proceed without inventing a second source
language or hiding the architectural differences.
