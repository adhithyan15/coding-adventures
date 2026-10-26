# Layer 07az — Xtensa LX6 Functional Simulator

## 1. Status, source authority, and configuration identity

This specification defines the first checked Xtensa functional cell in the
repository. The package is an instruction-level oracle for a single
**ESP32-class Xtensa LX6 integer core**, not a cycle-accurate ESP32 and not a
claim to reproduce a proprietary Tensilica implementation.

**Status:** independently reviewed specification; implementation authorized.
The exact reviewed spec-only head was
`83c2113b12e03601c7bafcbd5c50e2a423828202`.

The public primary sources are:

- Tensilica, Inc., *Xtensa Instruction Set Architecture (ISA) Reference
  Manual*, RC-2010.1, issue date April 2010, document PD-09-0801-10-01. A
  publicly accessible copy is available at
  <https://0x04.net/~mwk/doc/xtensa.pdf>. Chapters 3 through 7 are normative
  for instruction fetch, formats, state, options, and instruction semantics.
- Espressif Systems, *ESP32 Series Datasheet*, version 5.3, section 4.1.1, at
  <https://documentation.espressif.com/esp32_datasheet_en.html>. It identifies
  the shipped core as Xtensa LX6 and publicly confirms the mixed 16/24-bit
  instruction set, floating-point support, DSP multiply/divide/MAC support,
  and interrupt-vector boundary.
- Espressif Systems, *ESP32 Technical Reference Manual*, version 5.9, sections
  3.3.1 and 3.3.2, at
  <https://www.espressif.com/sites/default/files/documentation/esp32_technical_reference_manual_en.pdf>.
  It is normative here only for the ESP32 choice of little-endian instruction
  and data buses and for the fact that ordinary ESP32 data-bus byte,
  half-word, and word accesses may be unaligned.
- Espressif Systems, ESP-IDF `xtensa/config/core-isa.h` for the ESP32 target,
  at
  <https://github.com/espressif/esp-idf/blob/master/components/xtensa/esp32/include/xtensa/config/core-isa.h>.
  It is the public configuration cross-check for the LX6 option profile named
  below; instruction semantics continue to come from the Xtensa ISA manual.

The named, immutable package profile is `Esp32Lx6IntegerV1`:

| Configuration parameter | Pinned value |
|---|---|
| Core generation | Xtensa LX6, one core |
| Byte order | little-endian instruction and data transport |
| Address width | 32 bits architecturally; 64 KiB owned teaching memory |
| Physical AR registers (`NAREG`) | 64, with 16 logical `a0`–`a15` registers |
| Windowed Register Option | enabled |
| Code Density Option | enabled |
| Loop Option | enabled |
| User execution | problem-state integer execution only |
| Exception/interrupt delivery | surfaced as typed atomic boundary events |
| Debug | `BREAK 0,0` is the repository halt sentinel; other debug behavior is excluded |

The choice of 64 physical registers is part of this package profile, not a
runtime switch. Logical register `aN` maps to physical register
`AR[((WindowBase * 4) + N) mod 64]`. A snapshot always owns all 64 physical
registers, so hidden windows remain observable and restorable.

The ESP32 datasheet also confirms hardware whose complete public encoding and
state contract is not needed for this bounded first cell. Floating point,
MAC16/DSP, multiply/divide extensions, conditional store, Boolean registers,
thread pointer, coprocessors, custom TIE instructions, exception/interrupt
entry, timers, and debug hardware are explicitly deferred. An encoding from a
documented but deferred standard option is `UnsupportedOptionInstruction`; an
encoding that cannot be tied to a public standard option is
`UnknownInstruction`. No proprietary opcode, register, vector layout, or
microarchitectural behavior may be inferred.

## 2. Scope and non-goals

The implementation boundary is the public Xtensa Core Architecture plus the
Code Density, Loop, and application-visible Windowed Register instructions.
It includes:

- wrapping 32-bit arithmetic, scaled add/subtract, bitwise logic, conditional
  moves, field extraction, and SAR-based immediate/register shifts;
- byte, half-word, word, and literal loads and byte, half-word, and word
  stores;
