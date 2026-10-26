# Layer 07ay — Classic AVR Functional Simulator

## 1. Status and source-pinned profile

This layer specifies a checked instruction-level model of a classic 8-bit AVR
core, pinned to the ATmega328P/`avr5` CPU profile. Its normative sources are
Microchip's *AVR Instruction Set Manual* and the ATmega48A/PA/88A/PA/168A/PA/
328/P data sheet. Those sources define instruction availability, encodings,
status flags, the word-addressed program counter, register/I/O/data mapping,
stack behavior, and the Harvard program/data split.

"Classic AVR" is a family, not one universal instruction inventory. Pinning
this cell to ATmega328P prevents instructions from tinyAVR, XMEGA, AVRxt, or
larger three-byte-PC devices from silently entering the model. This is a CPU
oracle with bounded opaque I/O storage, not a complete ATmega328P SoC.

## 2. Authoritative state and address spaces

The core owns a 32 KiB byte-addressed program image corresponding to the
device's 16 Kiword Flash and a 14-bit word-addressed PC. Instruction transport
stores each 16-bit opcode word low byte first; 32-bit instructions occupy two
successive words. Installed-program origin and length are byte quantities, but
both must be even and fetch translates PC to byte address `2 * PC` without
wrapping.

The core also owns the complete 0x0000-0x08FF data address space:

- 0x0000-0x001F are the authoritative bytes for R0-R31;
- 0x0020-0x005F are the 64 ordinary I/O registers;
- 0x0060-0x00FF are extended I/O registers; and
- 0x0100-0x08FF are the 2 KiB internal SRAM.

SREG at data address 0x005F and SPL/SPH at 0x005D/0x005E are authoritative
memory-mapped bytes, not shadow fields. Register, flag, and stack-pointer API
views derive from those bytes. SREG contains I, T, H, S, V, N, Z, and C. The
16-bit stack pointer is valid only in 0x0100-0x08FF. Reset selects the device's
RAMEND value 0x08FF. Stack operations preflight their entire transfer before
changing SP or memory.

The complete state includes PC, program image and metadata, the entire data
space, deterministic halt, and no hidden peripheral state. EEPROM is a third
physical address space on the device but is peripheral-controlled and excluded
from this CPU cell.

## 3. Included arithmetic and logic boundary

The admitted ATmega328P arithmetic surface is:

- `ADD`, `ADC`, `ADIW`, `SUB`, `SUBI`, `SBC`, `SBCI`, `SBIW`, `CP`, `CPC`, and
  `CPI`;
- `AND`, `ANDI`, `OR`, `ORI`, `EOR`, `COM`, `NEG`, `INC`, `DEC`, `TST`, `CLR`,
  and `SER`, treating assembler aliases according to their encoded operation;
- `LSL`, `LSR`, `ROL`, `ROR`, `ASR`, and `SWAP`, again decoding opcode aliases
  through the one architectural operation they encode; and
- `MUL`, `MULS`, `MULSU`, `FMUL`, `FMULS`, and `FMULSU`, which place the
  documented 16-bit result in R1:R0 and update only their documented flags.

All results and flags follow the Boolean formulas in the instruction manual,
including half carry, two's-complement overflow, `S = N xor V`, and the chained
zero behavior of `ADC`, `SBC`, `SBCI`, and `CPC`. Word arithmetic accepts only
the documented even register pairs R25:R24, R27:R26, R29:R28, and R31:R30.
Instructions preserve every flag not listed as affected by their manual entry.

## 4. Included data movement and memory boundary

The cell includes `MOV`, `MOVW`, and `LDI`; direct `LDS`/`STS`; all device-
supported `LD`/`ST` X, Y, and Z forms including pre-decrement, post-increment,
and Y/Z displacement; `PUSH` and `POP`; `IN` and `OUT`; and all device-supported
`LPM` forms.

X is R27:R26, Y is R29:R28, and Z is R31:R30, with little-endian low/high
register pairing. Pointer updates use 16-bit wrapping arithmetic but the actual
data access must be within 0x0000-0x08FF. Direct and indirect data accesses see
the same authoritative mapped bytes, so writes through a pointer can change
registers, SREG, SP, or opaque I/O exactly as writes to those addresses do.

