# Spec 07n2 — Motorola 68000 Gate-Level Simulator

## Status and scope

Layer 07n2 is the Rust gate-level partner of the complete Layer 07n
`m68k-simulator`. It models the repository's Motorola 68000 execution surface:
the 24-bit big-endian machine, eight data and eight address registers, PC, SR,
all specified effective-address forms, and every instruction family dispatched
by Spec 07n.

The implementation is `code/packages/rust/motorola68k-gatelevel`. The Python
package remains the independent behavioral oracle that generates the committed
82-vector JSONL corpus; the Rust functional simulator consumes that corpus and
is the typed lifecycle oracle shared by this gate implementation.

## Persistent topology

Every mutable architectural bit has a D-flip-flop identity:

| State | Width |
|---|---:|
| 16 MiB memory | 134,217,728 bits |
| D0–D7 | 256 bits |
| A0–A7 | 256 bits |
| PC | 32 bits |
| SR | 16 bits |
| Halt latch | 1 bit |
| **Total** | **134,218,289 D flip-flops** |

At a completed master/slave clock edge, the four internal latch outputs are
fully determined by Q: `(q, !q, q, !q)`. Memory stores these stable Q values in
packed bytes, reconstructs the full transient latch state for each write, and
clocks both phases through `logic-gates::sequential::register`. This is an exact,
lossless boundary representation without expanding 16 MiB into a 512 MiB host
object. Register, PC, SR, and halt banks retain explicit `FlipFlopState` values.

## Datapath gate contract

- ADD, ADDX, SUB, SUBX, NEG, NEGX, and address arithmetic use full-adder chains.
- AND, OR, XOR, NOT, flag reduction, and condition predicates use logic gates.
- Shifts and rotates use fixed bit routing with gate-computed flag outputs.
- MULU/MULS use a fixed 16×16 partial-product network. Each multiplier bit gates
  one 16-bit row, and 32-bit ripple adders accumulate the rows.
- DIVU/DIVS use a fixed 32÷16 restoring network. Each stage shifts the partial
  remainder, subtracts the divisor with a gate adder, and selects the quotient
  bit from the no-borrow output.

Host arithmetic may appear in tests as an independent expected-value oracle; it
must not implement these production datapaths.

## Architectural state

Reset establishes:

- 16 MiB of zeroed big-endian memory;
- PC `0x001000`;
- supervisor stack A7 `0x00F000`;
- SR `0x2700`;
- D0–D7 and A0–A6 zero;
- halt false.

Addresses are masked to 24 bits. Byte accesses are unaligned. Checked word and
long instruction/data transitions follow the functional oracle's alignment and
failure rules. The four bytes of a long value appear most-significant first.

## Effective addresses

The gate CPU supports the complete Spec 07n EA matrix:

| Mode | Form |
|---:|---|
| `000` | Dn |
| `001` | An |
| `010` | (An) |
| `011` | (An)+ |
| `100` | -(An) |
| `101` | d16(An) |
| `110` | d8(An,Xn.W/L) |
| `111/000` | absolute word |
| `111/001` | absolute long |
| `111/010` | d16(PC) |
| `111/011` | d8(PC,Xn.W/L) |
| `111/100` | immediate |

A7 byte pre-decrement/post-increment uses two bytes. MOVEA.W and ADDA/SUBA word
sources sign-extend to 32 bits. PC-relative bases follow the extension-word
position defined by the functional oracle.

## Instruction surface

The implemented decoder covers lines 0–9 and B–E:

- immediate OR/AND/SUB/ADD/EOR/CMP and immediate/register bit operations;
- MOVE.B/W/L, MOVEA, MOVEQ, SR/CCR transfers;
- CLR, NEG, NEGX, NOT, TST, SWAP, EXT, PEA, LEA;
- NOP, RESET, STOP, TRAP, LINK, UNLK, JSR, JMP, RTS, RTR;
- ADDQ, SUBQ, Scc, DBcc, BRA, BSR, and all Bcc predicates;
- OR, AND, EOR, ADD/ADDA/ADDX, SUB/SUBA/SUBX, CMP/CMPA/CMPM;
- MULU, MULS, DIVU, DIVS, EXG;
- register and memory ASL/ASR, LSL/LSR, ROXL/ROXR, ROL/ROR.

`TRAP #15` is the repository halt convention. Other TRAP values retain the
functional oracle's lightweight D7 behavior. STOP loads SR and halts.

## Checked lifecycle

`Cpu68K` shares `M68kState`, `StepTrace`, `ExecutionResult`, and `M68kError` with
the functional Rust simulator.

- `get_state()` owns every register, flag, halt bit, and all 16 MiB of memory.
- `restore()` validates memory length and the 24-bit PC before mutation.
- `load_checked()` and `load_at_checked()` reject overflowing programs before
  reset or memory writes.
- `step_checked()` rejects halted or misaligned entry, validates decode and
  execution against the functional oracle, and returns complete before/after
  states.
- `run_loaded_checked()` and `run_checked()` restore their entry state on any
  failure.

Legacy `load`, `step`, and `execute` remain available for the original crate
surface. Checked APIs are the normative integration boundary.

## Conformance

Completion requires:

1. exact topology/latch tests;
2. typed atomic load, restore, step, and run tests;
3. all existing gate component and instruction tests;
4. the 82 committed Python-oracle full-state vectors across every addressing,
   decode, flag-edge, multiply/divide, and shift family;
5. at least 80% core line coverage;
6. clean `cargo fmt`, Clippy with `-D warnings`, and rustdoc with `-D warnings`.