- register, immediate, bit-test, zero-test, and mask-test branches;
- direct and indirect jumps, call0 calls/returns, windowed calls, `ENTRY`,
  `RETW`, and `MOVSP`;
- `LOOP`, `LOOPGTZ`, and `LOOPNEZ` with exact `LBEG`, `LEND`, and `LCOUNT`
  loopback semantics;
- ordering instructions as deterministic no-ops in the single-core owned
  memory model; and
- the public 16-bit Code Density forms corresponding to the included
  operations.

The cell is functional, not timed. It does not model the LX6 seven-stage
pipeline, speculative fetch, stalls, caches, buses, arbitration, MMU
translation, physical ESP32 address aliases, interrupt timing, dual-core
interaction, DMA, peripherals, or electrical behavior. It does not consume
ELF files or perform assembler/linker relaxation; input is an already linked
raw byte image.

## 3. Complete owned architectural state

`XtensaLx6State` is a frozen, owned value containing exactly:

- `ar: [u32; 64]`, the complete physical address-register file;
- `pc: u32`;
- `sar: u8`, restricted to six bits;
- `window_base: u8`, restricted to four bits;
- `window_start: u16`, one active-window bit for each group of four physical
  registers;
- `call_inc: u8`, the two-bit `PS.CALLINC` field used by windowed calls and
  `ENTRY`;
- `lbeg: u32`, `lend: u32`, and `lcount: u32`;
- exactly 65,536 bytes of little-endian owned memory;
- `halted: bool`; and
- installed-program `origin: Option<u32>` and `length: u32`.

There is no integer condition-code register and no hard-wired zero register.
Every physical AR is ordinary mutable state. The fixed profile behaves as
problem state with exception mode clear and window-overflow checking enabled;
those constants are not mutable snapshot fields. No deferred FPU, MAC,
Boolean, exception, interrupt, cache, or MMU register may appear as a hidden
host field.

The architectural manual leaves `WindowBase`, `WindowStart`, `LBEG`, and
`LEND` undefined after hardware reset. For deterministic repository behavior,
`reset` chooses `WindowBase = 0`, `WindowStart = 1`, `LBEG = LEND = LCOUNT =
0`, `CALLINC = 0`, `SAR = 0`, zeroes all ARs and memory, clears halt, and
removes the installed image. This is a simulator reset convention, not a
silicon reset claim.

Snapshots and equality compare every field and every memory byte. Public
logical-register helpers are views over the physical file and may never be an
independent `[u32; 16]` authority or mirror.

## 4. Memory, transport, and instruction fetch

### 4.1 Program installation

`load(program, origin)` accepts a nonempty byte image only when the complete
half-open range `[origin, origin + length)` fits in the owned 64 KiB memory and
the addition does not overflow. Xtensa instructions are byte addressed and
mixed-width; the origin has no alignment requirement. Successful load first
performs all validation, then resets the complete machine, copies the image,
sets `pc = origin`, records the exact origin and length, and leaves halt clear.

The teaching memory is unified so tests can observe deterministic
self-modifying code. This does not deny the separate instruction and data
buses of Xtensa or ESP32; bus topology and address aliases belong to the SoC
cell. Stores that overlap installed bytes are permitted. A trace always keeps
the raw bytes fetched before the instruction performs any store.

### 4.2 Byte order and data access

Instruction and data bytes use the ESP32 little-endian choice. Multi-byte
values place the least-significant byte at the lowest address. Ordinary
`L16*`, `L32I`, `S16I`, and `S32I` accesses are permitted at unaligned
addresses, matching the public ESP32 data-bus contract, and are assembled or
split bytewise. The complete byte range is validated before a load result or
any store byte is committed. No access wraps at 64 KiB.

`L32R` uses the Core Architecture PC-relative literal rule, including its
architectural word-aligned base and scaled signed displacement. Literal data
must fit in memory; it need not be inside the installed program image.

### 4.3 Mixed-width fetch and classification

Every `step` first validates that `pc` is inside the installed image and that
two bytes remain. It assembles the low 16 bits little-endian and uses bit 3 of
the low `op0` nibble exactly as the ISA fetch pseudocode does:

