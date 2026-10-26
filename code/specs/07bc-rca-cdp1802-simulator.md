# Layer 07bc — RCA CDP1802 Functional Simulator

**Status:** bounded functional implementation complete locally; independent
exact-head implementation review passed at
`e7224c60e1aa1125db77b77e05e60a2b724e26cf`; publication remains pending.
The exact reviewed spec-only head is
`9b59e29f4f87e1d3260facf3623d2c16c9e4c117`.

## 1. Status, sources, and profile

This specification defines a checked instruction-level model of the RCA
COSMAC CDP1802. It is a CPU and explicit bus-request oracle, not a COSMAC ELF,
VIP, spacecraft computer, or transistor recreation.

The exact spec-only commit passed independent review before implementation
began. The completed implementation then passed the mandatory different-
reviewer correctness/security audit recorded above.

Primary authority is Renesas/Intersil, *CDP1802AC/3 High-Reliability CMOS
8-Bit Microprocessor*, FN1441 revision 3.00, 17 October 2008,
<https://www.renesas.com/document/dst/cdp1802ac3-datasheet>. The document says
the A device is functionally identical to the original CDP1802 and provides
the register summary, reset/initialize behavior, complete Table 4 instruction
set, machine-state transitions, DMA, interrupt, flag, and I/O rules. The
archived RCA/Intersil publication mirror at
<https://www.cosmacelf.com/publications/data-sheets/cdp1802.pdf> is a
cross-check, not a substitute for the manufacturer-hosted source.

The immutable package profile is `Cdp1802Ac3FunctionalV1`:

| Property | Pinned value |
|---|---|
| Data width | 8 bits |
| Address width | 16 bits |
| Register matrix | sixteen 16-bit registers R0–RF |
| Program counter | the register selected by four-bit P |
| Data pointer | the register selected by four-bit X |
| Storage | one owned 65,536-byte memory |
| External CPU inputs | EF1–EF4, seven input-port bytes, DMA and interrupt through explicit calls |
| Instruction set | all FN1441 Table 4 encodings; 0x68 remains unknown |

No clock frequency, propagation delay, package, radiation, board, peripheral,
or electrical claim is part of this functional profile.

## 2. Complete authoritative state

`Cdp1802State` is a frozen owned value containing exactly:

- `registers: [u16; 16]`;
- `p: u8` and `x: u8`, each constrained to 0–15;
- accumulator `d: u8`, data flag `df: bool`, interrupt enable `ie: bool`,
  output flip-flop `q: bool`, and interrupt save register `t: u8`;
- sampled external flag inputs `ef: [bool; 4]`, where `true` means the
  asserted low/VSS condition used by Table 4;
- deterministic input bus values `input_ports: [u8; 7]`, corresponding to
  ports 1–7;
- all 65,536 memory bytes;
- installed-image `origin: Option<u16>` and `length: u32`, allowing a full
  65,536-byte image;
- `first_instruction_pending: bool`, the reset sequencer latch that suppresses
  interrupt recognition until one instruction has completed; and
- `idle: bool`.

There is no duplicate PC or data pointer: PC is always `registers[p]` and the
X pointer is always `registers[x]`. Instruction latches I/N and the internal B
temporary are execution details, not separately restorable architectural
state. Output-port writes and DMA-out bytes are transactions returned by the
operation that produced them, not invented persistent peripheral latches.

FN1441 reset leaves several programmer registers unspecified. For a
deterministic simulator, `new` and `reset` zero the complete register matrix,
D, DF, T, Q, memory, inputs, and image metadata; select P=X=0; set IE; and
clear idle. This contains the documented reset/initialize values and chooses
zero only for unspecified fields. Reset and successful load set
`first_instruction_pending`; DMA service leaves it set, every failed step
leaves it set, and the first successful instruction clears it after that
instruction completes. This includes IDL: entering idle completes the first
instruction and removes the inhibit. `restore` validates all ranges and image
invariants before committing.

## 3. Memory, installation, fetch, and traces

Memory is byte-addressed and naturally wraps modulo 2^16 for architectural
register arithmetic. `load(program, origin)` accepts a nonempty byte slice
only when the half-open host range fits without wrapping. It validates first,
performs deterministic reset, copies the bytes, records origin/length, places
origin in R0, and leaves P=X=0, IE=true,
`first_instruction_pending=true`, and idle=false.

