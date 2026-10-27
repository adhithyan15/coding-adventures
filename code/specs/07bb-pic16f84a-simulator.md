# Layer 07bb — PIC16F84A Classic Mid-Range Functional Simulator

## 1. Status, source authority, and pinned profile

This specification defines a checked instruction-level model of the classic
14-bit mid-range PIC CPU in the **PIC16F84A**. It is a CPU oracle for one exact
device profile, not a generic model of every PIC16 part and not a complete
PIC16F84A microcontroller.

**Status:** bounded functional implementation complete locally; independent
exact-head implementation review passed at
`a6cc743ae6c25bc2923d340988c38c79caf9af41`; publication remains pending.
The exact reviewed spec-only head was
`c7da48941de5ab6bb7d0949bd4ba081c018f5d3b`.

The public primary sources are:

- Microchip, *PIC16F84A Data Sheet*, revision C, document DS35007C, especially
  sections 2, 6, and 7,
  <https://ww1.microchip.com/downloads/aemDocuments/documents/MCU08/ProductDocuments/DataSheets/35007C.pdf>.
- Microchip, *PICmicro Mid-Range MCU Family Reference Manual*, document
  DS33023A (section files identify themselves as DS31006A and DS31029A),
  especially sections 6 and 29,
  <https://ww1.microchip.com/downloads/en/DeviceDoc/33023a.pdf>.

DS35007C is normative where the family manual permits configuration choices.
It pins 1,024 physically implemented 14-bit program words, 68 bytes of GPR
SRAM, a 13-bit PC, one 8-bit FSR, and an eight-entry 13-bit return stack.
DS33023A supplies the detailed instruction and PC/stack rules. The sources
publicly and completely describe the selected CPU boundary; no proprietary
silicon, RTL, or peripheral implementation is inferred.

The immutable package profile is `Pic16f84aClassicMidRangeV1`:

| Property | Pinned value |
|---|---|
| CPU | classic 14-bit mid-range core in PIC16F84A |
| Program transport | 1,024 owned `u16` words, each constrained to 14 bits |
| Architectural PC | 13-bit word address; physical fetch aliases through low 10 bits |
| Data transport | 8-bit file-register values with PIC16F84A banking and mirrors |
| Working state | W, STATUS, FSR, PCLATH, INTCON, 68 GPR bytes |
| Return stack | eight circular 13-bit entries; pointer is hidden from software |
| Instructions | the 35 instructions in DS35007C table 7-2 |
| Byte order | not applicable between program words; each image element is one 14-bit word |

The instruction-word API deliberately uses `u16` rather than inventing a byte
packing for the 14-bit Harvard bus. Data remains byte-wide. Program and data
are different address spaces and no file-register instruction can modify the
program image.

## 2. Complete authoritative state and deterministic reset

`Pic16f84aState` is an owned, frozen value containing exactly:

- `pc: u16`, constrained to `0x0000..=0x1fff`;
- `w: u8`;
- `status: u8`, canonicalized as described below;
- `fsr: u8`;
- `pclath: u8`, constrained to bits 4:0;
- `intcon: u8`;
- `gpr: [u8; 68]`, the physical SRAM at file addresses 0x0c–0x4f;
- `return_stack: [u16; 8]`, each entry constrained to 13 bits;
- `stack_pointer: u8`, constrained to 0–7 and not exposed as a file register;
- `program: [u16; 1024]`, each word constrained to 14 bits;
- installed-image `origin: Option<u16>` and `length: u16`, in physical program
  words and constrained to one nonwrapping range within 0x000–0x3ff;
- `sleeping: bool`; and
- `halted: bool`.

There is no shadow PCL byte: reading PCL derives it from the authoritative PC,
and writing PCL updates PC. There is likewise no duplicate STATUS, FSR,
PCLATH, INTCON, stack, or GPR state. Snapshot equality covers every listed
field, including program storage, the hidden stack pointer, and image metadata.

Silicon power-on values marked `x` by DS35007C are intentionally made
deterministic by this teaching model. `new` and `reset` zero W, FSR, GPR,
INTCON, the stack entries, stack pointer, program words, and image metadata;
set PC and PCLATH to zero; set STATUS to `0x18` (`TO=1`, `PD=1`, all other
implemented bits clear); and clear sleep and halt. This is a simulator
convention, not a claim that hardware's documented unknown values are zero.

STATUS has the PIC16F84A layout `00 RP0 TO PD Z DC C`:

- bits 7 (`IRP`) and 6 (`RP1`) are unimplemented, read zero, and must remain
  zero;
