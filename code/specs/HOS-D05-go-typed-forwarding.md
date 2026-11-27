# HOS-D05-GO1 — Typed RV32I operand forwarding in the Go pipeline

## Purpose

The published Go D05 core already runs the repository's RV32I decoder through
the reusable five-stage pipeline. The generic hazard path can report a scalar
forward, but the RISC-V executor reads both operands from the committed
register file. The decode token's `ALUResult` and `ForwardedFrom` annotations
do not supply the selected value to `rs1` or `rs2`. A dependent instruction can
therefore consume stale state even while the trace reports a forward.

This prerequisite makes the existing EX/MEM hazard decisions available as
owned, source-tagged operand values to the RISC-V execution callback. The
existing scalar pipeline API and its behavior for consumers that do not opt
into typed operands remain unchanged. The first consumer is the Go RV32I core.

## Decision and precedence

For each decoded source slot, retain the source position, source register,
selected value, and source stage (`EX` or `MEM`). The hazard detector continues
to decide the pipeline action once per cycle. For each source independently,
the producer in EX wins over an older matching producer in MEM. An EX load-use
match requests the existing stall and cannot fall back to an older MEM value;
the source is reconsidered on the next cycle, when the load result is in MEM.
Only a MEM result already produced by its memory callback may forward. Writes
to RISC-V x0 are not producers, and reads of x0 remain zero. Two operands may
carry separate values and source-stage tags in one response.

The typed records are copied when attached to a pipeline token and when a token
is cloned into a snapshot. Existing `ForwardValue`, `ForwardSource`,
`ForwardedFrom`, and scalar forwarding actions remain available to existing
generic pipeline callbacks. The RISC-V decoder uses typed records for `rs1` and
`rs2` before arithmetic, branch comparison, address calculation, or store-data
selection; the generic pipeline does not interpret instruction operands.

## Bounded acceptance evidence

Use real RV32I instruction words on `core.Core` configured with the classic
five stages and the existing hazard detector. A dependent guest must exercise:

- simultaneous distinct source forwarding, including `rs1` from MEM and
  `rs2` from EX;
- a younger EX write winning over an older MEM write to the same register;
- one load-use stall followed by the load value forwarding from MEM;
- a taken branch consuming forwarded values and flushing its wrong path;
- a store whose base and data operands are forwarded, with the expected word
  visible in flat RAM; and
- x0 reads and writes without a forwarded nonzero value.

Compare the final architectural registers, word memory, and retired instruction
PC sequence with a separately instantiated sequential `RiscVSimulator` running
the same raw little-endian bytes. Inspect stage snapshots and assert the tags,
values, stalls, and flushes directly. Also test the generic pipeline's legacy
scalar callback behavior and defensive typed-record copying. This is a small
focused prerequisite, not the missing shared D05 trace fixture.

## Scope and limits

The new evidence uses a fresh inline test program. The old HOS09a D05 fixture
and its hash are unavailable in this published checkout; this work does not
reconstruct or claim byte identity with it. A later D05 trace fixture must be
captured anew and have its own content identity after this forwarding behavior
is reviewed.

Core stores are still applied in MEM before WB retirement. This work does not
add rollback, precise faults, MMIO transactions, or a board/OS handoff. The
tests use aligned in-range word loads and stores in flat RAM. It does not change
the independent HOS03d Go semantic simulator, which has no guest CPU.

## Implementation boundary

The hazard detector may report both source resolutions while preserving its
single action and action priority. `cpu-pipeline` transports optional typed
records without changing legacy callback semantics. `core` maps the detector's
source positions to those records, and `riscv-simulator` consumes them. No
other ISA decoder receives or acts on the typed records unless it explicitly
implements the same contract.

The affected package BUILD recipes remain the validation entry point for
`cpu-pipeline`, `hazard-detection`, `core`, and `riscv-simulator`; the repo pins
Go 1.26.1.
