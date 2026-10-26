# Layer 07be — ZipCPU Functional Simulator

**Status:** `RCPU-085` is complete locally from the independently accepted
spec-only predecessor; publication awaits exact-clean-head implementation
review. `RCPU-086`, the dependent gate-level cell, begins only after the
functional cell merges.

## 1. Authority, provenance, and bounded profile

This layer defines a checked instruction-level model of one explicit ZipCPU
configuration. It is a CPU and ordered memory-transaction oracle, not a
ZipSystem, board, cache- or pipeline-timing model, Wishbone/AXI fabric,
debugger, FPGA implementation, transistor netlist, or proprietary silicon
claim.

The normative public architectural source is Gisselquist Technology,
*ZipCPU Specification*, revision 3.0, 26 June 2023, specifically sections
3.1–3.3 and tables 3.1–3.13. The immutable copy is
[`doc/spec.pdf`](https://github.com/ZipCPU/zipcpu/blob/42606d2d6ef55df313772232977621b2d72f0159/doc/spec.pdf)
at upstream commit
[`42606d2d6ef55df313772232977621b2d72f0159`](https://github.com/ZipCPU/zipcpu/tree/42606d2d6ef55df313772232977621b2d72f0159),
whose Git tree is `2231731472f82cf383680a4be1b25ec477e718eb`,
with SHA-256
`b74db787e2b0dd5f890ddaa50c770afbef44ab81380d5463c55a08c6c588aa0d`.
The author's official ISA overview,
<https://zipcpu.com/zipcpu/2018/01/01/zipcpu-isa.html>, is an explanatory
cross-check. The pinned PDF controls architectural intent. When its prose is
silent or internally ambiguous about a configuration-dependent edge, this
spec names the pinned RTL behavior explicitly and requires the discrepancy to
be recorded rather than silently choosing a host-language result.

The same immutable upstream commit provides open RTL and an independently
written C++ instruction simulator. The later educational gate cell uses the
RTL only as published structural evidence. Functional tests may reproduce
bounded vectors separately from both RTL and `sim/cpp/zsim.cpp`; neither is
linked into production code or required at build, test, or run time. The
pinned source hashes are:

| Evidence file | SHA-256 |
|---|---|
| `rtl/core/zipcore.v` | `b0db25e2017c2e6cb6715d8901806c3bdd12ae98879e0723ff5b58b750c7938e` |
| `rtl/core/idecode.v` | `f8e29e5e5d387a7fefc73dd456448bcbd41d65ed4d80e5d23d8541ae4e507c4e` |
| `rtl/core/cpuops.v` | `54707430127001eb7e716eb14ff3b733ecfeac437a4a1cec6906bb7befa48a7e` |
| `rtl/core/div.v` | `ca220c1db020a4b3365d8c29fe9cdb24dfbc93a5db82827cb818b888d834abf5` |
| `rtl/core/mpyop.v` | `2b831b42a8b14671925c2284ce2e838378f3d738b1293eca74ede4c861aed5a4` |
| `rtl/core/slowmpy.v` | `449527da756405fab2b30ad7121609a0a0d7555c7fba68a6d0c81eb0f90502d4` |
| `rtl/core/memops.v` | `b2d0f99a20263acc860d91c835f05df6ad0b2713a4a03ac1acb96da47946ed61` |
| `sim/cpp/zsim.cpp` | `cecb8e72d741c81a69e68982673021d273259da0d680dfce5158b881b9374e1b` |
| `sim/cpp/twoc.cpp` | `55466d21033429bd06cc28a075c96588db5e07973a04284652d7f7e6a521764a` |
| `sim/cpp/twoc.h` | `32110672ffd716f340da93ac2532ba485a64cd0f147d1f94798db938b723b1cb` |
| `sim/cpp/zipelf.h` | `4cecdc651d2d7f5235f10fe19fb2ac361e304b096dec7fac5a3e5ba5351e7bfa` |

`zsim.cpp` is independent evidence, not a perfect executable specification.
It enables a broader historical option set, derives some flags differently
from current RTL, and expresses signed division with host C++ arithmetic.
Therefore its corpus covers only common well-defined V1 cases. Exact overflow,
shift, PC, and configuration edges come from the manual-derived oracle plus
the pinned RTL corpus; every disagreement is checked in and classified.

The immutable configuration is `ZipCpuRev3NoCisFunctionalV1`:

| Property | Pinned value |
|---|---|
| Register/address/instruction width | 32 bits |
| Byte order | big-endian instructions and data |
| Register banks | supervisor and user, sixteen 32-bit registers each |
| Reset address | `0x0000_0000` |
| Owned storage | 65,536 bytes at `0x0000_0000..=0x0000_FFFF` |
| User mode | enabled |
| Multiply | enabled, source-backed slow shift/add configuration (`OPT_MPY=36`) |
| Divide and arbitrary shifts | enabled |
| Compressed ISA | disabled (`OPT_CIS=0`); bit 31 set is illegal |
| Bus lock and simulator opcodes | disabled (`OPT_LOCK=0`, `OPT_SIM=0`) |
| Alignment checking | enabled: word 4-byte, halfword 2-byte, byte unrestricted |
| FPU | absent |
| Start/debug behavior | `OPT_START_HALTED=0`, debug port and external halt disabled |
| Other disabled structures | caches, MMU, local peripherals, profiler, trace port, clock gating |
| RTL evidence controls | `ADDRESS_WIDTH=30`, `OPT_PIPELINED=1`, `OPT_PIPELINED_BUS_ACCESS=1`, `OPT_EARLY_BRANCHING=1`, `OPT_DISTRIBUTED_REGS=1`, `OPT_LOWPOWER=0` |
| Evidence responder | 32-bit big-endian request port, alignment errors enabled, deterministic 64-KiB owned target |

The chosen multiply implementation changes elapsed clocks, not architectural
results. Timing is excluded. The bounded memory is a testable bus responder,
not a claim that the CPU's 32-bit uniform address space is only 64 KiB.

## 2. Complete authoritative state

`ZipCpuState` is an owned frozen value containing exactly:

- two banks of R0–R13, represented without aliases;
- supervisor and user PC (R15), each a 32-bit byte address;
- canonical supervisor and user CC (R14) values, whose complete 32-bit read
  projections include each bank's software-preserved reserved bits 22:16 and
  the immutable fields described below;
- active mode, `Supervisor` or `User`;
- the sampled external interrupt input;
- all 65,536 memory bytes; and
- installed-image `origin: Option<u32>` and `length: u32`.

R13 is only the ABI stack-pointer convention, R12 the frame-pointer
convention, and R0 the link-register convention; none has hidden hardware
semantics. R14/R15 are not duplicated in a generic array. The active bank is
selected by mode, while supervisor `MOV` may explicitly address either bank.

The complete CC read contract follows the pinned `zipcore.v` construction
`{w_cpu_info, stored_cc[22:16], w_[ui]flags}`. Bits 31:23 are the immutable
profile identity `111010110` (`0xeb00_0000` in place): CPU-present is one,
multiply and divide are present, FPU is absent, the core is pipelined, bit 26
is zero, early branching and pipelined bus access are enabled, and CIS is
absent. Bits 22:16 are separate supervisor/user software-readable and
software-writable storage. They are architectural state and survive ordinary
instruction execution, snapshot/restore, faults, bank switches, and interrupt
transitions rather than being reconstructed from configuration.

The complete low CC contract is: bits 0–3 are per-bank Z/C/N/V; bit 4 is one
shared SLEEP state visible through either CC; bit 5 is the bank-select/GIE
value; bit 6 is user STEP and reads zero in supervisor CC and while user mode
is active; bit 7 is supervisor break-enable or user break status; bits 8–12 are
illegal/trap/bus-error/divide-zero/FPU status; bit 13 is permanently zero
because CIS is absent; bits 14–15 are write-only cache-clear commands and
always read zero. Supervisor CC always reads GIE clear and user CC always
reads GIE set. The trap indication is shared as the manual requires. FPU
status remains zero in this no-FPU profile. A write to CC stores bits 22:16 in
the selected bank, ignores attempts to alter bits 31:23, and filters the low
fields by current privilege and these fixed/read-only rules before commit;
cache-clear bits are accepted no-ops because caches are absent. User software
cannot create supervisor SLEEP: clearing GIE wins over setting SLEEP.

CPU writes may replace either bank's four flags. Supervisor writes may set
break-enable and user STEP, and may retain or clear existing user
break/illegal/bus/divide/trap status but may not manufacture those status
bits. User writes cannot set STEP or exception status; clearing GIE sets trap.
Returning to user clears trap and the user break/illegal/bus/divide status.
Supervisor illegal/bus/divide status is sticky until reset because the debug
port that could clear it is absent from V1. These rules, shared SLEEP/trap,
and the mode-derived GIE values are canonical restore invariants.

The CPU has no separate architectural halt latch in this profile. Supervisor
SLEEP is the HALT state; user SLEEP is WAIT. The sampled interrupt is part of
state so snapshots and replay are deterministic. `new` chooses zero for all
unspecified power-on registers, both banks' writable CC fields including bits
22:16, both PCs, memory, and the interrupt sample, selects supervisor mode,
and exposes the immutable CPU-info projection on CC reads. `reset` restores
that CPU state, including both PCs and both reserved fields to zero, while
preserving memory and valid installed-image metadata. A successful `load`
first establishes the same deterministic reset state, then clears memory,
installs the image, sets supervisor PC to `origin`, and leaves user PC zero.
`restore` validates the entire candidate before committing: both PCs are
word-aligned, mode agrees with the active-bank rule, each CC has the exact
`0xeb00_0000` CPU-info projection, bits 15:13 have their canonical read-zero
values, its remaining fixed low fields and the shared trap value are
canonical, and image metadata is either `(None, 0)` or a nonempty aligned
nonwrapping span wholly inside memory. Snapshots and complete-state equality
include both banks' reserved fields.

## 3. Transport, memory, and exact transactions

Programs are raw big-endian 32-bit words. `load(program, origin)` requires a
nonempty byte slice, word-aligned origin and length, and a nonwrapping
half-open range wholly within owned memory. It validates and preflights all
allocations first, resets deterministic state, clears memory, installs the
bytes, records metadata, and sets supervisor PC to origin.

Fetch requires a word-aligned PC. A four-byte request in owned memory must be
wholly inside the installed image. Data accesses use the complete owned
memory independently of the image: LW/SW require four-byte alignment, LH/SH
two-byte alignment, and LB/SB have no alignment restriction. Loads are
zero-extending, as table 3.3 specifies. Effective addresses wrap modulo
2^32 only while the 32-bit operand expression is formed; the resulting whole
access is then range- and alignment-checked before any byte is observed or
written. Self-modifying stores affect later fetches, while the current trace
retains the original four bytes captured before execution.

Every successful step records an ordered instruction-read transaction and
any data transaction with address, width, big-endian value, byte-enable mask,
read/write direction, and success or modeled bus-error outcome. There are no
implicit callbacks. The 64-KiB responder deliberately reports an
architectural bus error for an aligned data request whose whole span is
outside owned memory. A misaligned data request also follows the pinned RTL's
alignment-error path and is the same architectural data-bus-error transition.
An instruction request outside owned memory is an architectural illegal-fetch
transition; an in-memory request outside the installed image remains the
typed host-contract `FetchOutsideProgram` error so bounded programs cannot
silently execute zero-filled bytes.

## 4. Decode and operand formation

The decoder classifies every 32-bit word independently of execution. With CIS
disabled, bit 31 set is illegal before any 16-bit reinterpretation. For the
standard format, bits 30:27 name the destination/A register, bits 26:22 are
the five-bit opcode, bits 21:19 select one of eight conditions, and the
remaining field forms operand B. If bit 18 is clear, B is a sign-extended
18-bit immediate. If set, B is the selected register plus a sign-extended
14-bit immediate. When the B register is PC, the immediate is multiplied by
four before addition. All operand values and condition flags are sampled from
pre-state.

`MOV` (0x0D) instead uses bit 18 as destination-bank A, bit 13 as source-bank
B, and a register plus signed 13-bit offset. Supervisor mode honors each
selector independently; user mode ignores them and remains confined to its
bank. LDI (`opcode & 0x1e == 0x18`) is unconditional and sign-extends its
23-bit immediate. The special group uses destination 14 or 15 (the high three
destination bits all one) with opcodes 0x1C–0x1F. BREAK and NOOP ignore their
documented payload; LOCK is illegal because the option is absent and SIM is
illegal because hardware simulation support is absent. Opcodes 0x1A–0x1F
with ordinary R0–R13 destinations are absent FPU operations and are illegal.
Opcodes 0x1A/0x1B with destination 14/15 are the two unassigned special forms
identified by the pinned decoder and are illegal rather than aliases.

The complete admitted opcode surface is:

- ALU: SUB, AND, ADD, OR, XOR, LSR, LSL, ASR, BREV, LDILO, MPYUHI,
  MPYSHI, MPY, MOV, DIVU, DIVS, CMP, and TST;
- memory: LW, SW, LH, SH, LB, and SB;
- unconditional immediate LDI; and
- the enabled special BREAK and NOOP forms.

Conditions are exact table 3.4 functions of pre-state flags: always, Z, N
(`.LT`), C, V, !Z, !N (`.GE`), and !C. A false condition performs no register,
flag, memory, mode, or PC-target side effect beyond the already captured
sequential PC advance. LDI and every special form ignore the condition field.
Legality, option, and reserved-field validation precede condition evaluation:
a conditionally false illegal encoding is still illegal.

## 5. Integer, flag, PC, and bank semantics

ADD produces unsigned carry in C; SUB/CMP set C on unsigned borrow, matching
the pinned implementation's `{0,A}-{0,B}` carry-out convention. Both compute
exact signed overflow V. Z tests the 32-bit result. For ADD/SUB/CMP, N is the
result sign XOR V, which is the relation consumed by the published `.LT` and
`.GE` conditions; this pinned RTL behavior resolves the prose table's shorter
description of N as a sign indication. CMP discards the result. AND/OR/XOR/TST
set Z/N from the 32-bit result and clear C/V; TST discards the result.

Shifts consume the complete unsigned operand B rather than masking it. Count
zero preserves A and clears C. Counts 1–31 shift normally and C receives the
last displaced bit. At count 32, LSR/LSL yield zero and C receives the original
bit 31/bit 0 respectively; greater counts yield zero and clear C. ASR at any
count at least 32 yields all sign bits and C equals the original sign. LSR and
LSL set V when the result sign differs from the input sign; ASR clears V. Z/N
reflect the result. BREV bit-reverses operand B. LDILO replaces only the
destination's low 16 bits and does not set flags. MOV and LDI do not set flags.

MPY returns the low 32 product bits; MPYUHI the upper half of an unsigned
64-bit product; MPYSHI the upper half of a signed 64-bit product. DIVU uses
unsigned numerator A and divisor B. DIVS uses signed two's-complement inputs
and truncates toward zero; `0x8000_0000 / 0xffff_ffff` wraps to
`0x8000_0000`, as locked by the pinned divider rather than host signed
division. Multiply sets Z/N from its selected 32-bit result and clears C/V.
Successful divide sets Z/N from its quotient, V clear, and C precisely when
the division has zero remainder, following `div.v`. Divide by zero writes no
destination and enters the architectural exception transition in section 7.

Unconditional ALU operations update Z/C/N/V except BREV, LDILO, and MOV as
identified by the pinned decoder. An ordinary conditional instruction does
not update flags, even when its condition is true. Executed conditional CMP
or TST does update flags. No instruction targeting CC or PC also writes the
ordinary Z/C/N/V result. The active PC reads as the current instruction
address plus four; an inactive-bank PC read through supervisor MOV reads that
bank's retained value without the pipeline offset. Any PC register used as B
still scales its encoded immediate by four. A destination of PC changes
control flow after validating only the current instruction's operands and
effects and clears result bits 1:0, matching the pinned PC write path. It does
not synchronously probe the destination against owned memory or the installed
image. The PC write commits, and only the following step applies ordinary
fetch classification at that address. A non-PC instruction advances the
active PC by four.
Same-register cases, both PC-as-source forms, CC-as-source or destination, MOV
across banks, and destination-bank/mode changes capture all inputs before any
write.

Writes to the active CC implement the filtered status rules in section 2.
Setting GIE from supervisor enters user mode and resumes the retained user PC
only when the sampled interrupt input is clear; with an asserted input the
release is suppressed. Clearing GIE from user mode sets the shared trap
indication and resumes the retained supervisor PC. The derived RTU, TRAP,
WAIT, HALT, branch, call, return, and push/pop assembler spellings are tested
only as sequences of real admitted instructions; they are not extra decoder
cases.

## 6. Memory and control overlap rules

LW/LH/LB read from operand B then write A; SW/SH/SB capture A before address
evaluation and store its low 32/16/8 bits. If a successful load targets PC,
its loaded value becomes the next control target with bits 1:0 cleared. If it
targets CC, filtering and a resulting mode switch occur only after the memory
transaction succeeds. Stores using active PC observe instruction-address plus
four; inactive PC and either CC use their captured architectural pre-state.
All register/bank/source/destination/address equalities are defined by this
read-before-write ordering.

The trace preserves exact sequential and selected PCs, condition outcome,
source bank/index/value tuples, destination bank/index/value when written,
Z/C/N/V before and after, CC/mode before and after, full raw bytes and word,
stable uppercase mnemonic, ordered memory transactions, exception or
interrupt cause, terminal reason, and the complete post-state. No trace field
is reconstructed after self-modification or a mode change.

## 7. Interrupts, architectural faults, and terminal boundaries

`set_interrupt(asserted)` is an explicit atomic environment action. An
asserted interrupt is eligible only in user mode and is sampled before fetch.
Acceptance switches to supervisor mode without a vector, preserving both
banks and causing execution to continue at the retained supervisor PC. User
SLEEP/WAIT remains quiescent while the input is clear and accepts the same
transition when asserted. Supervisor mode cannot be interrupted.

User illegal instruction/fetch, data bus error, divide-by-zero, BREAK, or
software trap records its exact documented user-CC status and switches to the
retained supervisor PC; a completed instruction under user STEP also switches
without inventing a new exception status. No destination or data write from a
faulting instruction commits. A normal accepted interrupt sets no fault bit.
Supervisor illegal instruction/fetch, data bus error, divide-by-zero, or
BREAK records the corresponding supervisor status where defined and emits
`StopReason::ExternalBreak`. Because external-break handling belongs to the
excluded wrapper, V1 stops at this CPU output rather than inventing reboot or
debug-halt behavior. A user BREAK switches to supervisor and sets user-break;
if supervisor break-enable was already set it additionally emits the same
external-break terminal after the bank transition.

Writing supervisor SLEEP produces successful `StopReason::Halted`; a later
ordinary step returns typed `Halted` unchanged. Writing user SLEEP produces
successful `StopReason::WaitingForInterrupt`; a step with no asserted input
returns typed `WaitingForInterrupt` unchanged, while an asserted input is an
accepted interrupt transition. These are CC-derived states, not duplicate
host booleans.

Fault/decision priority at an instruction boundary is fixed:

1. validate entry state, then preflight required allocations;
2. service an eligible external interrupt, including waking user WAIT;
3. reject supervisor HALT or unawakened user WAIT without mutation;
4. require an installed image, validate aligned fetch and owned range, then
   validate the full four bytes against the image;
5. capture raw bytes, classify bit 31/opcode/options/reserved fields, and only
   then evaluate the condition;
6. capture operands, form any taken PC value with bits 1:0 cleared, and
   validate only a complete data access performed by the current instruction;
7. select one architectural interrupt/fault/normal transition, construct all
   effects and the trace, preflight final retention, and commit once.

Thus transport truncation outranks decode, illegal/reserved encoding outranks
a false condition, and a false legal memory/control instruction performs no
address/alignment/range check. For an executed memory operation, alignment
outranks responder range; either maps to the same architectural bus-error
class but the trace retains the exact failed request. No lower-priority
destination or flag effect leaks into a fault transition.

A legal taken PC write commits without probing its destination. The next
step alone reports `FetchOutsideProgram` when the aligned target is within
owned memory but outside the installed image, or performs the architectural
illegal-fetch/bus transition when the target lies outside owned memory. Thus
`step` exposes the committed transfer before that later classification. A
multi-step `run` still obeys its own all-or-nothing host-error contract if the
following fetch becomes a typed `FetchOutsideProgram`.

## 8. Checked lifecycle, errors, and allocation seams

The public checked API provides `new`, `reset`, `load`, `snapshot`, `restore`,
validated byte/halfword/word memory helpers, `set_interrupt`, `step`, `run`,
and `execute`. Retained complete-state traces are bounded to 64. `run` stops
successfully on HALT, WAIT, external break, or an architectural switch to
supervisor caused by interrupt/fault; exhaustion of a nonzero budget is an
error and restores the exact call-entry state. `execute` is atomic across
load and run. Repeated `step` is the API for callers that want to continue a
supervisor handler after a successful user transition.

Stable typed host errors distinguish empty/unaligned/oversized/overflowing or
out-of-range images, no image, malformed restored state, halted, waiting,
fetch outside image, trace limit, zero/exhausted step budget, allocation
failure, and a source-preserving execute wrapper. The standalone decoder also
returns typed unknown/unsupported/reserved classifications without state;
`step`, `run`, and `execute` translate those classifications into the
architectural illegal-instruction transition. Architectural illegal
instruction, bus error, divide-zero, BREAK, trap, and interrupt are successful
state transitions and are not misreported as host errors.

All production allocations are fallible and have deterministic test-only
seams at meaningful boundaries: state/memory clone, load copy, snapshot and
restore staging, fetched-byte retention, mnemonic/effect construction, trace
construction and retention, run result/final-state construction, and execute
wrapping. A seam after at least one successful candidate instruction exists
in both `run` and `execute`. Every typed failure preserves all 32 register
projections, both banks' seven software-preserved CC bits, every dynamic low
CC field, mode, interrupt input, memory, image metadata, and terminal
condition bit-for-bit. Tests must prove that each named seam reaches
a real production allocation or narrow the contract before implementation;
dead test-only counters are forbidden.

## 9. Independent decode, execution, and lifecycle evidence

Production code must not call, translate, or embed upstream RTL or `zsim`.
Completion requires a manual-derived independent encoder/classifier and two
separately reproducible checked-in corpora: one from the pinned C++ simulator
and one from the pinned RTL under the exact V1 parameters. Generators record
tool versions, commands, source hashes, configuration, and any classified
manual/model discrepancy. The PDF remains normative for architectural intent;
the expressly named RTL resolutions in this spec control V1 configuration
edges and the later topology.

The required matrices are:

1. all 32 primary opcode values, both bit-31 classes, every destination
   field including special 14/15, all eight conditions, immediate/register
   selector forms, every reserved/special/FPU/disabled-option neighbor, and
   exact normal/MOV/LDI/special width and field extraction;
2. every admitted instruction with immediate minima/maxima, R0/R13/CC/PC,
   same-source/destination, cross-bank MOV selectors in both modes, PC-scaled
   immediate, condition true/false, and false-illegal precedence;
3. exhaustive or mathematically complete Z/C/N/V relations for add/sub/CMP,
   logic/TST, shift counts 0/1/31/32/63, BREV, LDILO, all three multiply
   selections, signed/unsigned divide extrema and zero;
4. byte/half/word big-endian loads and stores, every alignment residue,
   memory endpoints, out-of-range responder faults, PC/CC destinations,
   all overlap/read-before-write cases, and self-modifying trace retention;
5. every CC writable/read-only/fixed bit in supervisor and user mode, the
   exact immutable `0xeb00_0000` CPU-info projection, exhaustive values and
   bank independence for software-preserved bits 22:16, their reset/load/
   snapshot/restore/fault preservation, RTU, trap, WAIT, HALT, STEP, shared
   trap clearing, all fault bits, BREAK-enable, accepted/masked interrupt, and
   retained-bank PC behavior;
6. instruction PC at image and owned-memory boundaries, PC as A/B, taken and
   untaken conditional PC writes, low-bit clearing, modulo operand formation,
   committed out-of-image and out-of-memory targets followed by next-step
   fetch classification, fetch bus error versus fetch-outside-image, and
   terminal transitions;
7. exact full-state and ordered-transaction comparison for every common case
   against both upstream corpora, while manual-only host lifecycle errors and
   wrapper-bound external-break behavior use independent hand oracles; and
8. new/reset/load/snapshot/restore/memory helper/interrupt/step/run/execute,
   malformed state, every typed fault, every architectural fault, trace and
   step limits, every real allocation seam, and post-progress run/execute
   rollback with exact complete-state equality.

The implementation must source-lock corpus hashes and reproduce them without
network access. Corpus generation may require Verilator or upstream C++ tools
only in a documented developer command; ordinary package tests consume the
checked-in bounded data.

## 10. Completion gate and excluded work

Implementation is incomplete until:

1. this independently accepted spec-only commit precedes every implementation
   commit;
2. one package owns the complete checked state, independent decoder,
   execution, transactions, typed faults, and effective allocation seams;
3. all section 9 matrices pass in release mode with stable counts and exact
   reproduction documentation;
4. Rustfmt, strict Clippy (`-D warnings`), rustdoc (`-D warnings`), package and
   affected BUILD targets, lessons validation, and greater than 80% measured
   release line coverage pass;
5. package README, changelog, workspace/BUILD wiring, and backlog report exact
   observed tests, corpus rows, and coverage; and
6. a different reviewer gives an exact-clean-head correctness/security PASS
   before publication.

V1 excludes CIS, LOCK atomic sequences, SIM operations, floating point,
caches, MMU, local bus/peripherals, interrupt controller policy, debug port,
external halt/reboot wrapper, profiling/trace hardware, DMA, arbitration,
Wishbone/AXI stalls/retries, pipelines and elapsed clocks. Their encodings or
signals are rejected or terminated exactly as defined above, never silently
implemented as another operation.

`RCPU-086` begins only after `RCPU-085` merges. It is a source-backed
**educational** gate-level model of this exact profile, using the pinned open
RTL to bound two register banks, CC/mode/PC storage, independent instruction
decode, operand muxes, ALU/flags/shifter, slow multiplier/divider, memory
request control, interrupt/fault transitions, and stable-Q transactional
state. It must fully differentially match this oracle and include
compiler-valid structural anti-bypass tests. Open synthesizable RTL is not a
transistor schematic, physical layout, timing model, or proof of any FPGA's
placed circuit; the gate cell must make none of those claims. A later SoC cell
may compose an interrupt controller, memory map, buses, DMA, and peripherals.
Transistor/SPICE descent remains blocked on the separate device-model wave.