Instruction fetch reads at `R(P)`, then increments that selected register
modulo 2^16 before execution. The opcode and every required following operand
byte must lie in the installed nonwrapping host image. Missing opcode is
`FetchOutsideProgram`; a missing branch/immediate byte after a fetched opcode
is `TruncatedInstruction`. Architectural data reads and writes use the full
owned memory independently of the installed image. Stores may therefore
self-modify installed code; traces retain the bytes fetched before mutation.

Every successful step trace contains PC/register selector before and after,
exact one/two/three raw bytes, stable uppercase mnemonic, D/DF/Q/IE before and
after, optional memory read/write, optional port transaction, branch/skip
decision, machine-cycle count, idle transition, and the complete post-state.

## 4. Exhaustive decode and data movement

The independent decoder oracle classifies all 256 first bytes. Every byte is
architectural except 0x68, which returns `UnknownInstruction { raw: 0x68 }`.
Aliases `SHRC/RSHR`, `SHLC/RSHL`, `NBR/SKP`, and `NLBR/LSKP` share one raw
encoding and one canonical trace mnemonic selected by the package docs.

The high-nibble register families implement the exact Table 4 relations:

- 0N is `IDL` for N=0 and `LDN` for N=1..15;
- 1N `INC`, 2N `DEC`, 4N `LDA`, 5N `STR`, 8N `GLO`, 9N `GHI`, AN `PLO`,
  BN `PHI`, DN `SEP`, and EN `SEX` cover all N;
- the 3N short-branch family consumes one address byte;
- the CN long-control family has one-byte skips/NOP and three-byte long
  branches exactly as Table 4 specifies; and
- 6N and 7N use their exact individually assigned opcodes.

`LDA`, `LDXA`, `STXD`, immediate forms, `RET`, `DIS`, `MARK`, `OUT`, and
`INP` preserve the datasheet's visible read/write/increment order. All work is
staged transactionally so aliasing such as P==X, N==P, X==2, or P==2 observes
the specified pre-state access followed by the specified wrapping update.

## 5. ALU and DF

OR, XOR, AND and their immediate forms affect D only. `SHR`/`SHL` shift in
zero and put the outgoing bit in DF. `SHRC`/`SHLC` shift the pre-state DF into
D and put the outgoing bit in DF.

All add operations produce the low byte in D and set DF on carry. `ADC` and
`ADCI` include pre-state DF. Subtract operations use the exact operand order:
`SD/SDB` compute memory minus D, while `SM/SMB` compute D minus memory;
immediate forms use the immediate byte in place of memory. Borrow forms
subtract `!DF`; result DF is one for no borrow and zero for borrow. No other
instruction changes DF. Independent exhaustive 256x256x2 oracles cover every
add/subtract family and both operand orders.

## 6. Branches, skips, selectors, and control

Short branches consume the byte at the post-opcode R(P). A taken branch
replaces only the low byte of R(P), retaining the high byte that exists after
opcode fetch; a false branch increments R(P) over the operand. Conditions are
D zero/nonzero, DF, Q, and each sampled EF input with the exact active-low
truth named in Table 4. BR is always taken; NBR/SKP always advances.

Long branches consume high byte then low byte. Taken branches replace the
complete selected R(P); false branches advance over both bytes. A taken long
skip performs the two S1 memory-read cycles shown by FN1441 Table 5, reading at
R(P) and incrementing after each read. A false long skip performs both reads at
the unchanged R(P). Those reads are real bus activity but the bytes are not
instruction operands: the CPU-only trace does not retain them and the
installed-image contract does not validate them. The later SoC cell exposes
the two ordinary memory-read strobes and values. LBR always loads, NLBR/LSKP
always performs the taken two-increment form, and C4 NOP consumes no operand
while retaining the documented extra execution cycle.

`SEP N` changes P after opcode fetch, so the next fetch uses R(N) without
modifying it. `SEX N` changes X. `SEQ`/`REQ` set/clear Q. `SAV` stores T at
M(R(X)). `MARK` forms T=(X<<4)|P, stores T at M(R2), then sets X to the old P
and decrements R2 modulo 2^16. `RET` and `DIS` read M(R(X)) into X/P, then
increment the same pre-state R(X) that supplied the saved selector byte; RET
sets IE and DIS clears IE.

