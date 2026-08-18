# Intel 8086 Simulator (Rust)

Behavioral simulator for the Intel 8086 (1978) — the 16-bit extension of
the 8080 architecture (NOT source- or binary-compatible with it, despite
the lineage) that introduced the segmented memory model and the ModRM
addressing byte. The IBM PC (1981) shipped with its cheaper 8-bit-bus
sibling, the 8088, founding the "PC-compatible" industry and making the
8086 the direct architectural ancestor of every x86 CPU made since. Rust
port of `code/packages/python/intel-8086-simulator` (Layer 07m); see
[`code/specs/07m-intel-8086-simulator.md`](../../../specs/07m-intel-8086-simulator.md)
for the full ISA writeup.

Ninth and **final** lane of the 9-architecture expansion documented in
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).

## Curated instruction subset (deliberately scoped — not the full ISA)

The Python reference implements essentially the full 8086 instruction
set in ~1670 lines. This crate ports a curated core instead:

- **Data transfer**: `MOV reg16,imm16` / `MOV reg8,imm8` (register-
  immediate), `MOV reg16,r/m16` (register-to-register only — ModRM
  `mod=11`).
- **Arithmetic/logical**: `ADD`/`SUB`/`AND`/`OR`/`XOR`/`CMP`, both as
  `AX,imm16` (accumulator-immediate) and `reg16,r/m16` (register-to-
  register, ModRM `mod=11` only).
- **Increment/decrement**: `INC reg16` / `DEC reg16` (preserve CF, per
  real 8086 semantics).
- **Halt**: `HLT` (`0xF4`) — a genuine single-byte hardware halt
  instruction, not a repurposed opcode or invented pseudo-halt.
- **Other**: `NOP` (`0x90`).

See `opcodes.rs`'s and `lib.rs`'s module docs for the full list of what's
**deferred**: memory-operand addressing (`[BX+SI]` and friends), segment-
override prefixes, string ops, stack ops, control flow, `MUL`/`DIV`,
shift/rotate, BCD adjust, I/O ports, and more.

## Segmented memory — the defining, structural feature

Every physical memory access goes through:

```text
physical_address = (segment_register << 4) + offset    (masked to 20 bits)
```

giving a 1 MiB address space built from 16-bit segment×offset pairs
across four segment registers (`CS`/`DS`/`SS`/`ES`). Instruction fetch
always uses `CS:IP`. This is **not deferrable** — even the trivial
`MOV AX,imm16; HLT` program this crate's `intel8086-backend` smoke test
compiles has its first opcode byte fetched through segmented addressing.
See `simulator.rs`'s module doc and `simulator::phys_addr` for the full
derivation.

## Architecture

```
opcodes.rs   -- curated opcode table (mnemonic, decode Format) +
                register-index constants + HLT_OPCODE
flags.rs     -- CF/PF/AF/ZF/SF/OF computation, ported from flags.py
decode.rs    -- fetch + operand decode (register-only ModRM -- memory
                operands are a decode error, not a silent misdecode)
execute.rs   -- instruction executor (methods over
                &mut Intel8086Simulator, mirroring mos6502-simulator's
                shape)
simulator.rs -- top-level Intel8086Simulator: segmented CS:IP fetch,
                registers, fetch-decode-execute loop, phys_addr()
encoding.rs  -- encode_* helpers (subset used by tests / intel8086-encoder)
```

## Usage

```rust
use intel8086_simulator::Intel8086Simulator;

let mut sim = Intel8086Simulator::new(65536);
sim.run(&[
    0xB8, 42, 0x00, // MOV AX, 42
    0xF4,           // HLT
]);
assert_eq!(sim.ax, 42);
assert!(sim.halted);
```

## Tests

87 tests total across this lane's three crates (61 in this crate alone)
covering: the opcode table, flag computation against the Python
reference's documented examples, decode of every supported instruction
shape (including the register-only ModRM rejection of memory operands),
execute-level behaviour (including CF preservation across `INC`/`DEC`),
the segmented physical-address formula itself (including its 20-bit
wraparound), and simulator-level integration — including the canonical
`MOV AX,42; HLT` "load immediate into the accumulator + halt" sequence
the `intel8086-backend` smoke test relies on.
