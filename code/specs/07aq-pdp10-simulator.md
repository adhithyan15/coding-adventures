# Spec 07aq — DEC PDP-10 KA10 Functional Simulator

**Architecture:** DEC PDP-10 KA10 user-visible integer core

**Year:** 1968

**Rust package:** `pdp10-simulator`

## Sources and scope

This cell follows Digital Equipment Corporation's KA10 documentation and the
DECsystem-10 Assembly Language Handbook. The model is intentionally a KA10
integer/control oracle, not a synthetic union of later KI10, KL10, KS10, TOPS-10,
or TOPS-20 behavior.

Primary references:

- <https://www.bitsavers.org/pdf/dec/pdp10/KA10/KA10ProcMaintMan.pdf>
- <https://www.bitsavers.org/www.computer.museum.uq.edu.au/pdf/DEC-10-NRZC-D%20decsystem-10%20Assemblly%20Language%20Handbook.pdf>

The first bounded cell implements the common KA10 integer data, Boolean,
comparison, skip, jump, stack, and signed multiply/divide surface described
below. Floating point, byte pointers, block transfer, execute, privileged APR,
and device I/O instructions return typed unsupported-instruction errors. Paging
and extended addressing are not KA10 features and are not invented.

## Word, address, and transport contract

PDP-10 words are 36 bits and addresses are 18 bits. Public word values use a
`u64` whose upper 28 bits must be zero; public addresses use a `u32` whose upper
14 bits must be zero. Arithmetic stores the low 36 bits and interprets bit 35 as
the two's-complement sign.

Canonical program transport stores every word in five big-endian bytes. The
high nibble of the first byte must be zero. A byte stream whose length is not a
multiple of five, a nonzero transport padding nibble, or a program that does not
fit at its requested origin fails before any state changes.

The machine owns 262,144 36-bit words. Locations 0 through 15 alias the sixteen
accumulators for normal processor memory references; installed program
transport still writes the owned memory image. The distinction is explicit in
snapshots so restoration cannot silently conflate an accumulator write with a
loader write.

## Architectural state

| State | Width / count |
|---|---:|
| Accumulators AC0–AC15 | 16 × 36 bits |
| Program counter | 18 bits |
| Main memory | 262,144 × 36-bit words |
| Arithmetic overflow | 1 bit |
| Carry 0 / Carry 1 | 1 bit each |
| Divide check | 1 bit |
| Halt | 1 bit |
| Installed origin / length | lifecycle metadata |

Snapshots are deep, owned values. Reset, load, restore, direct access, step,
bounded run, and execute expose no mutable internal storage.

## Instruction format and effective addresses

Every instruction word is decoded as:

```text
35             27 26       23 22 21       18 17                 0
+----------------+-----------+--+-----------+--------------------+
|  opcode (9)    |   AC (4)  | I| index (4)|     address (18)   |
+----------------+-----------+--+-----------+--------------------+
```

Effective-address calculation adds the selected accumulator's low 18 bits to
the address when the index field is nonzero. If the indirect bit is set, the
addressed word supplies another indirect/index/address tuple. Indirection is
bounded to 64 links; an unterminated chain returns `IndirectAddressLimit`
atomically instead of hanging the host.

## Implemented instruction boundary

Opcodes are written in the architecture's conventional octal notation.

- Data movement: the `MOVE`, `MOVEI`, `MOVEM`, `MOVES`, `MOVN`, and `MOVM`
  families, including accumulator, immediate, memory, and both forms where the
  handbook defines them.
- Fixed-point arithmetic: `ADD`, `ADDI`, `ADDM`, `ADDB`, `SUB`, `SUBI`,
  `SUBM`, `SUBB`, `IMUL`, `IMULI`, `IMULM`, `IMULB`, `IDIV`, `IDIVI`,
  `IDIVM`, and `IDIVB`. Overflow, both carry flags, and divide check are
  deterministic architectural outputs. Divide-by-zero and quotient overflow
  are typed atomic failures.
- Boolean operations: accumulator/immediate/memory/both variants of `AND`,
  `IOR`, `XOR`, and `EQV`, plus `SETZ`, `SETZM`, `SETO`, and `SETOM`.
- Comparison and control: all signed conditions for `CAI`, `CAM`, `JUMP`,
  `AOJ`, and `SOJ`; `AOBJN`, `AOBJP`, `JRST`, `PUSHJ`, `PUSH`, `POP`, and
  `POPJ`. The documented `JRST` halt form sets the halt latch.

Condition suffixes use the KA10 signed comparison result: never, less, equal,
less-or-equal, always, greater-or-equal, not-equal, and greater. A skipped
instruction advances the PC by one additional word. Stack pointers use a
36-bit accumulator with an 18-bit count in the left half and an 18-bit address
in the right half; push and pop update both halves before the memory transfer.

All other opcode classes fail as `UnsupportedOpcode { opcode, pc }`. A malformed
encoding inside an implemented family fails as `InvalidInstruction`.

## Lifecycle and atomicity

- `reset` restores power-on state and clears memory and installed-program
  metadata.
- `load_at` validates the whole transport and range before replacing state.
- `restore` validates every word, address, flag, memory length, and installed
  range before commit.
- `step` rejects post-halt execution and fetch outside the installed program.
- Every failed fetch, effective-address chain, memory access, arithmetic
  operation, stack operation, decode, or final snapshot leaves the complete
  pre-step state unchanged.
- `run(max_steps)` and `execute(program, origin, max_steps)` require a nonzero
  bound and roll the complete machine back on any failure.

Successful traces contain PC before/after, the exact 36-bit instruction word,
resolved effective address when applicable, and a stable uppercase mnemonic.
Execution results contain all traces and the complete final owned state.

## Required evidence

Tests must cover transport round-trips and rejection, all address/index/
indirect boundaries, accumulator-memory aliasing, every implemented opcode
form and condition selector, signed 36-bit arithmetic edges, multiply/divide
limits, stack half updates, halt, post-halt rejection, installed-range fetches,
restore validation, bounded execution, and whole-state rollback. Formatting,
strict all-target Clippy, rustdoc with warnings denied, affected BUILD targets,
and greater than 80% line coverage are mandatory before publication.