- `op0[3] == 1` selects a 16-bit Code Density instruction;
- `op0[3] == 0` selects a 24-bit instruction and requires a third installed
  byte.

Fewer than two bytes, or a selected 24-bit instruction with no third byte, is
`TruncatedInstruction`. A PC outside the installed half-open range is
`FetchOutsideProgram`. Fetch reads only the bytes of the selected instruction,
not a speculative aligned block.

The decoder must use the exact masks and fixed/reserved bits in ISA Manual
chapters 6 and 7. Classification order is:

1. determine width from `op0[3]`;
2. recognize an exact included encoding;
3. recognize exact documented encodings belonging to deferred standard
   options as `UnsupportedOptionInstruction`;
4. recognize privileged/window-handler/debug instructions that this
   problem-state cell excludes as `PrivilegedInstruction` or `Breakpoint`;
5. classify all remaining reserved or unassigned words as
   `UnknownInstruction`.

Assembler aliases are not separate encodings. Reserved fields must match; a
nearby word with a changed fixed bit must not fall through to a broader family.

## 5. Included instruction boundary

### 5.1 Core 24-bit instructions

The included Core Architecture surface is:

- `ABS`, `ADD`, `ADDI`, `ADDMI`, `ADDX2`, `ADDX4`, `ADDX8`, `NEG`, `SUB`,
  `SUBX2`, `SUBX4`, and `SUBX8`;
- `AND`, `OR`, `XOR`, `EXTUI`, `MOVEQZ`, `MOVNEZ`, `MOVGEZ`, `MOVLTZ`, and
  `MOVI`;
- `SLL`, `SLLI`, `SRA`, `SRAI`, `SRC`, `SRL`, `SRLI`, `SSA8B`, `SSA8L`,
  `SSAI`, `SSL`, and `SSR`;
- `L8UI`, `L16SI`, `L16UI`, `L32I`, `L32R`, `S8I`, `S16I`, and `S32I`;
- `BALL`, `BANY`, `BNALL`, `BNONE`, `BBC`, `BBS`, `BBCI`, `BBSI`, `BEQ`,
  `BNE`, `BEQI`, `BNEI`, signed `BGE`, `BLT`, `BGEI`, `BLTI`, `BGEZ`,
  `BLTZ`, and unsigned `BGEU`, `BLTU`, `BGEUI`, `BLTUI`, plus `BEQZ` and
  `BNEZ`;
- `J`, `JX`, `CALL0`, `CALLX0`, and `RET`;
- `RSR`, `WSR`, and `XSR` only for the non-privileged included special
  registers `SAR`, `LBEG`, `LEND`, and `LCOUNT` and only with the ISA-defined
  widths; and
- `NOP`, `DSYNC`, `ESYNC`, `ISYNC`, `RSYNC`, `MEMW`, and `EXTW` as ordered,
  traceable no-ops in this single-core model.

All additions and subtractions wrap modulo 2^32. Signed relations reinterpret
the same 32 bits as two's-complement; unsigned relations do not. Operations do
not synthesize flags. For an instruction whose destination is also a source,
all inputs are read from the pre-state before any destination is written.

`SSA8B` is part of the Core Architecture even in this little-endian profile;
its name describes a shift-amount operation, not a selectable processor byte
order. It writes exactly
`SAR = 32 - (AR[s][1:0] << 3)`. Thus low source bits `0`, `1`, `2`, and `3`
produce `SAR` values `32`, `24`, `16`, and `8`, respectively. `SSA8L` retains
its distinct ISA-defined operation.

### 5.2 Code Density instructions

The included 16-bit surface is `ADD.N`, `ADDI.N`, `L32I.N`, `S32I.N`,
`MOV.N`, `MOVI.N`, `BEQZ.N`, `BNEZ.N`, `NOP.N`, `RET.N`, and `RETW.N`.
The exact immediate maps, scaling, signedness, and forward-only density branch
offsets follow the ISA instruction descriptions. `ILL.N` is an explicit
`UnknownInstruction`; `BREAK.N` is `Breakpoint`, not a halt alias.

### 5.3 Reserved encodings and documented undefined operations

The following encoded relations are architecturally reserved and are rejected
as `UnknownInstruction { raw, width }` during classification, before any
register-window check or semantic evaluation:

- each included 24-bit conditional-branch format rejects the encoded signed
  displacement values `-3`, `-2`, `-1`, and `0`;
- `EXTUI` rejects `sa + op2 > 31`; and
- `MOV.N` rejects equal encoded source and destination logical ARs.

These tests are part of exact decoding; they may not be deferred until an
address, condition, or result is evaluated. Legal neighboring encodings at
`-4` and `1`, `EXTUI sa + op2 = 31`, distinct-register `MOV.N`, and both
same- and distinct-register `MOVSP` must remain accepted as encodings.

The ISA separately describes the following operations as undefined after a
valid encoding has been identified. They return
`UndefinedOperation { raw, width, reason }` atomically:

- `ENTRY` when its source-register field is greater than `3`
  (`InvalidEntrySource`); this check occurs after the instruction's mandatory
  `WindowCheck(0, CALLINC, 0)`, matching the ISA operation order;
- `SLL`, `SRA`, `SRC`, and `SRL` when the pre-state six-bit `SAR` value is
  greater than `32` (`ShiftAmountOutOfRange`); `SAR = 32` remains valid; and
- `RETW` or `RETW.N` when the increment encoded by the top two bits of
  pre-state logical `a0` is zero, or when `m` -- the first set bit among
  `WindowStart[WindowBase - 1]`, `WindowStart[WindowBase - 2]`, and
  `WindowStart[WindowBase - 3]` in that priority order -- is nonzero and does
  not equal that increment (`InvalidReturnWindow`). Index subtraction wraps
  modulo 16. If all three bits are clear (`m = 0`), the relation is legal and
  the missing caller window follows `WindowUnderflow { increment }` instead.

`UndefinedReason` is a public non-exhaustive enum with precisely the three
stable reasons named above for this cell. This distinction prevents
state-dependent undefined behavior from being hidden behind a generic decoder
failure while preserving `UnknownInstruction` for raw reserved encodings.

## 6. Control flow, windows, and loops

### 6.1 Ordinary control flow

Xtensa has no branch delay slot. The fall-through address is `pc + width`.
Every PC-relative base, displacement extension, scale, and call-region rule is
the one in the ISA description; implementation convenience must not replace it
with a generic branch helper. Control transfers commit their architectural
32-bit target even when it is outside owned memory or the installed image; the
following `step` then returns the appropriate fetch fault. This preserves the
architectural ordering between a control transfer and target fetch. A target
has no alignment check, because valid mixed 16/24-bit instructions can begin
at different byte residues.

`CALL0` and `CALLX0` write their exact fall-through return address to logical
`a0`; `RET` and `RET.N` branch through pre-state logical `a0`.

### 6.2 Windowed calls and returns

The included application-visible window surface is `CALL4/8/12`,
`CALLX4/8/12`, `ENTRY`, `RETW`, `RETW.N`, and `MOVSP`.

The fixed profile enables architectural window checking. After exact decode
and raw-reserved validation, but before reading an AR, evaluating an
instruction condition, computing an address, checking a documented undefined
operation, or changing scratch state, every included instruction that refers
to an AR performs the ISA's lazy pre-semantics `WindowCheck(wr, ws, wt)`.
For an ordinary encoding, `wr`, `ws`, and `wt` are respectively the high two
bits (`field[3:2]`) of the `r`, `s`, and `t` AR fields actually referenced by
that instruction; a field not referenced by the instruction contributes
zero, even if its raw bits are nonzero. Reads and writes both count as AR
references. Conditional moves and branches perform this check even when their
condition will be false.

`WindowCheck` uses the pre-state `WindowBase` and `WindowStart`. It selects the
first matching overflow in architectural priority: increment `1` when any
referenced group is nonzero and window `WindowBase + 1` is already active;
otherwise increment `2` when any referenced group has bit 1 set and window
`WindowBase + 2` is active; otherwise increment `3` when any referenced group
equals `3` and window `WindowBase + 3` is active. Window indexes wrap modulo
16. A match returns `WindowOverflow { increment }` without reading an operand
or mutating scratch state.

