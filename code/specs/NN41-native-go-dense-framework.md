# NN41: Native Go Dense Framework

Status: native implementation and local numerical/resume evidence complete;
exact-head repository CI and confirmed merge remain required for delivery.

## Smallest next native port

NN40 merged in PR #18222 at exact head `63e51b92796`, with required CI,
CodeQL, native rejection corpus and six-direction exchange all successful.
Reprioritize reliability, prerequisites, portability, then smallest complete
delivery: Go's existing matrix module is the next native arithmetic foundation.
Deliver one independent Go library and example before expanding the required
interchange runner to a fourth native language. Four-language conformance is a
separate following tranche, not implied by native training or static registration.

## Native public API and invariants

Create `code/packages/go/neural-framework`, module path matching the repository,
with a local `replace` for its Go matrix dependency. Do not wrap another runtime
or import an external ML framework. Use validated, copied `[][]float64` batches,
owned dense-layer records, private network state, four NN37 activations and
idiomatic error returns. Expose constructor, seeded constructor, prediction,
full gradient trace, single SGD step, exact-count fit, step inspection, and
owned checkpoint save/load plus JSON entry points. Public snapshots must never
alias network state. Validate shapes and finite values before constructing the
matrix foundation, whose public shape/data fields are not a trust boundary.

Match NN37's input-by-output weights, stable sigmoid, ReLU derivative at zero,
MSE reduction, wrapping uint32 LCG and draw order. Reject empty/disconnected
models, ragged/nonfinite batches, incompatible targets, negative counts,
nonpositive/nonfinite rates, invalid activations and unrepresentable capacities.
Step is a nonnegative integer at most `2^53-1`, checked before an update.
Compute candidate parameters and candidate batch loss first; errors must leave
every parameter and the step unchanged. Zero-epoch fit returns one initial loss.

Checkpoint JSON remains the closed NN37 V1 schema. Reject unknown/missing fields,
wrong roots/types, numeric booleans, unsafe/fractional/negative metadata and all
NN40 rejected cases; preserve accepted round-trip controls. Go's zero values must
not silently turn missing fields into valid version/step/layer records. Bound
JSON-text loading to 4 MiB; reject trailing JSON documents. Keep text parser
semantics distinct from typed owned record validation. No paths, code, dataset,
optimizer state, backend or new fields are serialized.

## Numerical, native and portable evidence

Directly consume `fixtures/neural-framework-v1/dense.json` and `checkpoints.json`.
Assert paper predictions, loss, every gradient and updated parameter at `1e-12`;
seeded XOR must reach loss below 0.01 after exactly 3000 steps. Central finite
differences cover all parameters in unequal-width, multi-output deep networks
and every activation. Test boundaries, ownership isolation, zero epochs, safe
step exhaustion, overflow and atomic failed updates, malformed JSON and shared
49-case parsed-document corpus. Target at least 95% library statement coverage;
run Go formatting, vet, tests, race checks where supported, and production build.

Supply a deterministic native XOR command producing the existing receipt fields
`checkpoint`, `predictions`, `initial_loss`, `final_loss`; `--resume` accepts a
bounded checkpoint from stdin and advances exactly one step. Native save/resume
must match uninterrupted training at steps 3001/3002 within `1e-9`. Record actual
example output; do not claim four-way interchange until its following tranche.

Provide README, changelog, BUILD/build metadata, required-capability manifest
where repository conventions require it, a plain-language lesson with a hand
oracle and trace, and roadmap evidence. Keep automated CI selecting actual Go
package tests/builds. Test full dependency installation, fixture paths, Markdown
links, lesson shards, and diff checks; independent security review precedes
every code push. Publish one ready PR and verify actual current-head CI/CodeQL
and conflict-free merge before selecting four-native-language conformance.

Local evidence: Go library tests exceed 95% statement coverage; standalone
example tests exceed 80%. Direct native XOR reaches loss approximately
0.00023307123378282042 at step 3000 and reload/resume reaches step 3002.
Race checks, vet, production builds, the paper/finite-difference/NN40 fixtures
and atomicity tests pass. The shell-free native validator names Go only and
explicitly leaves four-language conformance pending.
