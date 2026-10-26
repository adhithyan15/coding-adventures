# Layer 07bd — Microwatt / Power ISA Fixed-Point Functional Simulator

## 1. Status, purpose, and evidence lock

**Status:** independently reviewed specification; implementation authorized.
The exact reviewed spec-only head is
`7bbb725e059c97c75bbdfb1d026a25828f163968`.

This cell specifies one bounded, checked functional simulator for the scalar
fixed-point execution surface shared by Power ISA Version 3.0C and the
Microwatt soft core. It is the functional predecessor of any later
Microwatt-derived gate model. It is **not** a cycle model, a complete Power ISA
implementation, a Microwatt SoC, or evidence that this repository reproduces
the synthesized structure of a particular FPGA build.

The architecture source is the OpenPOWER Foundation's public *Power ISA
Version 3.0C*, dated 1 May 2020:

- specification page: <https://openpowerfoundation.org/specifications/isa/>;
- Version 3.0C document download:
  <https://files.openpower.foundation/s/eRGYZi56BXmCSY7/download>;
- immutable downloaded document SHA-256:
  `0c0064eda27161dcddf52f69a5d26799aaadc5ec73a2de9d90ed09a6ce015010`;
- Book I supplies instruction formats, register definitions, fixed-point,
  branch, and load/store semantics;
- Book II supplies the storage and synchronization semantics needed by this
  single-agent model; and
- Appendices C and D supply the primary and extended opcode maps.

