# Layer 07ba — Classic MSP430 Functional Simulator

## 1. Status, source authority, and pinned profile

This specification defines a checked instruction-level model of the original
16-bit MSP430 CPU, pinned to the CPU implemented by Texas Instruments'
**MSP430F2274**. It is intentionally not an MSP430X, MSP430Xv2, or complete
MSP430F2274 microcontroller model.

**Status:** independently reviewed specification; implementation authorized.
The exact reviewed spec-only head was
`4e750ba3abfc7e3a2dc794c7fc2aefe81bc0554f`.

The public primary sources are:

- Texas Instruments, *MSP430F2xx, MSP430G2xx Family User's Guide*, revision K,
  August 2022, document SLAU144K, especially chapter 1.4 and chapter 3,
  <https://www.ti.com/lit/ug/slau144k/slau144k.pdf>.
- Texas Instruments, *MSP430F22x2, MSP430F22x4 Mixed Signal Microcontroller
  Data Sheet*, revision G, August 2012, document SLAS504G, especially the CPU
  description and memory-organization table,
  <https://www.ti.com/lit/ds/symlink/msp430f2274.pdf>.

SLAU144K chapter 3 is normative for encodings and CPU semantics. SLAS504G pins
the configuration identity: the MSP430F2274 uses the classic 16-bit MSP430 CPU,
16 registers, a 16-bit address space, and not the later 20-bit MSP430X CPU.
The data sheet's 32 KiB Flash, 1 KiB RAM, information Flash, boot ROM, and
memory-mapped peripherals identify the physical device but do not become
fabricated behavior in this CPU-only cell.

The immutable package profile is `Msp430F2274ClassicCpuV1`:

| Property | Pinned value |
|---|---|
| CPU | classic 16-bit MSP430 as used by MSP430F2274 |
| Address width | 16 bits |
| Registers | sixteen 16-bit registers, R0–R15 |
| Byte order | little-endian instruction and data transport |
| Owned storage | one unified 65,536-byte teaching memory |
| Instruction formats | classic Format I, II, and III only |
| Extensions | no MSP430X/MSP430Xv2 address instructions or 20-bit state |

Using a flat owned byte array is a deliberate functional-oracle boundary. It
preserves the classic CPU's unified von-Neumann address semantics without
claiming Flash erase rules, ROM immutability, peripheral register behavior, or
the exact MSP430F2274 SoC decode map.

## 2. Complete authoritative state

`Msp430State` is a frozen, owned value containing exactly:

- `registers: [u16; 16]` with the special behavior below;
- all 65,536 memory bytes;
- `halted: bool`;
- installed-image `origin: Option<u16>`; and
- installed-image `length: u32`, permitting the full 65,536-byte image.

No flags, PC, SP, or constant-generator values are duplicated in hidden host
fields. R0 is the sole PC authority, R1 the sole SP authority, and R2 the sole
SR authority. R3 has no writable storage effect: it is the constant generator,
and writes to a register-mode R3 destination are discarded, as used by the
manual's `MOV #0,R3` NOP encoding. Snapshot equality covers every register,
memory byte, halt bit, and image-metadata field.

The simulator's deterministic `new` and `reset` convention zeroes all
registers and memory, clears halt, and removes the installed image. The manual
requires software to initialize SP, so reset choosing SP=0 is a simulator
convention rather than a silicon-reset claim. R0 and R1 are always even;
architectural writes clear bit 0. A restored odd PC or SP is malformed rather
than silently canonicalized. The stored R3 slot is canonically zero, and
`restore` rejects any nonzero R3 rather than accepting inaccessible state.

R2 exposes only the documented SR bits:

- bit 8 `V`, bits 7–4 `SCG1`, `SCG0`, `OSCOFF`, and `CPUOFF`;
- bit 3 `GIE`; and
- bits 2–0 `N`, `Z`, and `C`.

Bits 15–9 are reserved and always read zero. Instruction writes and `RETI`
mask them to zero; `restore` requires the canonical zero form. The clock,
power, and interrupt-delivery effects of the mode bits are excluded. In this
cell they are restorable architectural bits only: `CPUOFF` does not set
`halted`, and `GIE` does not cause asynchronous entry.

## 3. Memory, transport, installation, and fetch

Memory is byte addressed and little-endian. A word's low byte is at the even
address and its high byte at the following odd address. Byte operands may use
any address. Instruction words, extension words, and data words require even
addresses, matching SLAU144K sections 1.4.5 and 3.2.1. An odd word address is a
typed `MisalignedAccess` error; a word may not wrap from 0xFFFF to 0x0000.