The instruction manual marks the operand combinations in which an updating
pointer overlaps its load destination or store source as undefined. This
checked model does not invent an assignment order for them. It recognizes and
atomically rejects all of the following as `UnsupportedOperandCombination`:

- `LD R26,X+`, `LD R27,X+`, `LD R26,-X`, and `LD R27,-X`;
- `LD R28,Y+`, `LD R29,Y+`, `LD R28,-Y`, and `LD R29,-Y`;
- `LD R30,Z+`, `LD R31,Z+`, `LD R30,-Z`, and `LD R31,-Z`;
- `ST X+,R26`, `ST X+,R27`, `ST -X,R26`, and `ST -X,R27`;
- `ST Y+,R28`, `ST Y+,R29`, `ST -Y,R28`, and `ST -Y,R29`; and
- `ST Z+,R30`, `ST Z+,R31`, `ST -Z,R30`, and `ST -Z,R31`.

Non-updating indirect forms and displacement forms remain admitted when the
source or destination names a pointer register because they do not have this
manual-defined ambiguity.

`IN` and `OUT` address the 64 ordinary I/O locations and therefore map to data
addresses 0x0020-0x005F. `LDS`/`STS` and pointer forms may also reach extended
I/O and SRAM. Opaque I/O bytes have storage semantics only; timers, pins,
serial engines, ADC, interrupt flags, and write-one-to-clear behavior are not
fabricated by this CPU cell.

`LPM` reads program bytes using Z as the byte address, and Z+ updates the pair
only after validating the read. Program memory cannot be modified by ordinary
stores. `SPM` and boot-section programming are excluded. Every two-word
`LDS`/`STS` validates and fetches both opcode words before mutation.
The two undefined updating overlaps, `LPM R30,Z+` and `LPM R31,Z+`, return the
same atomic `UnsupportedOperandCombination` error; implied-destination `LPM`
and non-updating `LPM Rd,Z` remain admitted.

## 5. Bits, skips, and control flow

The included bit/status surface is `BSET`, `BCLR`, `BST`, `BLD`, `SBI`, `CBI`,
`SBIC`, `SBIS`, `SBRC`, and `SBRS`. `BSET`/`BCLR` and their named assembler
aliases address exactly one SREG bit. `SBI`/`CBI` and I/O skip forms accept only
their documented low I/O range.

The included compare/skip/branch surface is `CPSE`, all SREG-bit branches
encoded by `BRBS`/`BRBC` and their aliases, `RJMP`, `JMP`, `IJMP`, `RCALL`,
`CALL`, `ICALL`, `RET`, `RETI`, and `NOP`. Relative displacements are signed in
program words and targets are masked only to the device's 14-bit architectural
PC after validating that the target remains in the installed image.

A taken skip must first determine whether the following instruction is one or
two words from its first opcode word, then skip the complete instruction. The
skipped instruction is not decoded for semantic legality and cannot fault, but
its first word must lie in the installed image so its encoded width can be
classified. Truncated two-word successors fault atomically instead of becoming
one-word instructions.

`PUSH` and calls use the documented post-decrement stack: the first byte is
written at current SP and SP is then decremented. A two-byte return address is
the word address of the following instruction; its least-significant byte is
written first at the higher address, followed by the high byte, leaving SP two
lower. `POP` and returns perform the inverse pre-increment reads. Returns
reconstruct exactly the saved word address. `RETI` performs the same checked
pop and then sets SREG.I. Indirect targets use Z, and every direct, relative,
indirect, call, return, and skip target must select a complete installed
instruction before the next step.

## 6. Halt and excluded system instructions

The exact architectural `BREAK` encoding is intercepted as this repository's
deterministic halt sentinel. It emits one successful trace and sets halt; a
later step returns `Halted` without mutation. This is not a claim that BREAK is
a hardware halt on ATmega328P.