The implementation source is Microwatt commit
[`f341bed306a9a7281bf65200df8cc1f0a002617d`](https://github.com/antonblanchard/microwatt/tree/f341bed306a9a7281bf65200df8cc1f0a002617d).
That commit, rather than a moving branch name, is the RTL and differential
oracle lock. Its relevant source files are `README.md`, `core.vhdl`,
`common.vhdl`, `decode_types.vhdl`, `predecode.vhdl`, `decode1.vhdl`,
`execute1.vhdl`, `divider.vhdl`, `ppc_fx_insns.vhdl`, and `loadstore1.vhdl`.
They establish a VHDL-2008 Open POWER core, its decoder operations, fixed-point
helper behavior, XER bits, multiplier/divider result shaping, little-endian
64-bit reset mode, byte reversal, and the simulation `attn` termination path.
The pinned README explicitly lists Vector/VMX/VSX among the unimplemented
instruction areas.

The checked-in implementation must record both source locks in its README and
generated-vector manifest. A future source upgrade is a new reviewed cell; it
must not silently broaden this one.

## 2. Exact machine profile

The simulator pins one immutable profile:

- Power ISA Version 3.0C Book I scalar fixed-point instructions listed in
  Section 7 below;
- Microwatt's 64-bit mode (`MSR[SF] = 1`) and little-endian mode
  (`MSR[LE] = 1`) as constants, not mutable architectural state;
- one hardware thread in a fixed privileged/supervisor context, no privilege
  transitions, and no asynchronous inputs;
- 32 general-purpose registers of 64 bits each;
- 64-bit CIA, LR, and CTR;
- one 32-bit condition register with eight four-bit fields;
- the architecturally visible XER bits SO, OV, CA, OV32, and CA32;
- a flat, owned 65,536-byte memory; and
- one installed raw program image plus a checked halt state.

All arithmetic wraps at the architected operand width unless this specification
requires a checked fault. Power ISA bit numbers count from the most-significant
bit; Rust storage uses ordinary integer bit positions. Decoder helpers must
name the conversion and must never mix the two conventions implicitly.

GPR0 is a normal writable register. Only instruction operands defined by the
ISA as `(RA|0)` substitute zero when the RA field is zero. No implementation
shortcut may make GPR0 globally constant or apply the substitution to an
ordinary GPR source.

## 3. Complete public state

`MicrowattPowerIsaState` contains, and equality compares:

- `gpr: [u64; 32]`;
- `cia: u64`, `lr: u64`, and `ctr: u64`;
- `cr: u32`, preserving all CR fields in architectural bit order;
- `xer`, containing exactly Boolean `so`, `ov`, `ca`, `ov32`, and `ca32`;
- `memory: [u8; 65_536]`;
- `halted: bool`;
- `program_origin: Option<u16>`; and
- `program_len: u32`.

There is no hidden architectural register bank. Decoder caches or validation
scratch are private derived data, are never state authority, and must be
resynchronized after reset, restore, and rollback. Fixed SF/LE mode is exposed
as profile metadata, not writable state.

Reset zeroes every GPR, CIA, LR, CTR, CR, modeled XER bit, and memory byte;
clears halt and installed-image metadata; and restores the fixed profile.
Constructors are equivalent to reset. A snapshot is a deep copy and restore
validates it completely before any commit.

Restore rejects a state when:

- the image metadata is internally inconsistent, its origin is not four-byte
  aligned, or its range wraps or exceeds memory;
- CIA is not four-byte aligned or is greater than the one-past-memory sentinel
  65,536;
- there is no installed image but execution state claims halt; or
- any serialized reserved XER bit or mutable profile bit is present.

The concrete Rust representation may serialize XER as a normalized `u64`, but
only ISA positions SO=32, OV=33, CA=34, OV32=44, and CA32=45 may be nonzero.
State projection and `mfspr XER` use that mapping; `mtspr XER` masks every
unmodeled position to zero.

## 4. Program and data transport

Programs are supplied as raw bytes in architectural little-endian order. Every
instruction is exactly four bytes. Fetch at CIA retains those exact bytes and
decodes `u32::from_le_bytes(raw)`. The resulting word is then interpreted with
Power ISA's most-significant-bit-first field numbering. No ELF loader, host
word serialization, byte guessing, or implicit endian switch belongs in this
cell.

`load_program(bytes, origin)` requires:

- nonempty bytes;
- an origin divisible by four;
- `origin + len` representable without wrap and at most 65,536; and
- fallible allocation to have succeeded before reset or mutation.

A successful load performs reset, copies the image, records its exact origin
and length, and sets CIA to origin. A failed load changes nothing. Instruction
fetch is permitted only when all four fetched bytes are inside the installed
image. An image may end with one to three residual bytes so the next step can
report `TruncatedInstruction` with those exact bytes. Data access may use any
owned-memory address, including installed code; self-modifying code is
therefore observable on the next fetch.

Scalar data is also little-endian. Byte, halfword, word, and doubleword
accesses have widths 1, 2, 4, and 8. Halfword, word, and doubleword accesses
must be naturally aligned. The byte-reverse instructions reverse the selected
2-, 4-, or 8-byte transfer relative to ordinary little-endian transfer. Every
effective address is first computed in 64-bit wrapping arithmetic and then
range-checked as the whole access; it is never masked into 64 KiB.

## 5. Condition, carry, and width rules

Each CR field is `[LT, GT, EQ, SO]` in architectural order. Compare
instructions replace only BF and copy current XER.SO into that field's SO bit.
Record-form fixed-point instructions replace CR0 from the signed architected
64-bit result written by the instruction and current XER.SO after any overflow
update. The instruction-specific word-result shaping occurs before that
full-64-bit comparison. Record forms preserve CR1-CR7.

Architecturally defined arithmetic follows the Version 3.0C pseudocode
exactly; Section 7.2 separately pins Microwatt's deterministic values where a
word operation leaves part of a 64-bit result implementation-defined:

- CA is the 64-bit carry/borrow-not indicator for doubleword operations;
- CA32 is the corresponding low-32-bit indicator;
- OE forms replace OV and OV32 from signed overflow at their respective widths
  and update sticky SO to `old_SO OR new_OV`; OV32 alone never sets SO;
- non-OE forms preserve SO, OV, and OV32;
- instructions that do not define CA/CA32 preserve them;
- word results use the shaping pinned in Section 7.2 where applicable and the
  individual ISA operation otherwise, never one generic host cast; and
- all five modeled XER bits participate in snapshots and rollback.

Implementations must use explicit widened integer arithmetic for reference
checks. Host debug-overflow behavior, signed-shift behavior, or casts are not
architectural semantics.

## 6. Addressing, branches, and control flow

D-form and DS-form effective addresses use `(RA|0) + sign_extended_displacement`.
Indexed forms use `(RA|0) + GPR[RB]`. Update forms require RA != 0; load-update
forms additionally require RA != RT. The base update commits only after the
entire memory operation succeeds.

For `b` and `bc`, AA chooses absolute versus CIA-relative targets and LK writes
CIA+4 to LR. `bclr` uses the pre-instruction LR target and `bcctr` uses the
pre-instruction CTR target. LK still records CIA+4 whether or not a conditional
branch is taken. Conditional branches implement the complete BO/BI algorithm
from Version 3.0C, including CTR decrement and CR polarity. Encodings that ask
`bcctr` to decrement/test CTR are reserved and illegal. BH is accepted only in
its architecturally valid encodings but has no predictor/timing effect.

The fall-through address is CIA+4 and may become the aligned one-past-memory
sentinel. Its next `step` performs the ordinary installed-image fetch check.
Every taken target must be four-byte aligned and point to a byte in the
installed image before the branch commits. A target in the image's final one
to three bytes may commit, after which `step` reports `TruncatedInstruction`.
There is no address truncation. LR and CTR retain all 64 bits; branch targets
apply only the architecturally specified low-bit clearing before alignment and
image validation.

## 7. Included instruction surface

Only the following scalar instructions are implemented. Optional OE and Rc
suffixes are accepted exactly where their Version 3.0C forms define those
bits. Per Version 3.0C Book I section 1.3.3, fields printed as `/` are ignored
by the processor even when nonzero; only defined fields containing reserved
values are illegal. In particular, fixed-Rc count/parity instructions ignore
Rc, high multiply ignores its slash OE position, and compare, branch-hint,
condition, synchronization, and XFX slash fields do not constrain decode.

### 7.1 Immediate arithmetic and logical

- `addi`, `addis`, `addic`, `addic.`, `subfic`, and `mulli`;
- `ori`, `oris`, `xori`, `xoris`, `andi.`, and `andis.`; and
- `cmpi` and `cmpli`, with the L bit selecting word or doubleword comparison.

The sole assembly alias used by canonical traces is `nop` for
`ori r0,r0,0`. Compare traces use `cmpwi`, `cmpdi`, `cmplwi`, or `cmpldi` so
the width is explicit.

### 7.2 Register arithmetic

- `add`, `addc`, `adde`, `addme`, and `addze`;
- `subf`, `subfc`, `subfe`, `subfme`, and `subfze`;
- `neg`;
- `mulhw`, `mulhwu`, `mulhd`, `mulhdu`, `mullw`, and `mulld`; and
- `divw`, `divwu`, `divd`, and `divdu`.

All architecturally defined OE/Rc combinations for these listed operations are
included. Division by zero and signed minimum divided by negative one are
checked `DivideFault` cases in this teaching boundary, regardless of OE. This
is an intentional fail-closed lifecycle strengthening; it is not claimed as a
Power ISA synchronous-exception model and is not compared as a valid Microwatt
instruction result.

The pinned Microwatt RTL makes three deterministic 64-bit-mode choices for word
multiply/divide results, and this model reproduces them exactly:

- `mulhw` and `mulhwu` take product bits 63:32 and duplicate that 32-bit word
  into both halves of the 64-bit destination; and
- `mullw` places the complete signed 32-by-32 product in the 64-bit
  destination. OE compares that product with the signed 32-bit range and the
  multiplier's word-width overflow drives both OV and OV32; SO remains sticky
  from OV only; and
- `divw` and `divwu` place quotient bits 31:0 in the low half and clear the
  destination's upper 32 bits.

For record forms, CR0 compares that complete shaped 64-bit destination against
zero. Consequently a negative `divw.` quotient has a zero high half and records
GT rather than LT, while `mulhw[u].` derives sign and zero from the duplicated
result. These are deliberate Microwatt-profile rules, not host casts or a
generic Power ISA word-extension assumption.

### 7.3 Logical, count, extend, rotate, and shift

- `and`, `andc`, `or`, `orc`, `xor`, `nand`, `nor`, and `eqv`;
- `extsb`, `extsh`, and `extsw`;
- `cntlzw`, `cntlzd`, `cnttzw`, and `cnttzd`;
- `popcntb`, `popcntw`, `popcntd`, `prtyw`, and `prtyd`;
- `rlwinm`, `rlwnm`, `rlwimi`;
- `rldic`, `rldicl`, `rldicr`, `rldimi`, `rldcl`, and `rldcr`; and
- `slw`, `srw`, `sraw`, `srawi`, `sld`, `srd`, `srad`, and `sradi`.

Mask construction, the word-versus-doubleword result rule, arithmetic fill,
CA update, and shift counts at and beyond 32 or 64 follow the ISA pseudocode,
not the host language's shift operators. In this 64-bit profile, `rlwimi`
preserves the destination's upper 32 bits and merges only the masked low-word
lanes. The two `sradi` encodings use Microwatt's exact ten-bit selectors 826
and 827. Word arithmetic right shifts sign-extend their low-word result to the
complete 64-bit destination before Rc records CR0.

### 7.4 Condition register and special registers

- `cmp`, `cmpl`, and `mcrf`;
- `crand`, `crandc`, `creqv`, `crnand`, `crnor`, `cror`, `crorc`, and `crxor`;
- `mfcr` and `mtcrf`, in their L=0 forms; and
- `mfspr` and `mtspr` only for XER (SPR 1), LR (SPR 8), and CTR (SPR 9).

`mfcr` ignores the FXM field because it is slash-reserved in its L=0 form;
`mtcrf` honors its field mask exactly. The L=1 `mfocrf`/`mtocrf` forms are
recognized but outside this boundary and return `UnsupportedInstruction`
regardless of FXM. Any otherwise valid special-register
encoding naming another SPR returns `UnsupportedSpecialRegister` atomically.

### 7.5 Branch

- `b` with all valid AA/LK combinations;
- `bc` with every valid BO/BI/AA/LK combination; and
- `bclr` and `bcctr` with every valid BO/BI/BH/LK combination.

Canonical traces use base mnemonics plus `a` and `l` suffixes as encoded; they
do not substitute `blr`, `bctr`, `bdnz`, or other assembler aliases.

### 7.6 Loads and stores

- displacement: `lbz`, `lbzu`, `lhz`, `lhzu`, `lha`, `lhau`, `lwz`, `lwzu`,
  `lwa`, `ld`, `ldu`, `stb`, `stbu`, `sth`, `sthu`, `stw`, `stwu`, `std`, and
  `stdu`;
- indexed: `lbzx`, `lbzux`, `lhzx`, `lhzux`, `lhax`, `lhaux`, `lwzx`,
  `lwzux`, `lwax`, `lwaux`, `ldx`, `ldux`, `stbx`, `stbux`, `sthx`, `sthux`,
  `stwx`, `stwux`, `stdx`, and `stdux`; and
- byte reverse: `lhbrx`, `lwbrx`, `ldbrx`, `sthbrx`, `stwbrx`, and `stdbrx`.

Unsigned loads zero-extend, algebraic loads sign-extend, and stores select the
low transfer-width bits. DS-form reserved low bits are validated before any
effective-address work.

### 7.7 Synchronization and deterministic termination

- `sync` for L=0 and the architecturally named `lwsync` for L=1;
- `eieio` and `isync`; and
- exact word `0x0000_0200`, traced as `attn`.

The synchronization instructions are successful, trace-visible operations
that advance CIA but have no further effect in a single-agent, no-device
memory model. L=2 is the valid `ptesync` instruction; because this profile
excludes MMU and page-table synchronization, it is recognized and returns
`UnsupportedInstruction` atomically. Only L=3 is reserved and
`IllegalInstruction`. The exact `attn` word is the pinned Microwatt simulation
termination convention: it produces one successful trace, advances CIA by
four, and sets `halted`. It is not presented as an application-level Power ISA
halt instruction. Any other instruction that reaches Microwatt's broad ATTN
decode but fails its bits 1-10 qualification is illegal.

## 8. Explicit exclusions

This cell excludes:

- floating-point, decimal floating-point, Vector/VMX/AltiVec, VSX, MMA, and
  prefixed instructions;
- transactional memory, load-reserve/store-conditional, reservation state,
  and multi-agent memory ordering;
- MMU, TLB, SLB, page tables, segment state, cache state, cache maintenance,
  translation, and virtual addresses;
- system calls, traps, exception entry/return, interrupts, decrementer,
  timebase, PMU, privileged/hypervisor state, mutable MSR, and unsupported
  SPRs;
- TAR branches, branch prediction, speculation, pipeline stages, scoreboards,
  forwarding, stalls, cycle counts, and implementation timing;
- Wishbone, debug/JTAG, UART, GPIO, DRAM, firmware/boot ROM, DMA, peripherals,
  clocks, reset trees, and any complete SoC platform; and
- synthesis, FPGA primitives, cell libraries, physical timing, power,
  transistor, or SPICE behavior.

An encoding that is valid in the broader ISA or pinned Microwatt but belongs
to an excluded facility returns typed `UnsupportedInstruction`; it must not be
misreported as an unknown opcode or silently executed.

## 9. Decode, trace, and error contract

The decoder is table-driven from the official Version 3.0C opcode appendices.
Microwatt decode tables are a second implementation witness, never the sole
architecture transcription. The implementation must distinguish:

- `IllegalInstruction` for unallocated primary/extended selectors, reserved
  values in defined fields, and architecturally invalid option combinations;
- `UnsupportedInstruction` for a recognized instruction outside Section 7;
- `UnsupportedSpecialRegister` for a recognized `mfspr`/`mtspr` targeting an
  unmodeled SPR;
- `TruncatedInstruction`, `FetchOutsideProgram`, `MisalignedFetch`,
  `MisalignedAccess`, `MemoryOutOfRange`, `ProgramOutOfRange`,
  `BranchOutsideProgram`,
  `DivideFault`, `Halted`, `InvalidProgram`, `InvalidState`, `StepLimit`, and
  `AllocationFailed` for the corresponding lifecycle conditions.

Fault payloads include the faulting CIA and, where applicable, exact raw bytes,
raw word, selector, address, width, or SPR. Error formatting is stable and
contains no host path or allocation internals.

Within `step`, deterministic fault precedence is: `Halted`; misaligned CIA;
CIA outside an installed image; fewer than four installed bytes remaining;
illegal or recognized-but-unsupported decode; unsupported SPR or invalid
instruction option; checked divide failure; data alignment then data range;
and taken-branch target alignment then installed-image range.
`ProgramOutOfRange` describes a rejected load, while `BranchOutsideProgram`
carries the rejected taken target. Decode therefore precedes
operand-dependent execution faults, and no speculative effective address or
branch mutation is observable. Allocation preflight may return
`AllocationFailed` before execution begins but never after architectural
mutation.

Every successful `StepTrace` contains:

- CIA before and after;
- the exact four fetched bytes and decoded raw `u32`;
- width `4` and one deterministic lowercase canonical mnemonic;
- branch taken/not-taken and target when applicable;
- at most one scalar memory transfer record with kind, address, width, and
  transferred value;
- committed GPR/special-register/CR/XER effects; and
- complete deep state before and after.

Arithmetic mnemonics append `o` before record `.`, yielding forms such as
`addo.`. Immediate branches use `b`/`bc`, append `l` for LK, then append `a`
for AA, yielding `b`, `bl`, `ba`, or `bla` and the corresponding `bc` forms.
Indirect branch mnemonics append only `l` for LK. Apart from `nop` and the
width-explicit compare names, traces use no assembler aliases. Trace raw bytes
come from the fetch snapshot even when the instruction stores over itself.
Traces never claim pipeline stages, buses, or cycles.

## 10. Transactional lifecycle

Every public mutation is checked and atomic:

1. validate arguments and current state;
2. fetch and decode without architectural mutation;
3. compute all register, control-flow, and memory effects in a fallible private
   candidate;
4. validate every address, option, branch target, and allocation;
5. build the complete trace; and
6. commit once.

An error leaves all public state, memory, image metadata, halt state, and
private validation state byte-for-byte unchanged. Direct register and memory
APIs use the same range/alignment rules. `step` after `attn` returns `Halted`
without fetching or mutation.

`run(max_steps)` requires a positive bound, retains at most 64 complete traces,
and enforces a conservative 8 MiB retained-trace budget before execution. It
rolls the entire call back on the first fault, allocation failure, or exhausted
step bound. `execute(bytes, origin, max_steps)` preflights its program and trace
storage and atomically combines load and run; failure restores the exact
pre-call machine. Rollback itself cannot allocate.

## 11. Required verification

Completion requires all of the following.

### 11.1 Exhaustive classification

- enumerate all 64 primary opcode values;
- within every relevant primary family, enumerate all 1,024 ten-bit extended
  selectors or all 512 nine-bit OE selectors as applicable;
- exhaustively cover branch BO (32), BI (32), AA, LK, BH-validity, CR field,
  record, overflow, field-mask, L, defined-field reserved values, and every
  slash-field bit that must be classification-invariant;
- keep immediate/register payloads representative while exhaustively varying
  every field that can change classification; and
- prove each supported family plus adjacent reserved and recognized-but-
  unsupported encodings maps to exactly one result class.

The classification oracle is independently transcribed from Version 3.0C. It
must not call the production decoder or derive expected results by serializing
the production instruction enum.

### 11.2 Semantic and differential evidence

- a deterministic GHDL harness at Microwatt commit
  `f341bed306a9a7281bf65200df8cc1f0a002617d`, with FPU disabled and 64-bit
  little-endian mode pinned, seeds GPRs, CR, XER, LR, CTR, CIA, and bounded
  memory, executes one valid included instruction, and exports complete state;
- checked-in generated vectors record the commit, harness command, generator
  version, seed, row count, and content SHA-256 so CI does not need the network;
- the RTL corpus contains at least one complete-state row for every included
  execution family/form plus explicit source-lock rows for slash-field decode,
  SO/OV32, word multiply/divide shaping, arithmetic shifts, parity, branch,
  memory, and synchronization. It does not claim that all BO/BI outcomes,
  carry/overflow inputs, shift counts, or masks are separate RTL rows; those
  exhaustive matrices are independent Rust truth-table/property tests;
- OE rows include `addo` with `0x0000_0000_7fff_ffff + 1`, which produces
  OV=0 and OV32=1, once with incoming SO=0 and once with incoming SO=1; both
  rows prove SO is unchanged by OV32 alone, and the Rc variant proves CR0.SO
  observes that unchanged value;
- word multiply/divide rows include `mulhw.` and `mulhwu.` results that prove
  bits 63:32 equal bits 31:0, plus negative-quotient `divw.` and nonzero
  `divwu.` results that prove bits 63:32 are zero and CR0 compares the complete
  shaped 64-bit value; `mullwo.` rows cover positive, negative, and nonoverflow
  signed-word products and prove the full product plus OV/OV32/SO boundary;
- sync classification and lifecycle rows cover L=0 `sync`, L=1 `lwsync`,
  recognized-but-unsupported L=2 `ptesync`, and reserved L=3 without mutation;
- independent Rust tests cover the source-locked add/subtract carry and signed
  overflow boundaries, shift counts at/beyond 32 and 64, every CR Boolean
  operation, and representative taken/not-taken BO/BI/CTR branch decisions;
  exhaustive encoding classification is required by Section 11.1, while an
  exhaustive semantic input-space claim is intentionally not made; and
- typed fault, lifecycle, image-bound, rollback, allocation, and unsupported-
  facility cases are Rust boundary tests rather than fabricated RTL results.

The generator and both Rust consumers must rebuild and reproduce the committed
corpus exactly. Neither consumer may reuse production decode or execution as
its expected-value oracle.

### 11.3 Repository completion gate

The implementation is complete only when it also has:

1. package README and changelog documenting this exact boundary and both
   immutable source locks;
2. Cargo workspace and BUILD integration, affected-package builds, and vector
   regeneration/schema validation;
3. rustfmt, strict Clippy, rustdoc warnings-as-errors, release unit,
   differential, lifecycle, and doc tests all green;
4. release line coverage above 80%, with decode, fault, rollback, and trace
   paths included rather than excluded;
5. relevant repository lessons and backlog updated; and
6. an independent security/correctness review of the exact clean implementation
   head before publication.

## 12. Follow-on boundary

A later gate cell may use the pinned Microwatt VHDL as source evidence for a
bounded educational topology. It must separately state which RTL structures
it reproduces, count every stable storage bit, make those Q outputs the sole
architectural authority, and prove full-state differential equivalence to this
functional oracle. Cache/MMU/SoC integration and platform buses remain their
own cells. Transistor and analog claims wait for the repository's SPICE/device
model wave.
