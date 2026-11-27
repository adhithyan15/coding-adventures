# HOS-D06-GO1 — Opt-in deterministic D-MEM ready/wait for the Go precise core

## Purpose and boundary

Define a bounded timing extension to the opt-in precise flat-RAM memory path
from HOS-D05-GO7. This is a pipeline backpressure contract for one in-order
classic five-stage RV32I core. It covers only aligned, in-range word `LW` and
`SW` requests to flat RAM. It does not alter default or legacy execution and
does not make cache latency, `MemoryLatency`, or the existing asynchronous
`MemoryController` API implicitly active in the core.

The extension is selected explicitly and has a positive configured data-memory
service latency in pipeline cycles. The proposed setting is named
`DataMemoryLatencyCycles` so it cannot be mistaken for the existing
`MemoryLatency` field. Its omitted/default value is one cycle. Existing
constructors, default configuration, legacy callbacks, and precise-mode
configuration retain their current behavior unless the new option is enabled.
Latency one is the compatibility case: a valid memory operation completes in
its first MEM cycle, introduces no wait, and preserves the current precise
nonfault baseline of 16 cycles and 9 retired instructions, including the
published register/RAM state and trace projection.

This is not a general memory-system timing model. It has one outstanding
request, in-order completion, one fixed latency per core, and no arbitration.
It does not cover byte/halfword operations, instruction fetch, invalid
accesses, devices, MMIO, caches, write buffers, multiple cores, interrupts,
CSR/privilege behavior, or OS boot. Existing HOS-D05 terminal memory faults
remain selected by the existing EX preflight before a request can issue.

## Cycle convention

`Core.Step()` advances exactly one numbered pipeline cycle. A request's first
MEM cycle is its issue/service cycle and counts as service cycle one. At the
beginning of each cycle, before hazard arbitration, the memory timing unit
advances the outstanding request and makes any completed response visible to
the MEM token. For latency `L`, a request issued in cycle `M` is ready for
normal pipeline advancement in cycle `M + L - 1`. Therefore it incurs exactly
`L - 1` held cycles:

| Configured latency | Issue cycle | Held cycles | Ready/release cycle |
| ---: | --- | ---: | --- |
| 1 | `M` | 0 | `M` |
| 2 | `M` | 1 (`M`) | `M + 1` |
| 3 | `M` | 2 (`M`, `M + 1`) | `M + 2` |

The ready response is available before that cycle's hazard/forwarding decision,
not at the end of the cycle. This lets a dependent instruction use the normal
typed MEM forwarding path on the release cycle. The cycle counter, retired
count, and stage trace count each held cycle exactly once. A hold is not a
second memory access, retirement, or callback invocation.

## Request lifecycle and backpressure

Each dynamic pipeline instruction has a stable token identity distinct from
its PC; loops may execute the same PC more than once. When an eligible `LW` or
`SW` first enters MEM, it issues one request containing that token identity,
instruction PC, checked effective word address, operation kind, and the
configured latency. No request issues for a faulting or squashed instruction.
At most one request may be outstanding. The request cannot be replaced,
reissued, canceled, or completed out of order while pending. A request-complete
event and a request-issued count are recorded once per dynamic token; held-cycle
evidence records the token and cycle for each actual wait cycle.

If the request is not ready at the beginning of a cycle:

1. Hold IF, ID, EX, and MEM tokens in place, preserving their identities and
   values; keep the fetch PC unchanged and do not fetch or decode younger work.
2. Keep the pending request and MEM token unchanged except for its service
   progress. Do not invoke MEM read/store effects again.
3. Allow the older WB token present on the first wait cycle to run its normal
   WB callback and retire/commit once. The WB slot becomes a bubble for later
   wait cycles; it is never held or replayed. WB processing of a token must
   not repeat merely because a younger MEM request remains pending.
4. On the release cycle, expose the response before hazard selection, then
   release the held stages through the ordinary pipeline transition. Do not
   insert an additional bubble beyond the `L - 1` wait cycles defined above.

A dedicated additive wait/backpressure action is required if the existing
pipeline stall action cannot hold the MEM token and freeze every younger
stage while draining WB once. Do not redefine the behavior of existing
`HazardStall` callers.

### Load response and typed forwarding

An `LW` request samples its architecturally visible word at issue after
applying the existing newest-older-pending-store forwarding rule. The sampled
response remains attached to the request/token until ready. On the release
cycle, make the completed value available as a typed MEM result before normal
hazard and forwarding selection. A dependent consumer may forward from MEM on
that cycle; it must not see an uninitialized generic ALU result or the load's
effective address as the operand. The existing EX load-use rule remains in
force before the value reaches MEM.

### Store response and architectural commit

An `SW` has the same request issue, service, hold, and completion accounting,
but request completion is only timing readiness. It must not write RAM or emit
an architectural store-commit receipt. Preserve the D05 pending store intent
and store-to-load forwarding behavior. The store changes flat RAM and emits
its one commit receipt only when that same token reaches WB. If the store is
held in MEM, it cannot reach WB or commit early; after release it advances
normally and commits once. A following load must observe the older store,
including when the load issues while the store is in WB, using D05 forwarding
or the committed RAM value as appropriate.

## Fault ordering with a pending older request

