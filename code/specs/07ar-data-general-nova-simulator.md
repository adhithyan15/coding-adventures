# Spec 07ar — Data General Nova Functional Simulator

**Architecture:** Data General Nova / Nova 1200 common 16-bit CPU core

**Year:** 1969

**Rust package:** `data-general-nova-simulator`

## Sources and scope

This cell implements the documented common Nova integer/control architecture.
It deliberately excludes later Nova 3, Nova 4, Eclipse, third-party 64-Kword,
stack, byte, memory-map, floating-point, and multiply/divide extensions.

Primary references:

- Data General, *How to Use the Nova Computers*, April 1971:
  <https://bitsavers.org/pdf/dg/DG_NM-5_How_To_Use_The_Nova_Computers_Apr1971.pdf>
- Data General, *Nova Line Computers Programmer's Reference*, 1978:
  <https://docs.alexrp.com/nova/nova.pdf>
- Data General, *Nova Computers System Reference Manual*, revision 07,
  February 1974:
  <https://archive.computerhistory.org/resources/access/text/2024/06/102776244-05-0001-acc.pdf>

SIMH's independently maintained Nova decoder is a secondary executable
cross-check for field layouts, auto-index timing, and the CPU-device HALT
encoding, not the architectural authority:
<https://github.com/simh/simh/blob/master/NOVA/nova_cpu.c>.

## Word, memory, and transport contract

Words and accumulators are 16 bits. The program counter and effective addresses
are 15 bits. The machine owns exactly 32,768 words, matching the documented
Nova address space; absent-memory and third-party 64-Kword behavior are outside
this bounded model.

Canonical program transport is two bytes per word in big-endian order. Odd
byte counts, origins above octal `077777`, or an image that does not fit wholly
in memory fail before any state changes. Loading replaces the complete machine
state, records the installed origin and word length, and starts the PC at the
origin.

## Complete architectural state

| State | Width / count |
|---|---:|
| AC0–AC3 | 4 × 16 bits |
| Program counter | 15 bits |
| Carry | 1 bit |
| Main memory | 32,768 × 16-bit words |
| Halt | 1 bit |
| Installed origin / length | lifecycle metadata |

Snapshots own all memory and metadata. Reset, load, restore, direct access,
step, bounded run, and execute expose no mutable internal storage.

## Memory-reference instructions

The memory-reference format is:

```text
0  1  2  3  4  5  6  7  8                         15
+--+-----+-----+--+-----+-----------------------------+
|0 | op  | AC  | I| mode|       displacement (8)      |
+--+-----+-----+--+-----+-----------------------------+
```

The common instruction set implements `JMP`, `JSR`, `ISZ`, `DSZ`, all four
`LDA` accumulator selections, and all four `STA` selections. `JSR` saves the
already-incremented return PC in AC3. `ISZ` and `DSZ` wrap at 16 bits and skip
one additional word exactly when the stored result is zero.

The four direct addressing modes are page-zero, signed PC-relative using the
already-incremented next-instruction PC, signed AC2-relative, and signed
AC3-relative. The indirect bit applies the same first address calculation and
then follows memory words whose high bit requests another level. Locations
octal `00020`–`00027` increment before supplying their
address; locations `00030`–`00037` decrement before supplying it. Whether an
additional level is required is taken from the auto-index word before its
increment/decrement, as documented. The model bounds an indirect chain at
65,536 links and reports a typed atomic fault instead of hanging the host.

## Operate instructions

Every operate-field combination is decoded from:

```text
0  1  2  3  4  5  6  7  8  9 10 11 12 13 14 15
+--+-----+-----+--------+-----+-----+--+--------+
|1 | src | dst |  ALU   |shift|carry|NL|  skip  |
+--+-----+-----+--------+-----+-----+--+--------+
```

- ALU selectors implement complement, negate, move, increment, add
  complement, subtract, add, and AND.
- Carry input selectors retain, clear, set, or complement carry before the ALU.
- Shift selectors perform none, left-through-carry by one, right-through-carry
  by one, or byte swap. The shifted-out bit becomes the candidate carry.
- The no-load bit suppresses both destination-accumulator and carry updates but
  still evaluates the result and skip predicate.
- Skip selectors cover no skip, unconditional skip, carry zero/nonzero, result
  zero/nonzero, and the two documented combined carry/result predicates.

Traces use stable Nova mnemonics assembled from ALU, carry, shift, no-load, and
skip fields so every legal microprogrammed combination remains distinguishable.

## CPU control and unsupported I/O

The exact common CPU-device `HALT` word, octal `063077` (`DOC 0,CPU` with no
pulse), advances the PC normally and then sets the halt latch. All other I/O
transfer encodings fail with a typed `UnsupportedIo` error in this first cell.
Interrupt state, front-panel switches, I/O reset, priority masking, devices,
DMA, and timing belong to later SoC and peripheral cells.

## Lifecycle and atomicity

- `reset` clears registers, carry, memory, halt, and installed-program metadata.
- `load_at` validates the entire image and range before replacing state.
- `restore` validates PC, memory length, installed range, and all public fields
  before commit.
- `step` rejects post-halt execution and fetch outside the installed image.
- Failed fetch, address resolution, auto-index chain, decode, I/O, memory,
  restore, or lifecycle operations leave the complete prior state unchanged.
- `run(max_steps)` and `execute(program, origin, max_steps)` require a nonzero
  bound and roll the complete machine back on any failure.

Successful step traces contain PC before/after, exact raw word, resolved
effective address when applicable, and mnemonic. Execution results contain all
traces and a deep complete final state.

## Required evidence

Tests must cover transport and load boundaries; all memory-reference opcodes,
accumulator selections, direct modes, signed displacements, multi-level
indirection, pre-update auto-index/decrement behavior, and chain limits; every
ALU/carry/shift/no-load/skip selector and important cross-products; JSR return,
ISZ/DSZ wrapping, HALT and post-halt rejection; restore validation, installed
fetch range, bounded execution, and whole-state rollback. Formatting, strict
all-target Clippy, rustdoc with warnings denied, affected BUILD targets, and
greater than 80% line coverage are mandatory before publication.