`load(program, origin)` accepts a nonempty, even-length byte image at an even
origin only when the entire half-open range fits without wrapping in 64 KiB.
It validates first, then performs a complete reset, copies the image, sets R0
to origin, records the exact origin/length, and leaves halt clear. Empty,
odd-length, odd-origin, oversized, or overflowing images fail without changing
the prior machine.

Fetch requires R0 to be even and inside the exact installed half-open image.
Every required instruction or extension word must be wholly installed. A
missing first word is `FetchOutsideProgram`; a missing required byte or
extension word after a valid first word is `TruncatedInstruction`. Fetch never
wraps, reads an uninstalled zero, or borrows adjacent memory merely because it
exists in the owned array.

The first word determines the format and exact extension count:

- Format I double-operand words are opcodes 0x4–0xF. Source indexed,
  symbolic, absolute, and immediate forms consume their source extension
  first; an indexed/symbolic/absolute destination consumes the next extension.
  Constant-generator forms consume no extension.
- Format II single-operand families occupy their exact 0x1000–0x12BF masks,
  and use one extension for the same indexed/symbolic/absolute/immediate
  operand forms. `RETI` is exactly 0x1300.
- Format III jumps occupy 0x2000–0x3FFF and are always one word.
- exact word 0x0000 is this repository's deterministic halt sentinel. Other
  0x0001–0x0FFF words and all reserved Format II neighbors are unknown.

All two, four, or six instruction bytes are copied into the trace before any
operand read or write. Self-modifying instructions therefore report the bytes
that were actually fetched, not bytes reread after the store.

## 4. Registers, constant generation, and operand sequencing

R4–R15 are ordinary general-purpose registers. Register byte writes replace
the low byte and clear the high byte, as required by SLAU144K section 3.2.5.
Word writes replace all 16 bits. Special-register canonicalization then masks
PC/SP bit 0, SR reserved bits, or discards an R3 result.

The source `As` field implements all documented classic modes:

| Register / `As` | Meaning |
|---|---|
| ordinary Rn / 00 | register |
| ordinary Rn / 01 | signed 16-bit extension plus Rn |
| ordinary Rn / 10 | indirect `@Rn` |
| ordinary Rn / 11 | indirect autoincrement `@Rn+` |
| R0 / 01 | symbolic, signed extension plus the extension word's address |
| R0 / 10 | indirect PC |
| R0 / 11 | immediate extension word |
| R2 / 00 | SR register, word form only |
| R2 / 01 | absolute address from the extension word |
| R2 / 10, 11 | constants 4 and 8 |
| R3 / 00, 01, 10, 11 | constants 0, 1, 2, and 0xFFFF |

The destination `Ad` field implements register and indexed modes. R0 indexed
is symbolic and uses its own extension-word address as the PC base; R2 indexed
is absolute. Indexed R3 uses its architectural zero value as the base.
Indirect and autoincrement are source-only encodings. Constants are truncated
to 8 bits for byte operations. Because SLAU144K permits SR register mode only
for word instructions, a byte instruction naming R2 as a register-mode source
or destination returns `UnpredictableOperand` atomically.

Extension offsets are signed 16-bit additions modulo 2^16. Ordinary register
autoincrement adds one for a byte and two for a word, modulo 2^16. Word access
alignment is checked after effective-address calculation. There is no hidden
host-sized address arithmetic.

Operand evaluation follows the manual-visible sequence even though all work is
transactional:

1. the candidate PC advances past the first word;
2. source extensions are consumed; the source is read; source
   autoincrement is applied to candidate state;
3. destination extensions are consumed; the destination effective address and
   old value are resolved using that candidate state;
4. semantics and flags are calculated; and
5. all register/memory/PC changes commit together.

This pins overlaps such as `ADDC @R5+,-2(R5)`, `MOV @SP+,SP`, and source/dest
register aliasing. A symbolic source is based on its source-extension address;
a symbolic destination is based on its later destination-extension address.
Register-mode PC reads observe the sequential candidate PC at the point that
source or destination is resolved, not a host-language post-state.

## 5. Double-operand instructions and flags

All twelve classic Format I opcode families are included in byte and word
forms: `MOV`, `ADD`, `ADDC`, `SUBC`, `SUB`, `CMP`, `DADD`, `BIT`, `BIC`, `BIS`,
`XOR`, and `AND`. Assembler-only aliases such as `ADC`, `BR`, `CLR`, `DEC`,
`POP`, and `TST` are tested through the core encoding they expand to; they are
not extra decoder families.

For the selected width, results wrap modulo 2^8 or 2^16:

- `MOV`, `BIC`, and `BIS` preserve V, N, Z, and C.
- `ADD` and `ADDC` set N/Z from the result, C from unsigned carry, and V from
  signed overflow.