- bit 5 `RP0` selects bank 0 or 1 for direct file addressing;
- bits 4 `TO` and 3 `PD` are read-only to ordinary file writes;
- bits 2 `Z`, 1 `DC`, and 0 `C` are the arithmetic flags.

PCLATH bits 7:5 are unimplemented and always read zero. INTCON is retained as
an eight-bit software-visible latch because `RETFIE` has the CPU-defined effect
of setting GIE. This cell performs no asynchronous flag setting, interrupt
recognition, or vector delivery.

`restore` validates the complete candidate before changing the machine. It
rejects a noncanonical PC, stack word, stack pointer, STATUS, PCLATH, program
word, or installed range. It permits PC to name any 13-bit architectural
address, including an alias outside the literal installed range, because fetch
uses the physical low-ten-bit alias. A halted or sleeping state may be
restored, but halt and sleep may not both be true.

## 3. Harvard program transport, installation, and fetch

Program addresses count 14-bit words. `load(words, origin)` accepts a nonempty
slice of `u16` words only when every word is at most `0x3fff`, `origin` is at
most `0x03ff`, and `origin + words.len()` fits without wrapping in the 1,024
physical words. Validation precedes mutation. Success performs the complete
deterministic reset, installs the words, records their exact physical range,
sets PC to origin, and leaves sleep and halt clear.

The architectural PC increments modulo 2^13. Physical PIC16F84A program
memory is only 1K words, so fetch index is exactly `pc & 0x03ff`, as illustrated
by DS35007C's 0x020/0x420/... aliases. That physical index must lie in the
installed half-open range. Otherwise step returns `FetchOutsideProgram`
without mutation. All 35 architectural instructions are one word; there is no
truncated multiword form.

Sequential execution first computes the 13-bit fall-through PC
`(pc_before + 1) & 0x1fff`. During execution, a PCL read observes the low byte
of that fall-through value, matching the prefetch pipeline and computed-GOTO
examples. A successful non-control instruction commits that fall-through PC.
It may leave a physical index outside the installed image; the next step then
faults. No fetch reads uninstalled zero-filled storage.

Any explicit PC write flushes the prefetched instruction. The exact trace
records this as a two-cycle architectural operation, but this functional model
does not simulate oscillator phases or elapsed time.

## 4. File-register addressing and the CPU/peripheral boundary

A direct seven-bit `f` operand forms the eight-bit file address
`(STATUS.RP0 << 7) | f`. Indirect access through file address 0x00 or 0x80
uses the complete eight-bit FSR as the effective file address. PIC16F84A does
not implement IRP. If FSR names either INDF alias (0x00 or 0x80), an indirect
read returns zero and an indirect write has no storage effect; instruction
flag effects still occur.

The exact file map is:

| Address | Meaning in this CPU cell |
|---|---|
| 0x00 / 0x80 | INDF synthetic indirect portal |
| 0x02 / 0x82 | mirrored PCL view of PC bits 7:0 |
| 0x03 / 0x83 | mirrored STATUS |
| 0x04 / 0x84 | mirrored FSR |
| 0x0a / 0x8a | mirrored PCLATH |
| 0x0b / 0x8b | mirrored INTCON |
| 0x0c–0x4f / 0x8c–0xcf | the same 68 physical GPR bytes |
| 0x07, 0x50–0x7f, 0x87, 0xd0–0xff | unimplemented: reads zero, writes ignored |
| all remaining named SFRs | excluded peripheral boundary described below |

TMR0, OPTION_REG, PORTA, PORTB, TRISA, TRISB, EEDATA, EEADR, EECON1, and
EECON2 are not ordinary RAM. An instruction that actually reads or writes any
of those addresses returns `UnsupportedPeripheralRegister { address }`
atomically. This is more precise than fabricating latch behavior: real PORT
reads depend on pins, TMR0 changes autonomously, and EEPROM control has side
effects. It also leaves a clean seam for the later PIC16F84A SoC composition.
Address calculation alone does not fault; only an architecturally required
read or write crosses this boundary.

Writing PCL installs result bits 7:0 and PCLATH bits 4:0 into PC bits 12:8,
then marks a pipeline flush. PCLATH itself is not updated from PC. Writing
STATUS applies all of these rules in one merged write:

1. unimplemented bits 7:6 remain zero and TO/PD preserve their old values;
2. RP0 takes result bit 5;
3. for an instruction that affects Z, DC, or C, each affected flag comes from
   the ALU and not the file-write result; and
4. unaffected flag bits take the file-write result.

Thus operations such as `CLRF STATUS` and `ADDWF STATUS,F` implement the
device data sheet's write inhibition rather than using an ordinary byte store.
FSR and INTCON accept all eight result bits; PCLATH accepts only bits 4:0.

