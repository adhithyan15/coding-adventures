# Layer 07ax — ARM7TDMI Functional Simulator

## 1. Status and primary sources

This layer specifies a checked instruction-level model of the ARM7TDMI core.
Its normative sources are Arm's *ARM Architecture Reference Manual* for the
ARMv4T architecture, document DDI 0100, and the *ARM7TDMI Technical Reference
Manual*, document DDI 0029 revision G. Instruction encodings, banked registers,
ARM/Thumb state transitions, pipeline-visible PC values, flags, and memory
transfer rules come from those manuals rather than from a later Arm profile.

ARM7TDMI is a core, not a complete microcontroller. This first cell is a
coherent ARMv4T integer oracle with fixed teaching memory and deterministic
lifecycle behavior. It does not claim a particular cache, coprocessor, memory
controller, EmbeddedICE block, bus, interrupt controller, or SoC.

## 2. Complete architectural state

The simulator owns all 31 physical 32-bit general-purpose registers described
by ARMv4T: shared R0-R7; the normal and FIQ banks of R8-R12; the User/System,
FIQ, IRQ, Supervisor, Abort, and Undefined banks of R13-R14; and R15/PC. It
also owns CPSR and the five banked SPSRs for FIQ, IRQ, Supervisor, Abort, and
Undefined modes. User and System modes share the same unbanked register view
and have no SPSR.

CPSR exposes N, Z, C, V, I, F, T, and the five-bit mode field. The admitted
modes are User, FIQ, IRQ, Supervisor, Abort, Undefined, and System. Reserved
mode encodings are invalid restored state and invalid privileged status writes.
Changing mode changes only the visible bank; it never copies registers between
banks. ARM/Thumb state is represented solely by CPSR.T.

The complete state also owns every byte of flat 64 KiB memory, deterministic
halt, and the installed-program origin and byte length. Transport and data
access are fixed to little endian. Addresses and registers wrap at 32 bits,
but every actual fetch or transfer must fit owned memory without host indexing
wrap or truncation.

## 3. Included ARM instruction boundary

The ARM-state boundary includes these conditionally executed ARMv4T families:

- all sixteen data-processing operations with immediate and register Operand2,
  immediate shifts, register shifts, RRX, flag-setting forms, and documented
  PC/SPSR result behavior;
- `MUL`, `MLA`, `UMULL`, `UMLAL`, `SMULL`, and `SMLAL` with their flag forms;
- `LDR`, `STR`, `LDRB`, and `STRB` with immediate/register, pre/post-indexed,
  add/subtract, and writeback addressing;
- `LDRH`, `STRH`, `LDRSB`, and `LDRSH` with their ARMv4T immediate/register
  offset encodings;
- `LDM` and `STM` in IA, IB, DA, and DB forms, including empty-list rejection,
  checked writeback, PC load/store behavior, and ordinary current-bank lists;
- `B`, `BL`, and `BX`, including ARM-to-Thumb and Thumb-to-ARM exchange;
- `MRS` and `MSR` register/immediate forms for admitted CPSR/SPSR fields;
- `SWP` and `SWPB` as atomic single-core exchanges; and
- the exact canonical `SWI 0` encoding as the repository's deterministic halt
  sentinel rather than as architectural exception entry.

ARM coprocessor instructions, undefined encodings, halfword forms forbidden by
ARMv4T, privileged cache/test controls, and every other SWI return typed errors.
Condition `NV` is reserved in this ARMv4T cell and never aliases later-version
unconditional encodings.

## 4. Included Thumb instruction boundary

The Thumb-state boundary includes the complete ARMv4T Thumb-1 integer surface:

- immediate and register shifts, add/subtract forms, immediate move/compare,
  and the sixteen ALU operations;
- high-register add/compare/move and `BX`;
- PC-relative literal loads;
- register-offset and immediate-offset word, byte, halfword, signed-byte, and
  signed-halfword transfers;
- SP-relative loads/stores, PC/SP address generation, and SP adjustment;
- `PUSH`, `POP`, and `LDMIA`/`STMIA`, including LR/PC list extensions;
- every defined conditional branch, unconditional branch, and the two-halfword
  long branch-with-link sequence; and
- the exact canonical `SWI 0` encoding as the Thumb halt sentinel.

Thumb undefined/reserved encodings and nonzero SWIs return typed errors.
Later Thumb-2, BLX, BKPT, CPS, SETEND, and later architecture extensions are
outside this ARMv4T cell.

## 5. Conditions, arithmetic, and status registers

Every ARM instruction first evaluates its four-bit condition against CPSR.
A failed condition is a successful no-op that advances by four bytes, emits
the original instruction bytes and an uppercase `CONDITION_FAILED` mnemonic,
and changes no flags or registers. All fourteen useful condition senses are
centralized and tested. Thumb conditional branches use the same evaluator.