- `SUB`, `SUBC`, and non-writing `CMP` set N/Z from `dst - src - borrow`, C on
  no borrow, and V from signed subtraction overflow. `SUBC` uses borrow
  `1-C`; `SUB` uses borrow zero.
- non-writing `BIT` and writing `AND` set N/Z from the AND result, C to `!Z`,
  and clear V.
- `XOR` sets N/Z from the result, C to `!Z`, and V when both input sign bits
  were one.
- `DADD` performs packed-BCD digit addition with incoming C, sets N/Z from the
  result and C on a final decimal carry. SLAU144K declares V undefined, so the
  named profile deterministically preserves pre-state V; documentation and
  traces must not present that policy as a silicon guarantee. If any source or
  destination nibble is not 0–9, the manual declares the result undefined and
  this checked model returns `UndefinedBcdOperand` atomically.

`CMP` and `BIT` resolve and read their destination but never write it. Every
instruction preserves all SR bits not listed above, including the clock-mode
and GIE bits. For an operation that writes R2 and also affects flags, nonflag
SR bits come from the width-canonicalized ALU result and V/N/Z/C come from that
instruction's flag calculation; this single merged word is the only final SR
write. `MOV`, `BIC`, and `BIS` instead install their complete masked SR result
because they do not affect flags.

## 6. Single-operand instructions, stack, call, and return

The included Format II families are `RRC[.B]`, `SWPB`, `RRA[.B]`, `SXT`,
`PUSH[.B]`, `CALL`, and exact `RETI`:

- `RRC` shifts old C into the width's sign position and the old LSB into C;
  it sets N/Z and clears V.
- `RRA` retains the old sign bit, shifts the old LSB into C, sets N/Z, and
  clears V.
- `SWPB` swaps the bytes of a word and preserves SR.
- `SXT` sign-extends bit 7 through bits 15, sets N/Z, sets C to `!Z`, and
  clears V.
- `PUSH` decrements SP by two and stores the source. `PUSH.B` still decrements
  by two but writes only the low byte at the new SP, leaving the adjacent high
  byte unchanged. The manual's special `PUSH SP` case stores the already
  decremented SP value.
- `CALL` evaluates and saves the word destination into a temporary before
  stack mutation, decrements SP by two, stores the complete fall-through PC,
  then loads the target into PC. This order pins `CALL SP` and `CALL @SP+`.
- `RETI` preflights both stack words, restores masked SR from `@SP`, advances
  SP by two, restores the even-canonicalized PC from the next word, and
  advances SP by two again. The complete four-byte operation is atomic.

`CALL`, `SWPB`, `SXT`, and `RETI` are word-only. Encodings with their reserved
B/W bit set are `UnknownInstruction`. The manual permits immediate operands
for `PUSH` and `CALL` but declares immediate `RRC`, `RRA`, `SWPB`, and `SXT`
unpredictable; those exact forms return `UnpredictableOperand` atomically.

Stack arithmetic is 16-bit modulo arithmetic. Each word transfer must still be
even and individually valid. Because the model owns the complete 16-bit
address space, a multi-transfer stack operation may move from 0xFFFE to 0x0000
between complete word accesses, but no individual word access wraps through
0xFFFF.

## 7. Jumps, PC behavior, and halt

The eight Format III conditions are included exactly:

- `JNE/JNZ`: Z=0; `JEQ/JZ`: Z=1;
- `JNC/JLO`: C=0; `JC/JHS`: C=1;
- `JN`: N=1; `JGE`: N xor V = 0; `JL`: N xor V = 1; and
- `JMP`: always.

The signed 10-bit field is multiplied by two and added to `PCold + 2`, giving
the manual's -511 through +512 word range. Arithmetic wraps in 16 bits and the
target remains even. Jumps preserve every SR bit.

Any instruction destination that writes PC changes control flow after all
operand effects have been resolved. Architectural PC writes clear bit 0. The
step may successfully leave PC outside the installed image; the next step then
returns `FetchOutsideProgram`. This separates CPU addressing from the
repository's image boundary and allows complete terminal states to round-trip.

Exact reserved word 0x0000 is the repository-only `HALT` sentinel. It advances
PC by two, sets halt, and emits one successful trace. It is not represented as
a TI instruction or as `CPUOFF`. A subsequent step returns `Halted` without
mutation.

## 8. Typed lifecycle, traces, and atomicity

The public checked API provides `new`, `reset`, `load`, `snapshot`, `restore`,
byte/word memory helpers, register helpers, `step`, `run(max_steps)`, and
`execute(program, origin, max_steps)`. `restore` validates exact memory and
register lengths, canonical SR/PC/SP, image origin/length/evenness/range, and
the unloaded invariant `origin=None,length=0` before committing. It accepts an
even terminal PC outside the installed image and a halted terminal state.