Operand reads, ALU work, destination writes, PC effects, and status changes are
calculated in disposable candidate state and commit once. RP0 used for the one
file operand is sampled before a write to STATUS can change it. Read-modify-
write instructions perform one source read and one destination write; no
second host read may observe a changed bank or peripheral boundary.

## 5. Exhaustive decoder contract

Every fetched `u16` is first checked to be 14-bit. The decoder then uses the
DS35007C table 7-2 patterns below. `x` bits are true don't-cares and every raw
combination they denote is the same instruction, rather than an unknown alias.

| Family | 14-bit pattern |
|---|---|
| `ADDWF`, `ANDWF`, `COMF`, `DECF` | `00 0111 dfff ffff`, `00 0101 dfff ffff`, `00 1001 dfff ffff`, `00 0011 dfff ffff` |
| `DECFSZ`, `INCF`, `INCFSZ`, `IORWF` | `00 1011 dfff ffff`, `00 1010 dfff ffff`, `00 1111 dfff ffff`, `00 0100 dfff ffff` |
| `MOVF`, `RLF`, `RRF`, `SUBWF` | `00 1000 dfff ffff`, `00 1101 dfff ffff`, `00 1100 dfff ffff`, `00 0010 dfff ffff` |
| `SWAPF`, `XORWF` | `00 1110 dfff ffff`, `00 0110 dfff ffff` |
| `CLRF`, `CLRW`, `MOVWF`, `NOP` | `00 0001 1fff ffff`, `00 0001 0xxx xxxx`, `00 0000 1fff ffff`, `00 0000 0xx0 0000` |
| `BCF`, `BSF`, `BTFSC`, `BTFSS` | `01 00bb bfff ffff`, `01 01bb bfff ffff`, `01 10bb bfff ffff`, `01 11bb bfff ffff` |
| `ADDLW`, `ANDLW`, `IORLW`, `MOVLW` | `11 111x kkkk kkkk`, `11 1001 kkkk kkkk`, `11 1000 kkkk kkkk`, `11 00xx kkkk kkkk` |
| `SUBLW`, `XORLW`, `RETLW` | `11 110x kkkk kkkk`, `11 1010 kkkk kkkk`, `11 01xx kkkk kkkk` |
| `CALL`, `GOTO` | `10 0kkk kkkk kkkk`, `10 1kkk kkkk kkkk` |
| `RETURN`, `RETFIE`, `SLEEP`, `CLRWDT` | `00 0000 0000 1000`, `00 0000 0000 1001`, `00 0000 0110 0011`, `00 0000 0110 0100` |

These patterns classify 16,008 of the 16,384 possible 14-bit words as one of
the 35 architectural instructions. The compatibility-only `OPTION` word
0x0062 and `TRIS` words 0x0065–0x0067 are documented by DS33023A but are not in
the device's 35-instruction table and must return
`UnsupportedLegacyInstruction` atomically. Exact reserved word 0x0001 is the
repository halt sentinel described below. The remaining 371 words return
`UnknownInstruction`.

An exhaustive test iterates all 16,384 words and compares the production
decoder with an independent mask/value table. It also asserts the counts above,
every mnemonic family, every don't-care alias, the four legacy words, the halt
word, and all 371 unknown words. No production match arm, mnemonic parser, or
assembler helper may serve as the test oracle.

## 6. Byte-oriented arithmetic and file operations

The 18 byte-oriented families are implemented for every legal raw encoding:

- `ADDWF` computes `f + W`; `SUBWF` computes `f - W`. Both write W when `d=0`
  or the resolved file destination when `d=1`, set Z from the byte result, set
  C on carry/no borrow, and set DC on low-nibble carry/no borrow.
- `ANDWF`, `IORWF`, and `XORWF` write the named Boolean result and set only Z.
- `COMF`, `DECF`, and `INCF` write their wrapping byte result and set only Z.
- `DECFSZ` and `INCFSZ` write the wrapping result, preserve every STATUS flag,
  and skip the next program word exactly when that result is zero.
- `MOVF` copies f to the selected destination and sets only Z. `MOVWF` copies W
  to f and changes no flags.
- `CLRF` writes zero to f and sets Z. `CLRW` writes zero to W and sets Z. Their
  don't-care encodings have no hidden file operand.
- `RLF` shifts f left through C, writes old bit 7 to C, and preserves Z/DC.
  `RRF` shifts f right through C, writes old bit 0 to C, and preserves Z/DC.
- `SWAPF` swaps the two nibbles and changes no flags.
- `NOP` changes only PC and trace state.

