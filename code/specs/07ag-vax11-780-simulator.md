# Layer 07ag — DEC VAX-11/780 Behavioral Simulator

## Purpose

This package models the integer programming surface of the 1977 DEC
VAX-11/780.  VAX is the bridge between the PDP-11 and later DEC Alpha: it keeps
byte-addressed little-endian memory while expanding the programmer-visible
machine to 32 bits and making addressing modes part of a general operand
specifier grammar.

The implementation is intentionally an architectural simulator, not a timing
model.  It is the functional oracle for a later gate-level cell.

Primary references:

- Digital Equipment Corporation, *VAX-11 Architecture Reference Manual*,
  EK-VAXAR-RM-001, revision 6.1 (May 1982).
- VMS Software, *VAX MACRO and Instruction Set Reference Manual*.

## Machine state

The checked Rust machine owns all observable state:

- sixteen 32-bit registers, where R12 is AP, R13 is FP, R14 is SP, and R15 is PC;
- the low four PSL condition bits N, Z, V, and C;
- exactly 64 KiB of byte-addressed little-endian memory;
- the halt latch; and
- installed-program origin and length.

Reset clears every register, flag, memory byte, and lifecycle field.  State
snapshots are owned values.  Restoring an invalid snapshot fails without
changing the machine.

The architectural address calculation is 32-bit wrapping.  The educational
memory arena is bounded: any final byte, word, or longword address outside the
64 KiB arena is a typed fault.  VAX permits unaligned data accesses, so the
simulator does too.

## Instruction encoding

An opcode is one byte unless the first opcode byte is FC through FF.  This cell
implements the one-byte integer core.  Operands follow the opcode as general
operand specifiers.  The high nibble selects the mode and the low nibble the
register.

| Mode | Name | Meaning |
|---:|---|---|
| 0–3 | short literal | unsigned literal 0 through 63 |
| 4 | indexed | next specifier's address plus register times operand width |
| 5 | register | register value or register destination |
| 6 | register deferred | memory at register |
| 7 | autodecrement | decrement register by width, then use memory |
| 8 | autoincrement | use memory, then increment register by width |
| 9 | autoincrement deferred | read a longword pointer, increment register by four |
| A/B | byte displacement / deferred | signed byte displacement from register |
| C/D | word displacement / deferred | signed word displacement from register |
| E/F | long displacement / deferred | signed long displacement from register |

Special PC forms are architectural, not pseudo-instructions:

- PC autoincrement is immediate data;
- PC autoincrement deferred is an absolute address; and
- PC displacement modes are PC-relative, based on PC after the displacement.

Short literals and immediate operands are read-only.  Address operands reject
register and literal forms that do not denote storage.  Destination validation
is performed before architectural state is committed.

## Implemented integer core

### Machine and control

| Opcode | Mnemonic | Semantics |
|---:|---|---|
| 00 | HALT | set the halt latch (kernel-mode sentinel) |
| 01 | NOP | advance PC only |
| 05 | RSB | pop a longword PC from SP |
| 10/30 | BSBB/BSBW | push return PC and branch by signed byte/word displacement |
| 11/31 | BRB/BRW | unconditional signed byte/word branch |
| 12–15, 18–1F | Bcond | branch using N/Z/V/C |
| 16 | JSB | push return PC and jump to an address operand |
| 17 | JMP | jump to an address operand |

The twelve conditions are BNEQ, BEQL, BGTR, BLEQ, BGEQ, BLSS, BGTRU, BLEQU,
BVC, BVS, BGEQU/BCC, and BLSSU/BCS.

### Byte, word, and longword families

The B, W, and L families are implemented uniformly:

- ADD2/ADD3, SUB2/SUB3, MUL2/MUL3, and DIV2/DIV3;
- BIS2/BIS3, BIC2/BIC3, and XOR2/XOR3;
- MNEG and CASE;
- MOV, CMP, MCOM, BIT, CLR, TST, INC, and DEC; and
- MOVA and PUSHA.

The family bases are 80/A0/C0 for arithmetic, 90/B0/D0 for unary/data, and
9E/3E/DE plus 9F/3F/DF for address operations.  PUSHL is DD.  MOVZBL (9A),
MOVZBW (9B), and MOVZWL (3C) are also implemented.

Byte and word register writes replace only the low 8 or 16 bits; longword
writes replace the full register.  Stack pushes decrement SP by four and store
a little-endian longword.

### Condition codes

- logical and move instructions set N and Z, clear V, and preserve C;
- TST clears V and C;
- ADD sets signed overflow and unsigned carry;
- SUB and CMP set C when an unsigned borrow occurs;
- MNEG sets C when the result is nonzero;
- divide by zero is a typed atomic fault; and
- instructions that do not define flags preserve them.

## Checked lifecycle

`VaxSimulator` provides checked load, state restore, direct memory/register
access, one-step execution, loaded-program execution, and transactional
one-shot execution.

Every fault is typed and atomic.  The error set distinguishes:

- invalid state or register;
- program out of range;
- fetch outside the installed program;
- truncated opcode, operand specifier, displacement, or immediate;
- memory or stack out of range;
- unknown or unsupported two-byte opcode;
- reserved, recursive, or illegal destination addressing mode;
- divide by zero;
- post-halt stepping; and
- exhausted step or trace limits.

`run_loaded_checked` rolls the entire run back on any fault, including step
limit exhaustion.  Traces retain scalar register/PSL snapshots plus raw
instruction bytes and mnemonic; memory is owned once by the final state.  A
fixed memory-derived trace budget prevents attacker-controlled retained-trace
growth.  Callers needing longer runs stream `step_checked` results.

## Deliberate exclusions

Floating point, packed decimal, character/string, queue, CRC, process context,
MMU, interrupt priority, and operating-system trap delivery are later cells.
Privileged HALT is accepted as a deterministic kernel-mode sentinel.  Checked
faults report conditions that real hardware would route through exception or
trap machinery.

## Completion contract

The cell is complete when:

1. the public API and every documented opcode family have focused tests;
2. all general addressing modes have read and writable-boundary coverage;
3. lifecycle and malformed-input failures prove atomicity;
4. release tests, strict Clippy, rustdoc warnings, formatting, BUILD, and
   repository lessons validation pass; and
5. Rust line coverage is at least 95 percent.