Instructions with ISA-specific checks use their specified groups rather than
invented operand fields: `CALL4/8/12` and `CALLX4/8/12` use
`WindowCheck(0, 0, n)` for `n = 1/2/3`; `ENTRY` uses exactly
`WindowCheck(0, CALLINC, 0)`; and the documented `MOVSP` and `RETW`/`RETW.N`
availability/underflow rules run after any applicable raw-reserved validation.
Instructions with no AR reference perform no window check.

- A windowed call performs the exact pre-state check above, writes `PS.CALLINC`,
  stores the encoded return address in the documented outgoing logical
  register, and changes PC. It does not rotate `WindowBase`.
- `ENTRY` reads its source using the caller mapping, computes the new stack
  value, rotates `WindowBase` by `CALLINC`, writes through the callee mapping,
  and sets the new `WindowStart` bit. Its read and write may name different
  physical ARs even when the logical numbers look related.
- `RETW`/`RETW.N` derives the decrement from the top two bits of the pre-state
  logical `a0`, reconstructs the target within the current 1 GiB PC region,
  validates the `WindowStart` relation, rotates back, and clears the departing
  window bit only on success.
- `MOVSP` first performs the ordinary lazy `WindowCheck` for its referenced
  source and destination fields. It then inspects the three caller-window bits
  `WindowStart[WindowBase - 3]`, `WindowStart[WindowBase - 2]`, and
  `WindowStart[WindowBase - 1]`, with subtraction wrapping modulo 16. If all
  three bits are zero, it returns the distinct typed `AllocaCause` error
  atomically. Otherwise it copies the pre-state source AR to the destination
  AR. Equal source and destination fields are legal: the instruction remains
  an availability probe followed by a self-move, so it can still report
  `WindowOverflow` or `AllocaCause` before succeeding without changing the
  register value.

An architectural overflow or underflow that would enter an exception handler
is reported as typed `WindowOverflow { increment }` or
`WindowUnderflow { increment }` and leaves the complete state unchanged. The
privileged handler-only `ROTW`, `RFWO`, `RFWU`, `L32E`, and `S32E` instructions
are recognized but return `PrivilegedInstruction`; their encodings must not be
misreported as unknown or partially executed.

Fault precedence is therefore deterministic: halt and fetch/width failures
precede decode; raw reserved encodings precede window checks; a lazy
`WindowOverflow` precedes register reads, false conditions, memory checks, and
the `UndefinedOperation` cases above; then instruction-specific
window-underflow/undefined/allocation checks precede ordinary semantics. In particular,
the ISA-ordered `ENTRY WindowCheck(0, CALLINC, 0)` precedes
`InvalidEntrySource`; the branch, `EXTUI`, and `MOV.N` raw reserved relations
never reach `WindowCheck`; and a legal `MOVSP` reports lazy `WindowOverflow`
before its instruction-specific `AllocaCause`. Every failure preserves all
physical ARs, window state, special registers, memory, PC, and halt state.

### 6.3 Zero-overhead loops

`LOOP`, `LOOPGTZ`, and `LOOPNEZ` execute the ISA operations exactly:

- `LCOUNT = aS - 1` modulo 2^32;
- `LBEG = pc + 3`;
- `LEND = pc + 4 + zero_extend(imm8)`;
- `LOOPGTZ` skips to `LEND` when signed `aS <= 0`; `LOOPNEZ` skips when
  `aS == 0`; plain `LOOP` never skips (zero requests 2^32 iterations).

Before executing each fetched instruction, the sequential candidate PC is
computed. When that candidate equals `LEND` and pre-instruction `LCOUNT != 0`,
the candidate becomes `LBEG` and `LCOUNT` decrements. A taken branch or jump
from the instruction overrides that candidate and therefore does not loop back.
The loop setup instruction validates the ISA alignment restriction on the
first loop instruction and that `LBEG`/`LEND` delimit bytes inside the installed
image. Invalid setup is `InvalidLoop` and is atomic. There is one loop-register
set, so a later loop instruction replaces the prior setup; no host loop stack
exists.

## 7. Exception, privilege, and halt boundary