Arithmetic uses exact 32-bit wrapping results. Addition and subtraction carry,
borrow, and signed overflow follow ARM definitions. Logical flag-setting forms
take C from the shifter carry-out and preserve V. A zero shift amount, the
special LSR/ASR-by-32 forms, modulo-32 register shifts, RRX, and shifts by or
beyond 32 follow the ARMv4T tables rather than host shift behavior. Multiply
flag forms update their architecturally defined N/Z outputs. ARMv4 makes some
multiply carry/overflow outputs unpredictable: C is unpredictable for
`MULS`/`MLAS` and Thumb `MUL`, while C and V are unpredictable for flag-setting
long multiplies; `MULS`/`MLAS` leave V unaffected. This deterministic oracle
preserves every architecturally unpredictable C/V bit rather than inventing a
hardware value, documents that choice, and tests it explicitly.

MRS exposes only the selected admitted status register. MSR honors its encoded
field mask. User mode may write only the documented condition-code field;
privileged modes may write admitted control and flag fields after validating
the result. An MSR cannot change CPSR.T in this model—`BX` is the ordinary
state-exchange operation—and cannot select a reserved mode. SPSR access from
User/System returns `UnavailableSavedStatus` atomically.

An ARM data-processing instruction with `S=1` and `Rd=PC` restores CPSR from
the current mode's SPSR after validating that SPSR. Other writes to PC preserve
the current instruction-set state and apply their family-specific low-bit rule
defined below.

## 6. Pipeline-visible PC and state exchange

Although this is not a cycle model, operand reads preserve ARM7TDMI's
architectural pipeline observations. In ARM state, an ordinary read of R15 is
the executing instruction address plus eight. A store whose source is R15 uses
the documented plus-twelve value. PC-relative branch targets use plus eight.
The register-shifted Operand2 PC cases use the ARM7TDMI values specified for
their operand positions and are covered explicitly by tests.

In Thumb state, an ordinary PC read and branch base are the executing
halfword's address plus four. PC-relative literal/address generation aligns
that visible value down to four bytes where the encoding requires it. A Thumb
store cannot name PC as an ordinary source except where its instruction form
explicitly provides LR/PC list behavior.

`BX` sets CPSR.T from target bit zero and clears only that bit in the written
PC. A target whose low bits are `10` selects ARM state but is architecturally
unpredictable in ARMv4T, so this checked model rejects it atomically instead of
silently clearing bit one. The remaining `00`, `01`, and `11` patterns follow
the documented exchange rule, including BX executed from either state and a
Thumb BX whose own instruction address is two modulo four.

Other PC writes preserve CPSR.T but do not share one generic alignment rule.
In ARM state, `LDR` or `LDM` loading PC ignores loaded bits 1:0 and remains in
ARM state; ordinary ARM data-processing PC results use their documented ARM
alignment. In Thumb state, `POP {...,PC}` and ordinary Thumb PC writes ignore
bit zero and remain in Thumb state. Directly encoded branch displacements are
already aligned by construction. Tests cover every discarded low-bit pattern
without allowing an operation other than BX to exchange instruction state.

The two Thumb BL halfwords execute as separate architectural steps: the prefix
places its documented intermediate value in LR and the suffix forms the target
and return address. A suffix without a valid prefix-derived LR is still
interpreted by the architecture's register semantics; the simulator does not
invent hidden decoder state.

This model does not simulate fetch/decode/execute timing, pipeline refill
cycles, or bus cycles. Pipeline visibility here means only the programmer-
visible operand and link values above.

## 7. Memory and multiple transfers

Byte accesses need no alignment. Halfword accesses require two-byte alignment;
word and swap accesses require four-byte alignment. ARM7TDMI systems may wire
alignment handling differently, so this checked teaching profile deliberately
faults misaligned halfword/word accesses instead of implementing external-bus
rotation. Signed byte/halfword loads sign-extend; other narrow loads clear the
upper bits. Stores use the low source bits.

Every effective address is computed with 32-bit wrapping arithmetic and then
checked against the whole owned-memory transfer. Pre-index/post-index and
writeback ordering follow ARMv4T. Architecturally constrained base/destination
overlaps are either implemented exactly where ARMv4T defines them or rejected
as `UnpredictableInstruction`; this package never chooses one arbitrary result
for an architecturally unpredictable combination.

LDM, STM, PUSH, and POP validate the complete register list, full memory span,
PC alignment/state result, and writeback before the first read or write. Their
memory order follows ascending register number at ascending addresses. User-
bank transfer (`^`) and LDM exception-return list forms are excluded and return
`UnsupportedInstruction` without partial transfer.