An EX token may already carry a D05 precise fault while an older valid memory
request waits in MEM. The older MEM wait has priority over selection of that
younger EX fault:

- Keep the fault token in EX and all younger tokens frozen/suppressed while
  the older request is not ready. Do not select the fault, advance the fault
  token, issue younger memory work, or fetch a replacement.
- On the older request's ready cycle, expose its response and let the older
  MEM token advance toward WB. The existing precise-fault action may then
  select/squash the younger fault according to D05 ordering while allowing the
  older token to drain.
- The older instruction's GPR/store effect must commit once before the
  terminal fault outcome. The younger fault remains absent from successful
  retirement and has no memory request or architectural effect.

This ordering applies to an already detected D05 EX preflight fault; D-MEM
readiness does not create new fault classes or defer address validation.

## Independent validation oracles

Validation must use two references with separate responsibilities:

1. **Independent cycle/hold oracle.** A small test-only state model derives
   issue, held, completion, and release cycles from `(latency, first MEM
   cycle)`, without calling the production request/tick or pipeline-wait
   helpers. For the table above it expects respectively `(M, no holds, M)`,
   `(M, [M], M+1)`, and `(M, [M,M+1], M+2)`. The test compares token identities
   and per-cycle IF/ID/EX/MEM/WB observations, fetch-PC stability, issue/complete
   counts, wait-cycle evidence, and exactly-once WB retirement/commit. It also
   exercises at least one latency greater than three and a second memory
   instruction to prove the single-outstanding in-order rule.
2. **Independent sequential architectural oracle.** Run each guest program
   through a fresh sequential `RiscVSimulator`, then compare the precise
   pipeline's successfully retired PCs, all architectural registers, and all
   flat-RAM words. For faulting programs compare the selected fault and the
   state after the successful prefix, preserving D05's rule that the faulting
   PC is not a successful retirement. This oracle does not predict pipeline
   wait cycles.

The pipeline oracle must not use a captured golden fixture as its execution
input, nor derive expected holds from the implementation's request counter.
Use direct small programs and explicit cycle expectations; retain the
published 16-cycle/9-retirement fixture unchanged as a compatibility check.

## Required evidence

1. **Default/legacy isolation.** With the opt-in disabled, current constructors
   and traces remain byte-for-byte behaviorally unchanged. Existing precise
   D05 tests pass. With opt-in latency one, the 16-cycle/9-retirement baseline
   remains unchanged and reports zero wait cycles.
2. **Exact latency accounting.** For latencies 1, 2, and 3, assert the issue,
   held-cycle list, completion, and release cycles exactly as in the table.
   Add a larger deterministic latency and show total cycles increase by
   exactly `L - 1` for one isolated memory operation, with all other expected
   pipeline events unchanged.
3. **Stage hold and WB drain.** While pending, assert stable IF/ID/EX/MEM token
   identities, stable fetch PC, no fetch/decode/execute/MEM callback replay,
   and no duplicate retirement or store receipt. A valid older WB operation
   retires/commits once on the first wait cycle; later wait cycles see an empty
   WB slot. Deliberately repeat a callback or WB completion in a negative
   test seam and demonstrate the evidence detects it.
4. **Delayed typed load forwarding.** Use an `LW` followed by a dependent ALU
   instruction. Assert the data hazard behavior before the load result is
   ready, the load value appears as a typed MEM result before release-cycle
   forwarding, and the consumer's register result matches the sequential
   oracle. Ensure the effective address and any prior generic ALU value cannot
   be mistaken for the loaded operand.
5. **Delayed store/load ordering.** Use adjacent same-address `SW; LW`, and a
   delayed store followed by a dependent load. Assert the store does not
   change RAM at request completion, the load sees the newest older pending
   store, and the store has exactly one WB commit/receipt. Compare complete
   architectural state to the sequential oracle.
6. **Older wait before younger fault.** Hold a valid older load or store in
   MEM while a younger EX token carries a D05 fault. Assert no fault selection
   before the older response is ready, no younger fetch/effect, older effect
   exactly once, then the selected younger fault and exact successful prefix.
7. **Wrong-path suppression.** A redirected/squashed memory token never issues
   a request or contributes latency/wait events. Existing D05 wrong-path fault
   behavior remains unchanged.
8. **Bounded differential.** Every new guest program is at most 32
   instructions and every run at most 400 cycles. Compare all registers and
   RAM to the independent sequential oracle; fault tests compare the successful
   prefix and selected fault. The cycle oracle separately verifies timing and
   stage control.

## Implementation constraints and deferred work

- Keep the feature opt-in and additive. Do not overload existing
  `MemoryLatency`, claim cache timing, or route the precise path through
  `MemoryController.RequestWrite` unchanged; that API commits RAM at request
  completion rather than at precise WB.
- Preserve the existing D05 synchronous path and constructors. A latency-one
  request must not add a pipeline cycle.
- Keep the first implementation to flat-RAM word `LW`/`SW`, one core, one
  outstanding in-order request, and deterministic service latency.
- Do not add a device/MMIO API, cache miss model, write buffer, multiple
  outstanding transactions, reordered completion, bus arbitration, multi-core
  ordering, new address fault classes, instruction-fetch waits, CSR/trap
  semantics, or OS lifecycle behavior. Each needs a separate contract and
  independent evidence.
