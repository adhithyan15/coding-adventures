# Layer 07bc2 — RCA CDP1802 Educational Gate-Level Simulator

## 1. Status, purpose, and evidence boundary

**Status:** implementation contract complete locally; implementation must
follow in a later commit and receive an independent exact-head review before
publication.

This specification defines `cdp1802-gatelevel`, the architectural gate-level
companion to the merged `cdp1802-simulator`. Its primary source is the
Renesas/Intersil *CDP1802AC/3 High-Reliability CMOS 8-Bit Microprocessor*
data sheet, FN1441 revision 3.00, 17 October 2008. The archived RCA/Intersil
data sheet is a corroborating source. Normative instruction, reset, DMA,
interrupt, flag, I/O, and machine-state behavior remains the bounded functional
contract in Spec 07bc.

The manufacturer publishes an architectural block diagram, register matrix,
ALU behavior, machine cycles, and external bus protocol, but not the complete
physical transistor netlist of a particular die. This cell is therefore an
**educational architectural gate model**. It does not claim transistor
topology, propagation delay, radiation behavior, package pins, clock phase
timing, fabrication process, placement, routing, or board/SoC peripherals.
Those claims require separately sourced SPICE/device and SoC cells.

Production code must not depend on or invoke `cdp1802-simulator`. The
functional crate is a development-only differential oracle. Decode and
execution are independently implemented from the source-locked specification.

## 2. Complete stable-Q topology

Every mutable public-state bit has exactly one authoritative stable-Q location.
It changes only by clocking repository `logic-gates` D flip-flop/register
primitives. A packed byte may physically store eight settled Q bits for memory
scalability, but each changed byte must be clocked through eight repository
DFFs. Derived nonzero-address indexes, allocation seams, and temporary
candidate values are reconstructible host metadata, never state authority.

| Stable-Q bank | Width |
|---|---:|
| 65,536-byte memory | 524,288 |
| R0–RF register matrix | 256 |
| P and X selectors | 8 |
| D accumulator | 8 |
| DF, IE, and Q | 3 |
| T interrupt-save register | 8 |
| sampled EF1–EF4 inputs | 4 |
| seven sampled input-port bytes | 56 |
| installed-image present bit | 1 |
| installed-image origin | 16 |
| installed-image length, including 65,536 | 17 |
| reset interrupt-inhibit latch | 1 |
| IDL state latch | 1 |
| **Exact total** | **524,667 DFFs** |

Six primitive gates per master/slave DFF gives 3,148,002 storage-gate
equivalents before combinational decode, ALU, selection, control, and bus
networks. The count is derived from the actual owned storage containers and is
asserted structurally; a detached constant is not acceptable evidence.

Image metadata participates in checked lifecycle state even though it is not a
CDP1802 die register. It is included so snapshots, fetch bounds, rollback, and
the published inventory describe the same authoritative machine. There is no
duplicate PC: the fetch address is always the Q value of R(P). There is no
duplicate X pointer. Public snapshots are projections of Q, not writable
mirrors.

## 3. Independent gate transition engine

The engine reconstructs the complete accepted Spec 07bc boundary:

- a four-to-sixteen high-nibble decoder and low-nibble one-hot decoder classify
  all 256 first bytes; their conjunctions make opcode 0x68 the sole unknown
  encoding and route every other family;
- sixteen-way register selection and write-enable networks make the selected
  register matrix authoritative for fetch, P, X, SEP, SEX, MARK, RET/DIS,
  DMA, branches, and all N-register families;
- fixed eight-bit ripple adders, two's-complement subtraction networks,
  carry/no-borrow wires, Boolean gates, and one-stage shift/rotate muxes derive
  D and DF for every memory and immediate ALU family;
- gate equality/zero reductions and sampled Q/DF/EF/IE wires decide short and
  long branch/skip conditions, interrupt eligibility, and idle control;
- fixed sixteen-bit increment/decrement/add/mask/mux networks derive register
  sequencing, page-preserving short targets, full long targets, R0 DMA
  sequencing, and wrapping address changes;
- byte muxes and gate-derived enables route memory reads/writes, input/output
  ports, SAV/MARK, and exact pre-state alias ordering; and
- host control flow may orchestrate already-settled one-hot lines, fixed loop
  bounds, and candidate assembly, but must not replace an architecturally
  visible data result, predicate, selector, or write enable.

All 0N–FN families, individually assigned 6N/7N/CN operations, aliases,
one/two/three-byte widths, cycle counts, branch outcomes, memory effects,
ports, Q, IDL, DMA, and interrupts match Spec 07bc. The engine retains the
functional profile's deterministic reset choices and exact typed failures.

## 4. Clock and commit discipline

One checked instruction is evaluated against an owned value-level candidate.
Fetch, decode, operand availability, address selection, memory reads, gate
networks, trace allocation, and the complete post-state snapshot settle before
any architectural Q changes. Success clocks only the changed DFF banks, then
projects the committed trace. Failure leaves every DFF unchanged.

Reset, load, validated restore, direct register/memory writes, sampled EF/port
configuration, instruction commit, DMA-in/out, interrupt acknowledgement, run
rollback, and execute rollback all use the same DFF clock paths. No direct
assignment to packed Q storage is permitted. A derived changed-address index
may accelerate reset/restore, but tests must rebuild and deliberately perturb
that index without changing public behavior.