Every successful step trace contains:

- PC before and after;
- exact original fetched bytes in address order and raw word(s);
- width in bytes (2, 4, or 6);
- stable uppercase core mnemonic and operand-width marker;
- source and destination addressing classifications;
- taken/not-taken for jumps; and
- the complete post-state.

Traces name the encoded core operation, not an assembler alias. `run` and
`execute` are transactional as a whole. They preflight a bounded maximum of 64
retained complete-state traces so allocation or trace-limit failure cannot
leave partial architectural progress.

Stable typed errors distinguish at least: malformed state, empty/malformed or
out-of-range program, no installed program, fetch outside program, truncated
instruction, misaligned instruction/data, unknown instruction, unpredictable
operand, undefined BCD operand, halted execution, trace limit, exhausted step
budget, and allocation failure. Errors carry the relevant PC/address/raw
word/reason and have nonempty stable display text.

Fault priority is: halted; installed-image/PC fetch validation; complete
instruction fetch; exact decode and fixed-bit validation; unpredictable or
undefined operand classification; complete candidate-state data/stack/control
preflight; allocation/trace retention; commit. A failed public operation
preserves every register, SR bit, memory byte, halt bit, and image-metadata
field. Candidate autoincrement, extension fetch, flag updates, stack writes,
and PC changes are never externally visible on error.

## 9. Explicit exclusions and follow-up cells

This cell excludes interrupt acceptance, vector fetch, reset-vector behavior,
NMI, peripheral and SFR behavior, watchdog, Flash/ROM write restrictions and
erase/program protocols, DMA, GPIO, ADC, timers, serial interfaces, clock
sources, low-power entry/wake, CPUOFF stopping behavior, cycle counts,
pipeline/bus timing, debug/JTAG/Spy-Bi-Wire, bootstrap loader, electrical
behavior, package pins, and the rest of the MSP430F2274 SoC.

It also excludes every MSP430X/MSP430Xv2 feature: 20-bit registers and address
space, extension words, `.A` forms, `MOVA`, `CALLA`, `RETA`, `PUSHM`/`POPM`,
multi-bit rotates, address instructions, and extended exceptions. No opcode
from those families may be guessed or treated as a classic instruction.

After the functional oracle is accepted, an educational gate cell may build a
source-auditable 16-bit register file, ALU/BCD path, decoder, address
generation, PC/SP/SR storage, and byte-lane memory interface and prove full-
state differentials against this model. It must be labelled educational unless
a public transistor/RTL/netlist authority supports a stronger claim.

A later MSP430F2274 SoC cell may compose the CPU with source-backed address
decode, Flash/RAM/boot regions, clocks/reset, interrupt routing, DMA, timers,
GPIO, serial peripherals, and deterministic bus/transaction traces. Those
components must move values through explicit buses and arbitration rather than
becoming hidden CPU callbacks. Transistor and analog timing descent waits for
the repository's SPICE/device-model layer.

## 10. Completion gate for the future implementation

The functional cell is complete only when it has all of the following:

1. exhaustive first-word classification over all 65,536 values, exact fixed-
   bit and width checks, every included core family, reserved neighbors, the
   0x0000 halt sentinel, and explicit non-classic/MSP430X rejection;
2. a generated matrix spanning every legal Format I source `As`/register,
   destination `Ad`/register, byte/word width, extension count/order, constant-
   generator value, PC-relative extension base, and representative alias;
3. Format II matrices for every legal addressing mode, byte/word restriction,
   immediate-unpredictable form, stack/PC/SP alias order, and exact RETI;
4. exhaustive jump conditions over N/Z/C/V and signed displacement boundaries
   -512, -511, -1, 0, +1, +511 (encoded word offsets), including 16-bit wrap;
5. independent 8- and 16-bit flag oracles for arithmetic/logic/shift, carry and
   signed-overflow boundaries, unaffected SR bits, valid packed-BCD digits,
   invalid-BCD atomic faults, and the named DADD-V preservation policy;
6. byte/word register and memory witnesses, every alignment boundary,
   autoincrement and overlapping register case, symbolic-source/destination
   base distinction, complete 2/4/6-byte fetch, and self-modifying trace bytes;
7. complete-state reset/load/snapshot/restore/halt/run/execute witnesses,
   malformed empty/loaded states, terminal PCs, trace and budget bounds,
   allocation seams, and rollback comparisons for every typed fault family;
8. release line coverage above 80 percent for the package; and
9. package-scoped Rustfmt, strict Clippy, rustdoc, release tests, BUILD and
   affected-plan integration, lessons validation, latest-main ancestry, clean
   diff checks, and mandatory independent exact-head review before publication.
