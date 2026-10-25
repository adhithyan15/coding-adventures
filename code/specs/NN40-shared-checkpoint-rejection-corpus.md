# NN40: Shared Native Checkpoint Rejection Corpus

Status: specified before implementation.

## Prioritized deliverable

NN39 merged in PR #18218 at head `f8e5ce59571`, with all six actual native
checkpoint exchanges and applicable CI/CodeQL successful. Reprioritization by
reliability, prerequisites, portability, and smallest complete deliverable
selects a shared checkpoint rejection corpus before Go/Ruby ports or bindings.

Add one language-neutral JSON fixture under `fixtures/neural-framework-v1/`.
Its closed root contains `contract` and `cases`; each case has a unique `id`,
an `accept` boolean, and a parsed JSON `checkpoint`. Use small explicit values,
not language-specific mutation functions. Include accepted controls so a loader
which rejects everything cannot pass. Cover wrong root/metadata types, booleans
as numbers, unsafe/fractional/negative steps, unknown and missing fields,
invalid activations, empty/ragged/non-numeric weights and biases, and disconnected
layers. Valid JSON overflow numbers may test finite-value rejection; do not
put nonstandard NaN or Infinity tokens in the fixture.

## Native execution and boundaries

Python, TypeScript, and Rust package tests independently read that exact fixture
and call their public native checkpoint loaders. Each case must agree with the
shared accept/reject expectation. Accepted controls must serialize and reload
without changing the model. Assert the corpus contract, unique IDs, closed
case fields, and presence of both accepted and rejected cases. Native error
messages need not agree; the portable contract is acceptance, not spelling.

This is parsed-document conformance, not a promise that each runtime's JSON
text parser has identical duplicate-key or nesting semantics. Rust exercises
its JSON reader before its owned model loader; its existing text bounds remain
unchanged. No checkpoint format fields, native training algorithms, bindings,
architectures, or new CLI input surface are added.

## Delivery evidence

Retain numerical, finite-difference, ownership, atomic-update and six-direction
NN39 tests. Run all three native suites against the shared corpus, production
builds and coverage, required-gate selection tests, link validation, lesson
validation, Ruff/format/Clippy, and diff checks before independent security
review and publication. The existing reusable conformance workflow must execute
these package tests; shared fixture changes must select that gate. Actual CI
must validate the real Rust workspace; local sparse validation may use the
documented exact-copy manifest override without claiming full workspace proof.

Update lesson, changelogs and roadmap. Mark completion only with direct native
execution and exact-head required CI plus confirmed merge.
