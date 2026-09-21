---
category: Compiler / VM / language pipeline
---

# A hand-maintained list that mirrors what code does will drift, and tests written from the list cannot catch it

`closurec` reported which optimization passes a compile ran from two constants,
`SIMPLE_PASS_NAMES` and `ADVANCED_PASS_NAMES`, sitting a few hundred lines from
the `pipeline.add(...)` calls that actually register passes. They drifted:
`inline` is registered under `if advanced.is_some()`, but the SIMPLE constant
still listed it, so every SIMPLE run emitted provenance naming a pass that
never executed.

The test suite did not catch it, and could not have. The assertion had been
written by copying the constant:

```rust
body.contains("\"passes\":[\"constant-fold\",\"fold-control-flow\",\"dce\",\
               \"inline\",\"inline-variables\",\"rename\"]")
```

A test derived from the same list it is checking restates it. It passes exactly
when the code agrees with the list, which is what the list was already
asserting — so it proves the copy is faithful and says nothing about the
behaviour.

When the constants were removed and the value taken from the scheduler's own
`PipelineOutput::execution_order`, a **second** drift appeared that both the
constant and the test had hidden: the pipeline does not execute in registration
order at all. Its Kahn scheduler pops a FIFO ready queue, so every pass
declaring no `depends_on` is scheduled ahead of every dependent pass —
`rename`, registered eighth, runs second. One wrong list had been concealing
two different untruths, one about membership and one about order.

**What to do instead.** If a value describes what the code did, get it *from*
the code: ask the component that performed the work what it performed, rather
than maintaining a second copy and hoping review keeps them equal.
`PassPipeline` already returned `execution_order`; the parallel constants sat
beside an authoritative answer nobody read. Treat "these two must be kept in
sync" as a defect report rather than a maintenance note — a comment asking
future editors to update a second location is a standing invitation to the bug,
and discipline does not scale across contributors or across months.

**Never write a test's expected value by copying the constant the code under
test reads.** Derive it from observable behaviour: run the thing and assert a
property (`inline` must not appear when its registration was gated off), or the
test is a tautology in the shape of coverage.

Watch also for the mirror that duplicates a *decision* rather than a list. The
same change deleted `will_rename_properties`, a boolean that re-derived whether
a conditionally registered pass had been scheduled so a second site could
report on it. It happened to be correct, but it was the identical pattern one
refactor away from disagreeing.

Reporting, logging, and provenance code is where this hides, because wrong
output there is not a crash and no golden file changes. If a list of what ran
is a `const` rather than a return value, assume it is already wrong.