`SLEEP`, `WDR`, and `SPM` return typed `UnsupportedSystemInstruction` errors.
Real reset sequencing, interrupt recognition/priority/vector entry, watchdog
reset, sleep/wake, debugWire, and self-programming are excluded. `RETI` remains
admitted as a deterministic stack/status operation for software-supplied state,
even though this cell never enters an interrupt automatically.

Reserved opcodes and instructions unavailable on the pinned `avr5` device
return typed unknown or unsupported-in-profile errors. They never alias a
nearby classic, enhanced, tiny, XMEGA, or AVRxt operation.

## 7. Fetch, traces, and lifecycle

`new` and `reset` zero the program and data spaces, PC, SREG, and halt, set SP
to 0x08FF, and clear installed-image metadata. `load(program, byte_origin)`
accepts a nonempty even-length image at an even origin, requires the complete
image to fit the 32 KiB program space, zeroes all state as reset does, installs
the image, and sets PC to `byte_origin / 2`.

Fetch requires the first word and every required second word to be wholly
inside the exact installed image. Terminal fetches never wrap, read erased
Flash, or synthesize zeros. `restore` validates PC, installed byte range and
evenness, exact program/data lengths, SREG storage, SP, and halt before
committing.

Every successful step returns PC before and after, exact two or four fetched
bytes captured before any mutation, raw word(s), width, stable uppercase
mnemonic, skip/branch outcome, and a complete post-state. `step` is atomic.
`run(max_steps)` and `execute(program, origin, max_steps)` are transactional as
a whole and preflight a maximum of 64 retained complete-state traces.

Every instruction preflights decode/profile legality, full fetch width, all
source registers, pointer/writeback results, complete data/program ranges,
stack span, and control target before committing. A typed error preserves PC,
every program/data byte, all register and flag views, SP, halt, and image
metadata. Errors have stable nonempty display text and distinguish malformed
program/state, fetch outside image, truncated instruction, data/program/stack
range, invalid SP, unknown opcode, unsupported profile/system instruction,
unsupported operand combination, halted execution, trace limit, allocation
failure, and exhausted budget.

## 8. Explicit exclusions and follow-up backlog

This cell excludes peripherals, GPIO electrical behavior, timers, serial
interfaces, ADC/comparator, EEPROM protocol, interrupts and vectors, sleep and
watchdog behavior, boot locks and Flash programming, debugWire, fuses, clock
sources/prescalers, reset timing, cycle counts, pipeline timing, power, package
pins, and all board/SoC behavior. It also excludes three-byte PCs, RAMP/EIND,
ELPM, DES, XMEGA/AVRxt extensions, and tiny cores with reduced register files.

Follow-up cells must separately add a repository-gate implementation of this
exact pinned core, then compose it with source-backed AVR peripheral and bus
models. The README and changelog must call it an ATmega328P-compatible classic
AVR CPU subset, not a complete ATmega328P microcontroller.

## 9. Completion gate

The functional cell is complete only when it has:

1. classification witnesses for every admitted opcode family, every 32-bit
   width prefix, reserved neighbors, and instructions unavailable on `avr5`;
2. exhaustive flag formulas and boundaries, chained Z, every word-register
   pair, multiplier signed/fractional cases, and unaffected-flag preservation;
3. direct, pointer, displacement, pre/post-update, program-read, mapped-register,
   opaque-I/O, SP, stack, and complete-range rollback witnesses, plus one
   classification-and-complete-state rollback witness for every undefined
   X/Y/Z updating `LD`/`ST` overlap and both undefined `LPM Rd,Z+` overlaps;
4. taken/not-taken branches and every skip source over one- and two-word
   successors, calls/returns/RETI, PC wrapping boundaries, and target faults;
5. complete-state reset, load, restore, halt, run, execute, trace-bound,
   allocation, malformed/truncated image, exact fetched bytes, and
   transactional rollback tests;
6. release line coverage above 80 percent; and
7. scoped Rustfmt, strict Clippy, rustdoc, release tests, BUILD integration,
   lessons validation, latest-main ancestry, and independent exact-head review
   before publication.