This first cell does not enter an exception vector. Illegal instructions,
window overflow/underflow, breakpoints, memory/fetch failures, and privileged
operations are typed return values with full rollback. Asynchronous interrupts
never arrive. The implementation therefore must not contain hidden EPC,
EXCCAUSE, EXCVADDR, vector-base, interrupt-enable, or ring state.

The exact documented `BREAK 0,0` encoding is reserved as the repository's
deterministic semihosting halt sentinel. It commits one trace, advances to the
fall-through PC, and sets `halted = true`. Any other `BREAK` or `BREAK.N`
returns `Breakpoint` atomically. A subsequent `step` returns `Halted` without
fetch or mutation.

This halt convention is transport policy, not an assertion that ESP32 silicon
halts on `BREAK 0,0`.

## 8. Checked API and transactional lifecycle

The package exposes checked `new`, `reset`, `load`, `snapshot`, `restore`,
`step`, `run`, and `execute` operations. Public results never borrow simulator
storage.

`restore` validates, before commit:

- exact memory length;
- six-bit SAR, four-bit `WindowBase`, and two-bit `CALLINC` ranges;
- a set current-window bit in `WindowStart`;
- installed origin/length overflow and memory bounds;
- consistency of halt and installation metadata.

PC is not constrained to the installed image during restore: a previously
committed branch may legitimately leave the next fetch outside that image.

Undefined hardware reset values are not accepted as a reason for nondeterminism.

`step` executes against disposable state and commits only after fetch, decode,
all source reads, complete address-range checks, window/loop checks, target
arithmetic, trace construction, and required allocations succeed. `run(max_steps)`
and `execute(program, origin, max_steps)` are transactional as a whole: the
first fault, trace-allocation failure, or exhausted budget restores the exact
entry state. `execute` preflights limits before reset/load.

At most 64 complete traces may be retained. Larger requested limits are
`TraceLimitExceeded` before allocation or mutation. `run` succeeds only after
the halt sentinel commits; reaching `max_steps` first is `StepLimitExceeded`
and rolls back the whole run.

Every committed `StepTrace` owns:

- pre-instruction PC and committed next PC;
- exact fetched bytes (`[u8; 3]` plus width 2 or 3) and the zero-extended raw
  instruction value;
- an exact uppercase opcode token such as `ADDI.N`, `CALL8`, or `LOOPNEZ`;
- any committed data-memory read/write address, width, and value; and
- the complete post-state.

Trace creation occurs after execution on scratch state but before commit, so
allocation failure cannot leak AR, window, loop, memory, PC, or halt changes.

## 9. Typed error taxonomy

The public error enum has stable, nonempty display text and distinguishes at
least:

- `EmptyProgram`, `ProgramOutOfRange`, and `ProgramAddressOverflow`;
- `NoProgramLoaded`, `FetchOutsideProgram`, `TruncatedInstruction`, and
  `FetchOutOfRange`;
- `UnknownInstruction { raw, width }` and
  `UnsupportedOptionInstruction { raw, width }`;
- `UndefinedOperation { raw, width, reason: UndefinedReason }`, where this
  cell's reasons are `InvalidEntrySource`, `ShiftAmountOutOfRange`, and
  `InvalidReturnWindow`;
- `UnsupportedSpecialRegister { number }`, `PrivilegedInstruction`, and
  `Breakpoint { code }`;
- `DataOutOfRange { address, width }`;
- `InvalidLoop`, `WindowOverflow { increment }`, and
  `WindowUnderflow { increment }`;
- `AllocaCause` for a `MOVSP` whose three caller-window availability bits are
  all clear;
- `InvalidState`;
- `Halted`, `TraceLimitExceeded`, `StepLimitExceeded`; and
- allocation failure for every fallible snapshot/trace path.

Decode errors contain the already fetched raw bytes but do not expose stale
bytes beyond the selected width. Errors never mutate state.

## 10. Validation matrix and completion gate

The implementation cell is incomplete until independent tests cover:

1. **Fetch/classification:** every `op0` width class; every included format;
   all fixed/reserved-bit neighbors; 16-bit/24-bit instructions starting at
   each byte residue; two-byte and three-byte image truncation; documented
   deferred-option versus unknown classification.
