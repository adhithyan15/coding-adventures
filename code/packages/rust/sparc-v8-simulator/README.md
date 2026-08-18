# SPARC V8 Simulator (Rust)

Behavioral simulator for the SPARC V8 (1987) — the first **open** RISC
instruction-set standard, designed by Sun Microsystems and later
powering Sun SPARCstation workstations and Solaris servers for two
decades.  Rust port of `code/packages/python/sparc-v8-simulator`
(Layer 07r); see
[`code/specs/07r-sparc-v8-simulator.md`](../../../specs/07r-sparc-v8-simulator.md)
(if present) or the Python package's own docstrings for the full ISA
writeup.

## Supported Instructions

- **Format 1**: `call`
- **Format 2**: `sethi`; `Bicc` (`ba`/`bn`/`bne`/`be`/`bg`/`ble`/`bge`/
  `bl`/`bgu`/`bleu`/`bcc`/`bcs`/`bpos`/`bneg`/`bvc`/`bvs`); `nop`
- **Format 3 ALU**: `add`/`addcc`/`addx`/`addxcc`, `sub`/`subcc`/`subx`/
  `subxcc`, `and`/`andcc`/`andn`/`andncc`, `or`/`orcc`/`orn`/`orncc`,
  `xor`/`xorcc`/`xnor`/`xnorcc`, `sll`/`srl`/`sra`, `umul`/`umulcc`,
  `smul`/`smulcc`, `udiv`/`udivcc`, `sdiv`/`sdivcc`, `mulscc`, `rdy`/
  `wry`, `jmpl`, `save`/`restore`, `ticc` (including `ta 0` -- HALT)
- **Format 3 memory**: `ld`/`ldub`/`lduh`/`ldsb`/`ldsh`, `st`/`stb`/
  `sth`
- **Halt**: `ta 0` (trap-always, software trap #0) -- this simulator's
  (and the Python original's) HALT sentinel

## Architecture

```
opcodes.rs   -- op / op2 / op3 field constant tables (Formats 1/2/3)
encoding.rs  -- encode_* helpers to construct machine code words
decode.rs    -- instruction decoder for all four instruction shapes
registers.rs -- windowed register file (8 globals + NWINDOWS x 16) + CWP
execute.rs   -- instruction executor + big-endian memory accessors
simulator.rs -- top-level SparcV8Simulator with fetch-decode-execute
```

## What makes SPARC V8 different from every other simulator in this series

- **Overlapping register windows.**  32 logical registers (`%g0-%g7`
  globals, `%o0-%o7` outs, `%l0-%l7` locals, `%i0-%i7` ins) map onto a
  56-register physical file (8 globals + 3 windows x 16) via a Current
  Window Pointer.  `SAVE` rotates CWP backward (procedure entry);
  `RESTORE` rotates it forward (procedure exit).  The *ins* of window
  `W` alias the *outs* of window `(W+1) % NWINDOWS` -- this is what
  makes argument passing across a call boundary free.  See
  `registers.rs` for the full `virt_to_phys` derivation, cross-checked
  bit-for-bit against `sparc-v8-gatelevel::register_file::virt_to_phys`.
- **A condition-code register**, not compare-into-GPR.  Unlike MIPS
  R2000 (`SLT`/`SLTU` write `0`/`1` to a GPR), SPARC has traditional
  PSR N/Z/V/C flags that `*cc`-suffixed ALU ops update and `Bicc`
  branches consume.
- **Big-endian memory**, same as MIPS R2000 (unlike RISC-V/ARM/x86).
  `cpu_simulator::Memory`'s `read_word`/`write_word` are little-endian,
  so `execute.rs` builds its own big-endian word/halfword accessors.
- **Branch/CALL displacement is relative to the instruction's own PC**
  (`pc + disp*4`), not `pc + 4 + disp*4` (MIPS's delay-slot-shaped
  convention) -- see the `execute.rs` module docs for the derivation.
- **`ta 0` (HALT) still advances PC by 4** before halting -- unlike
  `mips-r2000-simulator`'s `SYSCALL`, which reports the unchanged PC.
  This follows directly from the Python original's fetch-then-dispatch
  pipeline; see `execute.rs` module docs.
- **No branch-delay slots.**  Matches the Python original's explicit
  simplification -- branches/calls/JMPL take effect immediately.
- **Fail-closed halting instead of exceptions.**  The Python simulator
  raises `ValueError` on `UDIV`/`SDIV` by zero, register-window
  overflow, and non-`TA` `Ticc` traps.  This port has no exception
  channel through `step() -> String`, so those cases halt the
  simulator instead (destination register / `Y` / CWP left unwritten)
  -- the same fail-closed pattern `mips-r2000-simulator` uses for
  signed-overflow `ADD`/`ADDI`/`SUB` and `DIV`/`DIVU` by zero.

## Register-window scoping note

This crate ports the **full** register-window machinery -- nothing is
stubbed here.  `sparc-v8-backend` (one layer up, the `Backend`-trait
implementation) is the crate that scopes its v0.1.0 CIR lowering to
globals-only (`%g0`/`%o0`), since the minimal-viable `const_*`/`ret_*`
program never needs `SAVE`/`RESTORE`.  See that crate's docs.

## Usage

```rust
use sparc_v8_simulator::SparcV8Simulator;
use sparc_v8_simulator::encoding::*;

let mut sim = SparcV8Simulator::new(65536);
sim.run_instructions(&[
    encode_add_imm(8, 0, 1),   // %o0 = %g0 + 1
    encode_add_imm(9, 0, 2),   // %o1 = %g0 + 2
    encode_add(10, 8, 9),      // %o2 = %o0 + %o1
    encode_ta(0),               // ta 0 -- halt
]);
assert_eq!(sim.regs.read(10), 3);
```