The reset-inhibit and IDL DFFs are sole authorities. DMA clears only IDL and
does not clear reset interrupt inhibition. The first successful instruction,
including IDL, clears the inhibit. Accepted interrupt clocks T=(X<<4)|P,
X=2, P=1, IE=0, and IDL=0 atomically. Masked and inhibited requests clock
nothing, with inhibition retaining precedence.

## 5. State, traces, and explicit bus transactions

The public state and error types are source-compatible projections of Spec
07bc: sixteen registers, P/X, D/DF/IE/Q/T, sampled EF and port input bytes,
complete memory, installed origin/length, reset inhibit, and IDL. Successful
instruction traces preserve exact raw bytes, width, canonical uppercase
mnemonic, selector/PC/D/DF/Q/IE/idle transitions, memory access, port
transaction, branch decision, cycles, and complete before/after states.

`dma_in`, `dma_out`, and `interrupt` remain explicit acknowledged
transactions. They model the CPU side of one request and do not invent a DMA
controller, arbitration, WAIT/CLEAR scheduler, peripheral latch, or elapsed
clock. This is the stable composition seam for a later CDP1802 SoC cell.

## 6. Typed atomic lifecycle and allocation

The public machine provides deterministic construction/reset, Q projection,
fallible projection, validated restore, checked image load, register/memory
access, sampled-input configuration, step, bounded run, atomic execute, DMA,
and interrupt service. Program fetch remains bounded to the installed
nonwrapping image while architectural data addresses wrap in sixteen bits.

All allocations needed for snapshots, candidates, complete traces, retained
run results, and rollback storage are fallibly preflighted. Deterministic
test-only allocation seams must cover step, run, and execute, including a
failure after an earlier instruction has committed inside the same run. The
same complete call-entry Q state must be restored without allocating.

Errors distinguish the Spec 07bc empty/range/image/fetch/truncation/unknown,
invalid-state/input, IDL, interrupt, retention, step-budget, and allocation
boundaries. Fault precedence and source-preserving execute errors match the
functional oracle exactly.

## 7. Differential and structural evidence

Completion requires all of the following:

1. Independently classify every first byte and verify exact mnemonic, width,
   cycles, and the unique 0x68 unknown boundary.
2. Compare exact success traces and complete post-state against
   `cdp1802-simulator` for all 255 accepted first bytes with deterministic
   operands, plus focused alias/P/X/N overlap, wrap, self-modification, branch,
   skip, immediate, and port cases.
3. Exhaustively prove the gate ALU against independent 256x256x2 add/subtract
   truth tables, both subtract operand orders, carry/no-borrow polarity, and
   all shift endpoints.
4. Differentially cover all seven ports, Q, IDL/resume, DMA-in/out, masked,
   inhibited, and accepted interrupts, T packing, first-instruction behavior,
   R0 wrap, restore, reset/load, direct access, faults, run, execute, rollback,
   and injected allocation failures.
5. Assert the 524,667-DFF inventory from owned containers, stable-Q sole
   authority, and actual repository DFF/gate routing.
6. Keep the functional oracle in dev-dependencies only and structurally reject
   production imports, calls, aliases, dead oracle routers, host arithmetic or
   predicate substitutions, direct Q writes, detached gate calls, and topology
   constants not derived from containers.
7. Provide at least five hostile **complete temporary crates**. Each mutation
   must first pass `cargo check --lib` in isolation and then be rejected by the
   structural validator. Fixtures cover oracle substitution, host ALU bypass,
   host predicate/decode bypass, direct state/Q write, and dead/detached gate
   decoys.
8. Pass release tests, Rustfmt, strict Clippy, rustdoc warnings-as-errors,
   package BUILD, workspace/affected integration, documentation, lesson and
   backlog checks, greater than 80% combined package line coverage, and an
   independent correctness/security review of the exact clean head.

## 8. Follow-on boundary

The next systems cell may compose this CPU with ROM/RAM, port decoders,
peripheral devices, interrupt sources, DMA controllers, request priority,
WAIT/CLEAR behavior, clock/reset domains, and deterministic transaction
movement. COSMAC VIP/ELF video, keypad, cassette, and spacecraft profiles are
separate configurations. Transistor, analog, and timing descent remains
blocked on source-backed SPICE/device models and never retroactively expands
this educational architectural gate claim.

## 9. Completion evidence

The completed local cell derives exactly 524,667 DFFs and 3,148,002 storage
gates from owned containers. Gate-derived high/low decoder conjunctions are
the sole legality and family authority, including the unique 0x68 rejection;
gate-derived settled wires likewise authorize branch, skip, and interrupt
decisions. Its 40 release tests plus one doctest cover every decode byte, exact
full traces and states for all 255 accepted transitions and focused alias,
P/X/N, wrap, self-modification, branch, skip, immediate, port, lifecycle, DMA,
interrupt, fault, run, execute, and allocation matrices. They also cover
exhaustive 256x256x2 add/subtract tuples, gate shifts, rollback after a first
successful private instruction, and eleven hostile full temporary crates that
compile before the validator rejects them. Rustfmt, strict Clippy, rustdoc
warnings-as-errors, and the affected BUILD are green. `cargo llvm-cov` reports
95.62% combined line coverage (1,442 of 1,508 lines). Publication still
requires an independent review of the exact clean reconciled head.