Opcode 00 `IDL` is not a halt sentinel. It enters the documented repeating
idle state after its successful trace. Ordinary `step` while idle returns
`Idling` unchanged. A serviced DMA or eligible interrupt clears idle and
normal instruction execution resumes. `run(max_steps)` succeeds only on IDL;
budget, fetch, or other faults restore the entry state. There is no fabricated
unused-opcode halt because 0x68 must remain an observable unknown encoding.

## 7. I/O, DMA, interrupt, and SoC seam

`OUT 1..7` reads M(R(X)), returns a `PortWrite { port, value }` transaction,
then increments R(X). `INP 1..7` samples the corresponding configured input
byte once, writes it to M(R(X)), and copies it to D without incrementing R(X).
Changing EF or input-port values is an explicit validated environment action
and is included in snapshots; it never occurs invisibly during a step.

External bus service is exposed through typed atomic operations:

- `dma_in(byte)` writes byte to M(R0), increments R0, clears idle, and returns
  a DMA-in transaction;
- `dma_out()` reads M(R0), increments R0, clears idle, and returns the byte and
  a DMA-out transaction; and
- `interrupt()` requires IE=true and `first_instruction_pending=false`, sets
  T=(X<<4)|P, then X=2, P=1, IE=false, clears idle, and returns an
  interrupt-acknowledge transaction. The reset inhibit has first precedence:
  while it is set, the call returns `InterruptInhibited` unchanged regardless
  of IE, including after any number of pre-instruction DMA cycles. Once the
  inhibit is clear, IE=false returns `InterruptMasked` unchanged.

These calls model one acknowledged S2 or S3 cycle. Simultaneous-request
priority, WAIT/CLEAR pins, bus arbitration, devices, and elapsed clocks belong
to a later SoC scheduler. This explicit seam allows data movement and
interrupt routing without hidden callbacks.

## 8. Lifecycle and atomic faults

The checked API provides new/reset/load/snapshot/restore, memory helpers,
input configuration, step/run/execute, and explicit DMA/interrupt service.
`execute` is atomic across load and run. Retained complete-state traces are
bounded to 64 and all fallible allocations are preflighted.

Stable typed errors distinguish empty/oversized/overflowing image, no image,
fetch outside image, truncated instruction, unknown 0x68, invalid state,
idling, masked interrupt, reset-inhibited interrupt, trace limit, step limit,
allocation failure, and a source-preserving execution wrapper. Validation,
fetch, decode, all reads, transaction allocation, and candidate updates
precede commit. Every failure preserves registers, selectors, flags, inputs,
memory, image metadata, reset inhibit, and idle state exactly.

## 9. Completion gate

Implementation is incomplete until it provides:

1. an independent exhaustive 256-byte decoder classification and exact
   one/two/three-byte width table, including 0x68 and every alias;
2. every N/P/X equality combination relevant to register, memory, MARK,
   RET/DIS, SEP/SEX, OUT/INP, and immediate sequencing;
3. exhaustive ALU/DF equations, shift endpoints, and no-borrow polarity;
4. every short/long branch and skip condition true/false, all EF inputs,
   low-byte page retention, 0xFFFF wrapping, and truncation shape;
5. all seven port inputs/outputs, self-modifying traces, Q, IDL/resume,
   DMA-in/out, masked/inhibited/accepted interrupt, T packing, and R0 wrap,
   including reset/load inhibit, DMA before first fetch, first-step fault, and
   first-instruction IDL witnesses; an overlap regression must restore
   IE=false with the inhibit set and prove `InterruptInhibited` precedence;
6. reset/load/snapshot/restore/run/execute rollback and injected allocation
   failures with exact complete-state equality;
7. package docs, BUILD integration, lessons validation, release tests,
   Rustfmt, strict Clippy, rustdoc, affected builds, and greater than 80%
   release line coverage; and
8. a different reviewer’s exact-clean-head correctness/security PASS before
   publication.

## 10. Follow-ups

A bounded educational gate cell may implement the manufacturer-published
register-matrix, ALU, selector, decoder, Q/DF/IE storage, and bus-control
blocks from repository gates and DFFs, differential against this oracle. The
published block diagram is not transistor topology, so the model must remain
explicitly educational.

A later CDP1802 SoC cell will compose RAM/ROM, decoded ports, request
arbitration, DMA controllers, interrupt sources, clock/reset domains, and
deterministic bus transactions. COSMAC VIP/ELF video, keypad, and cassette
devices are separate board profiles. Transistor and analog timing descent
waits for the source-backed SPICE/device-model layer.