2. **Arithmetic/data:** zero, one, sign boundary, signed minimum/maximum,
   carry-discard, aliasing destinations, all immediate extrema, every SAR
   boundary including valid `32` and rejected `33..63`, signed/unsigned branch
   disagreements, all B4 constant-table entries used by immediate branches,
   and `SSA8B` source-low-bit witnesses `0/1/2/3 -> 32/24/16/8`.
3. **Memory:** every width, sign extension, little-endian byte order, aligned
   and unaligned access, final valid byte range, every partial-overrun shape,
   literal loads, self-modifying raw-byte capture, and store rollback.
4. **Control flow:** taken/not-taken witnesses for every branch relation, exact
   branch bases/scales, direct/indirect calls, return links, 1 GiB windowed
   return reconstruction, in-range and invalid targets, next-step target fetch
   faults, absence of delay slots, and reserved displacement rejection for
   every value `-3..0` with accepted `-4` and `1` neighbors.
5. **Physical windows:** all 64 physical ARs, every logical-to-physical mapping,
   wrap at window 15, increments 4/8/12, `ENTRY` caller-read/callee-write
   ordering, `RETW` clearing, `MOVSP`, deliberate hidden-window preservation,
   and atomic overflow/underflow at each increment. A table-driven check must
   cover every included AR-using opcode's actual `r/s/t` reference groups,
   groups `0/1/2/3`, wraparound, priority `1` before `2` before `3`, ignored
   non-reference fields, false conditional execution, and the exact
   `ENTRY WindowCheck(0, CALLINC, 0)` cases for `CALLINC = 0/1/2/3`.
6. **Loops:** zero/one/two/maximum counts, positive/zero/negative conditional
   setup, 16/24-bit loop bodies, first-instruction alignment boundaries,
   ordinary loopback, taken-branch early exit, `LCOUNT` decrement, replacement
   of an existing loop, and invalid-range rollback.
7. **Lifecycle:** deterministic reset, replacement load, restore validation,
   post-halt behavior, fault rollback after candidate register/window/loop/
   memory changes, whole-run rollback, whole-execute rollback, trace-limit and
   allocation-failure seams, and exact complete-state equality.
8. **Reserved/undefined precedence:** exhaustive `ENTRY s = 4..15`, every
   `EXTUI` pair on both sides of `sa + op2 = 31`, same/different-register
   `MOV.N`, same- and different-register `MOVSP`, all six-bit SAR values for
   each affected shift, and all RETW increments plus all eight combinations of
   the three preceding `WindowStart` bits. The `MOVSP` matrix must separately
   exhaust all eight combinations of
   `WindowStart[WindowBase - 3..WindowBase - 1]`: only `000` reports
   `AllocaCause`, while every other combination permits the move after the
   lazy check. It must include same-register self-moves and combined witnesses
   where a lazy `WindowOverflow` masks `AllocaCause`. Combined-fault cases must
   prove raw reserved decode before `WindowCheck`, `WindowOverflow` before
   `InvalidEntrySource`, `AllocaCause`, and the other
   `UndefinedOperation`/semantic memory/control checks, and atomic state
   preservation for every typed result.

Release line coverage for the package must exceed 80%. Completion additionally
requires Rustfmt, strict Clippy with warnings denied, rustdoc with warnings
denied, release unit/integration/doctests, repository BUILD integration,
lessons validation, and independent correctness/security review.

## 11. Ordered follow-ups

The bounded follow-up backlog is:

1. extend the functional oracle one publicly specified standard option at a
   time (multiply/divide and conditional store before MAC16/FPU), with state
   and encoding tests added in the same cell;
2. add exception/interrupt delivery only from a public configuration-specific
   vector/state contract, then compose a dual-core ESP32-class SoC with public
   memory aliases, cache/MMU boundary, interrupt routing, DMA, and peripherals;
3. build an educational gate-level implementation of this repository-owned
   bounded datapath and storage topology, differentially checked against the
   functional oracle; and
4. descend into timing/transistor/SPICE only after the repository's lower-level
   device work lands.

No later gate cell may claim to recover the proprietary LX6 netlist or physical
layout. A synthesized educational topology must say so explicitly and count
only storage and combinational networks it actually owns.