All subtraction uses the PIC no-borrow polarity documented for C and DC. C=1
means no unsigned borrow and DC=1 means no borrow from bit 4. Flag computation
uses the source operands, not a potentially masked STATUS destination write.

A file destination that resolves to PCL applies the PC-write rule after the
ALU result and any flags are known. In particular `ADDWF PCL,F` adds W to the
fall-through PCL byte, combines that result with PCLATH bits 4:0, and flushes
the pipeline. A file source naming PCL likewise reads fall-through PCL.

## 7. Bit, literal, skip, and control operations

`BCF` and `BSF` clear or set the selected bit after a single file read and
preserve STATUS except where STATUS itself is the destination under its write
mask. `BTFSC` and `BTFSS` read the selected bit, do not write the file or
flags, and skip respectively when clear or set.

`ADDLW` computes `W + k`; `SUBLW` computes `k - W`; both set C, DC, and Z under
the same carry/no-borrow rules as the file forms. `ANDLW`, `IORLW`, and `XORLW`
write W and set only Z. `MOVLW` and its four don't-care opcode aliases write W
and preserve every flag.

A taken `DECFSZ`, `INCFSZ`, `BTFSC`, or `BTFSS` advances candidate PC by one
additional 13-bit program word and records `skip_taken=true`,
`pipeline_discarded=true`, and `cycles=2`. It does not decode or semantically
execute the discarded word. The discarded physical address need not be in the
installed host image; a subsequent step at the post-skip PC performs the normal
fetch check. A false skip records one cycle and no pipeline discard.

There is one orthogonal overlap to pin explicitly. When `DECFSZ` or `INCFSZ`
uses `d=1` and resolves f to PCL, the Q4 destination write is the authoritative
PC load described by DS33023A section 29.3.2. A zero result still records that
the skip condition was true, but it does not increment the newly loaded PCL
target a second time: the PCL write and true skip discard the same prefetched
word and the instruction remains one two-cycle flush. A nonzero result is also
two cycles because the PCL write flushes the pipeline. This ordering must have
dedicated result-zero/result-nonzero tests across PCLATH boundaries.

`GOTO k` loads PC bits 10:0 from k and bits 12:11 from PCLATH bits 4:3.
`CALL k` first pushes the complete fall-through 13-bit PC, then forms the same
target. Although PIC16F84A physically implements only 1K words and therefore
does not require software paging, retaining the documented architectural page
bits is observable in PC/snapshots; physical fetch still aliases through low
10 bits. Both instructions are two-cycle pipeline flushes.

`RETURN`, `RETLW`, and `RETFIE` pop a complete 13-bit PC and flush the pipeline.
`RETLW` also writes its literal to W. `RETFIE` also sets INTCON.GIE. The cell
permits software-authored RETFIE even though asynchronous interrupt entry is
outside scope.

## 8. Return-stack behavior

The stack is the device's inaccessible circular eight-entry hardware stack,
not a checked software stack. Push stores at `stack_pointer` and advances the
pointer modulo eight. Pop decrements the pointer modulo eight and reads that
entry. The ninth push therefore overwrites the first and the tenth overwrites
the second exactly as DS35007C states.

The hardware exposes no stack pointer and no overflow/underflow status. This
model consequently does not invent overflow or underflow faults or a hidden
depth counter. A pop without a preceding push reads the deterministically
initialized circular slot. Complete snapshots still include the hidden pointer
and all entries so restore, rollback, wraparound, and POP-after-overwrite are
reproducible.

## 9. SLEEP, CLRWDT, and deterministic halt

All 35 documented instructions remain represented even though watchdog and
clock behavior are excluded:

- `CLRWDT` performs only its CPU-visible effects, setting STATUS.TO and
  STATUS.PD. Watchdog and prescaler state do not exist in this cell.
- `SLEEP` sets STATUS.TO, clears STATUS.PD, sets `sleeping`, and completes one
  successful trace. A later step returns `Sleeping` without mutation. External
  wake-up, time-out, clock stopping, and interrupt entry are SoC behavior.

Reserved word 0x0001 is intercepted as this repository's deterministic halt
sentinel. It is not a PIC instruction and is never named as part of the 35.
It emits one successful `HALT` trace, advances PC normally, and sets `halted`.
A later step returns `Halted` without mutation. Halt and sleep are distinct.

The sentinel choice is pinned by the exhaustive classification table: 0x0001
is neither a table-7-2 instruction nor OPTION/TRIS. It therefore cannot steal a
documented encoding or a don't-care alias.

## 10. Trace, lifecycle, and typed atomic faults

Every successful step returns a `Pic16f84aStepTrace` containing:

- architectural `pc_before` and `pc_after`;
- physical `fetch_index = pc_before & 0x03ff`;
- exact fetched 14-bit `raw_word`;
- stable uppercase mnemonic, including `HALT` for the repository sentinel;
- `cycles` equal to one or two;
- `pipeline_discarded`, `skip_taken`, and `branch_taken` booleans;
- optional file effective address and whether it was direct or indirect; and
- the complete post-state.

Trace fields are captured from pre-mutation transport and candidate semantics;
they are never reconstructed by rereading committed state. File addresses are
reported even for unimplemented zero/ignored locations. A fault produces no
trace and no state change.

The public typed error set includes at least:

- `EmptyProgram`, `ProgramWordOutOfRange`, and `ImageOutOfRange`;
- `FetchOutsideProgram`;
- `UnknownInstruction` and `UnsupportedLegacyInstruction`;
- `UnsupportedPeripheralRegister`;
- `InvalidState` with a field-specific reason;
- `Sleeping` and `Halted`;
- `TraceLimitExceeded`; and
- a wrapper-preserving `ExecutionFailed { step, pc, source }` for transactional
  `run`/`execute` failures.

`step` clones or otherwise stages the complete candidate before any fetch,
decode, file read, stack mutation, or write. Every error leaves all state and
trace output byte-for-byte unchanged. `run(max_steps)` is transactional as a
whole and preflights capacity for at most 64 retained complete-state traces.
`execute(words, origin, max_steps)` is atomic across load and run: any load,
step, sleep-before-halt, trace-limit, or budget failure restores the caller's
complete entry state. A successful run terminates only after HALT; SLEEP is a
typed non-halt stop unless a later SoC supplies a wake event.

## 11. Required independent validation

Implementation may begin only after independent spec review. Completion then
requires all of the following:

1. exhaustive independent classification of all 16,384 raw words, including
   exact family counts, don't-care aliases, the four legacy words, halt, and
   all unknowns;
2. all 128 direct f values in both RP0 banks, all 256 FSR values, every mirror,
   all 68 shared GPR bytes, INDF self-reference, unimplemented zero/ignore
   locations, and every excluded peripheral address;
3. all 256-byte arithmetic inputs plus carry boundaries where applicable,
   with exhaustive C/DC/Z oracles for add/subtract and rotate boundaries;
4. every STATUS-as-destination family, proving TO/PD preservation,
   unimplemented-bit masking, and ALU-owned flag precedence;
5. PCL reads and every PCL-writing family across all 32 PCLATH values,
   fall-through low-byte wrap, computed-GOTO carry, and physical 1K aliasing;
6. CALL/GOTO targets across all 11-bit k values and all PCLATH page bits;
7. stack push/pop sequences at depths 0, 1, 7, 8, 9, 10, and multiple complete
   wraps, including RETURN/RETLW/RETFIE distinctions;
8. true and false boundaries for all four skip families at PC low-byte,
   physical 1K, and architectural 8K wrap points;
9. CLRWDT, SLEEP, HALT, post-sleep/post-halt, terminal fetch, restore, malformed
   state, and load/run/execute rollback witnesses;
10. exact trace assertions for ordinary, indirect, skip, computed-GOTO, call,
    return, sleep, and halt steps; and
11. package-scoped formatting, release tests, strict Clippy, rustdoc, affected
    BUILD/doctest validation, lessons validation, and greater than 80% line
    coverage over all package targets using the repository's standard coverage
    command and scope.

Test oracles must not call the production decoder, file resolver, ALU helper,
stack helper, or trace formatter. Boundary matrices and exhaustive loops use
independently written equations or literal mask/value tables.

## 12. Explicit exclusions and follow-up cells

This first cell excludes interrupt recognition/vector entry, Timer0 counting,
prescaler behavior, watchdog timing/reset, oscillator and instruction timing,
SLEEP wake-up, electrical pins and port sampling, EEPROM storage/control,
in-circuit programming, code protection, configuration words, debug, and every
peripheral side effect. It also excludes enhanced mid-range, PIC16F1, PIC18,
and other PIC-family instructions and state.

Two source-honest follow-ups are queued:

1. an **educational PIC16F84A gate model** composed from repository gates and
   stable-bit storage, differential against this functional oracle. It must be
   labeled educational unless a published implementation topology is found;
   and
2. a **PIC16F84A SoC composition** adding address-decoded Timer0, ports,
   EEPROM, watchdog/reset, interrupt routing, pin samples, clock/reset domains,
   and deterministic bus/event traces around this CPU.

Neither follow-up may claim transistor/electrical timing. Descent below gates
waits for the separate SPICE/device-model work and source-backed lower-level
models.