## 8. Execution, exceptions, and atomicity

Normal ARM execution advances PC by four; normal Thumb execution advances it
by two. Branches and PC writes replace that result after all validation. This
core has no delay slots. SWP/SWPB validate source, destination, and the whole
memory access before atomically exchanging the value in the single-threaded
model.

Real reset, Undefined, SWI, Prefetch Abort, Data Abort, IRQ, and FIQ exception
entry, vector fetch, priority, and return are outside this first cell. The
banked registers and SPSRs are retained because they are programmer-visible
ARM7TDMI state and are needed for exact status/mode semantics. Unsupported
exception-causing instructions return typed errors rather than fabricating a
vector or partially changing CPSR/LR. The exact ARM and Thumb `SWI 0` sentinels
are repository-only successful halts.

Every instruction preflights decode legality, privilege, source status,
alignment, complete memory ranges, multi-register constraints, PC targets, and
all destination state before committing. A typed error preserves every
physical register, CPSR/SPSR bit, memory byte, PC, instruction state, halt bit,
and image field.

## 9. Transport, traces, and lifecycle

`new` and `reset` zero every register and memory byte, select Supervisor ARM
state with I and F set, clear halt and image metadata, and place PC at zero.
Reset is a deterministic lifecycle operation, not a timed model of the RESET
pin or vector fetch.

`load(program, origin, state)` accepts a nonempty contiguous image whose origin
and length obey the starting ARM/Thumb state's alignment and whose complete
range fits memory. A successful load replaces the entire state. Fetch width is
four bytes in ARM state and two in Thumb state; fetch must be aligned and wholly
inside the exact installed image. Terminal fetches never wrap or return zeros.

`restore` validates all physical banks, CPSR and SPSRs, mode, T state, PC
alignment, halt, image metadata, and exact memory length before committing.
The immutable snapshot exposes every physical register and status register,
not merely the currently visible bank.

Every successful `step` returns state and PC before and after, instruction-set
state, width, exact bytes captured before any self-modifying write, decoded raw
value, stable uppercase mnemonic, condition outcome, and complete post-state.
`step` is atomic. `run(max_steps)` and `execute(program, origin, state,
max_steps)` are transactional as a whole and preflight a maximum of 64 retained
complete-state traces. A halt trace succeeds once; a later step returns
`Halted` without mutation.

Typed errors distinguish invalid programs and restored states, reserved modes,
unavailable SPSR, privilege and unpredictable-instruction violations,
misaligned and out-of-range fetch/data/targets, fetch outside the installed
image, unknown and unsupported instructions, unsupported exceptions, halted
execution, excessive trace retention, and exhausted
budgets. Every error has stable nonempty display text.

## 10. Explicit exclusions and follow-up backlog

This cell excludes real exception entry/return sequencing, asynchronous IRQ and
FIQ delivery, coprocessors, EmbeddedICE/JTAG, debug state, breakpoints and
watchpoints, the multiplier's cycle timing, pipeline hazards/refill timing,
AMBA or other buses, caches, write buffers, MMU/MPU, memory wait states,
big-endian configuration, external abort signals, DMA, peripherals, clocks,
reset domains, power behavior, and all SoC integration.

Follow-up cells must separately add source-backed exception sequencing, a
repository-gate ARM7TDMI datapath/control model, and then an SoC composition
using the shared bus/interrupt/DMA primitives. The README and changelog must
call this a bounded ARMv4T integer core, not a complete ARM7TDMI-based chip.

## 11. Completion gate

The functional cell is complete only when it has:

1. table-driven classification of every admitted ARM and Thumb family and its
   reserved, coprocessor, privileged, later-architecture, and malformed
   neighbors;
2. witnesses for all ARM conditions, arithmetic flags, shifter boundaries,
   multiply widths, status masks, privilege, every register bank, mode change,
   SPSR restore, and all PC-visible pipeline cases;
3. ARM/Thumb `BX` over every low-bit pattern, instruction-state-preserving
   LDR/LDM/POP/data-processing PC writes, ARM branches/links, both Thumb BL
   halves, taken/not-taken control flow, and target-fault rollback;
4. every transfer width/sign/addressing mode, swap, multiple-transfer order,
   overlap rejection, writeback, memory boundary, and full rollback;
5. complete-state load, restore, halt, run, execute, trace-bound,
   malformed-image, fetch, exact self-modifying bytes, and transactional
   rollback tests;
6. release line coverage above 80 percent; and
7. scoped Rustfmt, strict Clippy, rustdoc, release tests, BUILD integration,
   lessons validation, latest-main ancestry, and independent exact-head review
   before publication.
