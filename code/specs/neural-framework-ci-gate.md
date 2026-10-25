# Required Neural Framework Conformance

NN37's native training and portable checkpoint exchange must contribute to the
required repository `CI gate`, not merely an independent optional workflow.

## Scheduling contract

Register `neural-framework-conformance` as a job-scoped gate. Both affected
Python/TypeScript framework or matrix packages and direct changes to their
sources, the shared fixture, interchange validator, gate tests, registry, or
either workflow select it. Unrelated documentation does not select it. Forced
main builds select it through the existing registry evaluator.

Use one reusable workflow for native package validation and checkpoint exchange.
Keep manual dispatch, but let repository CI own automatic PR/main scheduling so
the same source change does not claim duplicate conformance runners.

## Fail-closed completion

The required roll-up depends on the conformance job and inspects its result.
When selected, only `success` is acceptable: failure, cancellation, skipped,
missing, and unknown results fail. An unselected job may be skipped or pass.
Missing/unknown selection flags fail. Preserve the existing exact-head handling
for superseded workflows and the roll-up's `always()` guards.

## Evidence

Tests pin registry inputs, detect output, reusable invocation, dependency/result
wiring, and execute the roll-up shell with selected/unselected success, failure,
cancellation, skipped, and malformed states. Run native numerical/coverage builds
and the real two-way checkpoint exchange; CI on the published head must show the
reusable conformance job plus the required roll-up. This changes validation
ownership, not the training API, checkpoint format, or native-language roster.
