# Zilog Z80 Simulator (Rust)

Behavioral simulator for the Zilog Z80 (1976) — one of the most widely
produced microprocessors ever, powering the TRS-80, ZX Spectrum, MSX, the
original Game Boy (via a variant core), and countless CP/M machines. Rust
port of `code/packages/python/z80-simulator`; see
[`code/specs/z80-encoder.md`](../../../specs/z80-encoder.md) /
[`code/specs/z80-backend.md`](../../../specs/z80-backend.md) for the
encoder/backend writeups this crate feeds. Seventh lane of the
9-architecture expansion documented in
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).

## The Z80 is an Intel 8080 superset

Every valid 8080 opcode is a valid Z80 opcode with **identical** semantics
and **identical** byte encoding — Zilog designed the Z80 for source (and
largely binary) compatibility with 8080 software. This crate's base
instruction set is therefore a direct structural port of
`intel8080-simulator`, renamed to Zilog's assembler mnemonics (`LD`
instead of `MOV`/`MVI`/`LXI`/`STA`/`LDA`/…, `JP` instead of `JMP`, `CP`
instead of `CMP`, `RLCA`/`RRCA`/`RLA`/`RRA` instead of `RLC`/`RRC`/`RAL`/
`RAR`, `CPL`/`SCF`/`CCF` instead of `CMA`/`STC`/`CMC`). See
`code/specs/z80-encoder.md` for the full byte-identity table between the
two encoders.

## Supported instructions

**Base (8080-compatible) set** — byte-identical to `intel8080-simulator`:
- Data transfer: `LD r,r'`, `LD r,n`, `LD rp,nn`, `LD (nn),A`/`LD A,(nn)`,
  `LD (nn),HL`/`LD HL,(nn)`, `LD (BC),A`/`LD (DE),A`/`LD A,(BC)`/
  `LD A,(DE)`, `EX DE,HL`
- Arithmetic: `ADD`, `ADC`, `SUB`, `SBC`, `INC r`/`DEC r`, `INC rp`/
  `DEC rp`, `ADD HL,rp`, `DAA`
- Logical: `AND`, `XOR`, `OR`, `CP`, `RLCA`/`RRCA`/`RLA`/`RRA`, `CPL`,
  `SCF`, `CCF`
- Branch: `JP` + 8 conditional, `CALL` + 8 conditional, `RET` + 8
  conditional, `RST 0`–`7`, `JP (HL)`
- Stack: `PUSH`/`POP` (including `PUSH AF`/`POP AF`), `EX (SP),HL`,
  `LD SP,HL`
- I/O: `IN A,(n)`, `OUT (n),A` (256 ports each direction)
- Control: `NOP`, `HALT`, `EI`, `DI`

**Z80-only additions**:
- Alternate register bank: `EX AF,AF'`, `EXX`
- Relative jumps: `DJNZ e`, `JR e` + 4 conditional forms
- `CB`-prefix: `RLC`/`RRC`/`RL`/`RR`/`SLA`/`SRA`/`SLL`(undocumented)/`SRL`
  and `BIT`/`RES`/`SET` against any of the 8 `r`-coded operands
- `DD`/`FD`-prefix (IX/IY basics only): `LD IX,nn`/`LD IY,nn`,
  `INC IX`/`INC IY`

**Deliberately NOT ported** (v0.1.0 scope cut — see `decode.rs` module
docs for the rationale): the entire `ED`-prefix opcode space — 16-bit
`ADC HL,rp`/`SBC HL,rp`, the block-transfer/compare/I-O instruction
families (`LDIR`/`LDDR`/`CPIR`/`CPDR`/`INIR`/`INDR`/`OTIR`/`OTDR` and
their non-repeating siblings), `LD A,I`/`LD A,R`/`LD I,A`/`LD R,A`, `NEG`,
`RETN`/`RETI`, and interrupt-mode selection. Every `ED`-prefixed opcode
decodes to `"undefined"`, which the executor treats as a fail-closed
halt — never silently executing garbage. Full displacement addressing
through IX/IY (`LD r,(IX+d)`, ALU-via-`(IX+d)`, `LD (IX+d),n`) is also
not ported — only the unconditional `LD IX/IY,nn` and `INC IX/IY` forms
the "IX/IY basics" scope calls for.

## Architecture

```
opcodes.rs   -- opcode / register / condition-code constant tables
encoding.rs  -- encode_* helpers to construct machine code byte sequences
decode.rs    -- variable-length instruction decoder (1-4 bytes)
execute.rs   -- instruction executor + register/flag state
simulator.rs -- top-level Z80Simulator with fetch-decode-execute
```

## What differs from `intel8080-simulator`

- **An extra flag.** The Z80 F register carries `N` (add/subtract),
  needed for correct `DAA` behaviour after both `ADD`- and `SUB`-family
  ops; `P/V` is dual-purpose (parity after logical ops, signed overflow
  after arithmetic ops) rather than 8080's parity-only `P`.
- **An alternate register bank** stored as raw byte values (`Registers`
  has `a2`/`f2`/`b2`/…) rather than unpacked flags — it's opaque to every
  instruction except `EX AF,AF'`/`EXX`.
- **Fail-closed halt on `ED`-prefixed and undefined opcodes** — no
  exception channel through `step() -> String`, matching the fail-closed
  convention every simulator in this workspace uses.

## Usage

```rust
use z80_simulator::Z80Simulator;
use z80_simulator::encoding::*;
use z80_simulator::opcodes::*;

let mut sim = Z80Simulator::new(65536);
sim.run_instructions(&[
    encode_ld_r_n(REG_B, 1),
    encode_ld_a_n(2),
    vec![encode_alu_reg(ALU_ADD, REG_B)],
    vec![HALT],
]);
assert_eq!(sim.regs.a, 3);
```
