# BEAM13 — root intermediate imported-call values

## Problem and boundary

An imported call may collect and relocate heap terms. Scratch X registers
above the call's argument prefix are not roots. An IIR value that survives the
whole instruction already uses the normal `live_across` Y slots, but a
temporary needed only between two calls in one lowering arm does not.

Current ETS-backed `array_set` carries its table identifier in a scratch X
register across `erlang:list_to_tuple/1` before `ets:insert/2`. `call_closure`
similarly carries its function atom across `erlang:'++'/2` before
`erlang:apply/3`. The atom is normally immediate, but the call contract still
does not promise to preserve its X register. ETS `alloc_array` already saves
its declared length in an initialized Y slot across `ets:new/2`.

## Required lowering

Reserve one initialized transient Y slot per function that contains any of
these three paths. It is separate from Y slots assigned to IIR variables live
across calls. The three paths execute sequentially, so they may share that
slot. Count it in the 255-slot frame limit.

For ETS `array_set`, retain the existing integer and bounds guards and the
`test_heap` root prefix. Once the table is in the post-reservation scratch X
register, save it in the transient Y slot before `list_to_tuple/1`. Reload it
from Y into `x0` before `ets:insert/2`; never read the pre-call scratch copy.

For `call_closure`, save the extracted function atom in the transient Y slot
before `erlang:'++'/2`, then reload it from Y into `x1` before
`erlang:apply/3`. Keep the existing IIR live-variable save/restore around the
whole instruction. The append result reaches apply without another allocating
operation between those calls.

## Evidence

Instruction-shape tests must locate the relevant imported calls and assert
the save/call/reload ordering and the shared slot reservation. Real-Erlang
tests must execute ETS stores and closure calls under repeated allocations,
checking returned values and that the emulator does not crash. The existing
array bounds, string-array, closure liveness, and BEAM allocation tests must
continue to pass. Audit remaining scratch/call sites under issue #15882
before treating that backend-wide issue as closed.
