# Layer 07at — Motorola MC68020 Functional Simulator

## Status and bounded first cell

This specification defines the repository's first checked, instruction-level
Motorola MC68020 cell. The architectural sources are Motorola's *MC68020 /
MC68EC020 Microprocessors User's Manual* and *M68000 Family Programmer's
Reference Manual*. The cell deliberately models a coherent problem-state
subset rather than claiming the MC68020's entire ISA in one change.

The implemented boundary is:

- `MOVE.L` and `MOVEA.L` across the compatible data/address modes, including
  MC68020 scaled brief index words and full extension words;
- full-format base/index suppression, null/word/long base and outer
  displacements, and preindexed/postindexed memory indirection;
- byte, word, and MC68020 long `BRA`, `BSR`, and `Bcc` displacements;
- `RTD`, `LINK.L`, `EXTB.L`, and all three `TRAPcc` forms;
- the repository's `TRAP #15` halt convention; and
- a small inherited register-only compatibility surface (`NOP`, `MOVEQ`,
  `SWAP`, `EXT.W`, `EXT.L`, and data-register `ADDQ`/`SUBQ`) executed by the
  checked Layer 07n MC68000 core behind private translation.

The inherited bridge does not expose any Layer 07n type in this package's
public API. Extension opcodes are decoded before delegation, and differential
tests keep the inherited subset in complete-state lockstep.

### Retrospective specification audit

An unpublished draft described a broader MC68020 surface and was narrowed only
while implementation was being developed. Before publication, local history
was reconstructed without changing the resulting tree so that this corrected,
bounded contract is a spec-only commit preceding every implementation commit.
The boundary in this document is therefore the only implemented claim, and
every family named below remains backlog work.

The audit also makes three primary-manual constraints explicit for the completed
boundary:

- in a full extension word, `IS = 1` combined with `I/IS = 100` through `111`
  is reserved and must be rejected before consuming a base or outer
  displacement or performing any indirect-pointer access; the adjacent legal
  index-suppressed encodings remain valid; and
- trace bytes are the immutable instruction bytes fetched during decode.
  Destination writes that overlap the executing instruction may alter memory,
  but must not retroactively alter that step's `raw` bytes; and
- MC68020 data operands and memory-indirect pointer longwords may begin at odd
  addresses. Their complete byte ranges are checked before transfer. Only
  instruction and extension-word fetches retain the even-address requirement.

Bit fields, `CAS`/`CAS2`, `CHK2`/`CMP2`, long multiply/divide, `PACK`/`UNPK`,
`CALLM`/`RTM`, `MOVEC`, the remaining inherited MC68000 instruction families,
exception frames, cache/MMU/coprocessor behavior, bus timing, and interrupts
are follow-up cells. Recognized privileged or out-of-scope encodings return
typed errors rather than approximated effects.

## Complete architectural state

`Mc68020State` is an owned snapshot containing eight 32-bit data registers,
eight 32-bit address registers, 32-bit PC, 16-bit SR, USP/ISP/MSP, halt state,
every byte of the bounded storage window, and exact installed origin/length.
The active A7 bank is selected by SR.S/SR.M and must agree with its saved stack
pointer on restore.

The architectural address calculation is 32-bit and wrapping. This first cell
installs 16 MiB of storage; each actual data or indirect-pointer transfer must
fit that window. Byte, word, and longword data accesses may begin at any byte
address. Instruction and extension-word fetches must remain even.

## Checked lifecycle and atomicity

Programs are exact big-endian bytes. `load(program, origin)` rejects an empty
image, odd origin, arithmetic overflow, or an image outside storage before
mutation. Successful load clears state/storage, installs the whole image,
records its exact range, selects problem state, and sets PC to `origin`.

Fetch begins at an even PC and every extension word must remain in the
installed image. `step`, `run`, `execute`, restore, memory, and register
helpers are checked. A failed step restores complete state; a failed bounded
run/execute restores the complete entry state. Allocation failure is outside
the architectural fault model.

`TRAP #15`, not privileged `STOP`, is the deterministic repository halt, in
line with Layer 07n and its toolchain. A halted step returns `Halted` without
mutation. Problem-state `STOP` returns a typed unsupported/privileged fault.

## Effective addresses

`MOVE.L` sources accept data/address registers, address indirect,
postincrement, predecrement, displacement, indexed, absolute word/long,
PC-relative displacement/indexed, and immediate. Destinations accept data or
address registers and alterable memory modes. Address-register destination is
`MOVEA.L` and does not update condition codes.

A brief extension word uses an eight-bit displacement, word/long index value,
and scale 1, 2, 4, or 8. A full extension word implements Motorola's exact
base-suppress, index-suppress, base-displacement, and I/IS encodings. Reserved
encodings and the all-suppressed/no-displacement case are invalid. PC-relative
indexed modes use the address of their extension word as the PC base.

Preindexed indirect computes `([base + bd + index] + od)`; postindexed
computes `([base + bd] + index + od)`. Indirect pointers are big-endian
longwords and are fully checked before any address-register or memory update.
Postincrement/predecrement changes happen exactly once and roll back on fault.

## Instructions, flags, and traces

`MOVE.L` sets N/Z, clears V/C, and preserves X. `EXTB.L` sign-extends the low
byte, sets N/Z, clears V/C, and preserves X. Branch conditions use the full
MC68000 condition table. A branch low byte of zero selects a word extension;
`0xFF` selects a long extension; the displacement base is opcode PC + 2.
`BSR` pushes the address following all extension words.

`RTD` pops a big-endian return PC then adds the signed word displacement to A7.
`LINK.L` pushes the selected address register, copies A7 to it, and applies the
signed long displacement. `TRAPcc` accepts word, long, and no-operand forms: a
false condition only consumes the instruction, while a true condition returns
an atomic typed trap.

Every successful step reports PC before/after, exact raw bytes and width,
uppercase mnemonic, an effect description, and complete resulting state.
Errors distinguish lifecycle, fetch/truncation/alignment, decode/EA, privilege,
trap, data range, state validation, halt, and bounded-execution failures, with
stable nonempty display text.

## Acceptance criteria

- Tests cover every claimed instruction/form, all compatible `MOVE.L` source
  and destination mode classes, all brief scales, full-format suppression and
  displacement sizes, pre/post indirect ordering, PC bases, aliases, flags,
  32-bit wrapping calculations, stack banks, lifecycle, and atomic faults.
- Data-transfer tests include odd-address word/long helpers, odd `MOVE.L`
  sources and destinations, odd memory-indirect pointer longwords, and
  whole-range boundary rollback while fetch alignment remains enforced.
- Differential tests compare every claimed inherited family with the checked
  Layer 07n simulator from equivalent state.
- The unimplemented families listed above remain explicitly queued.
- Line coverage exceeds 80%.
- Formatting, strict Clippy, rustdoc, release tests, package BUILD, lessons
  validation, and affected workspace compilation are run before review.
