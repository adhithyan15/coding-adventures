# NN45: Native Ruby Dense Framework

Status: specified before implementation; implemented and locally validated;
publication and exact-head repository CI pending.

## Smallest complete native port

NN44 merged in PR #18243 at exact head `bf21d308e98`, squash `58756842612`,
after actual `ruby/matrix BUILT`, repository metadata and Ruby CodeQL passed.
The prerequisite is a buildable pure-Ruby matrix gem, not neural training.
Reprioritize reliability, prerequisites, portability and smallest complete
delivery: implement one independent Ruby dense library/example now; require
actual five-native checkpoint interchange in a separate following tranche.
Do not substitute the existing Rust binding, teaching demos, or external ML.

## Public native API and boundaries

Create `code/packages/ruby/neural-framework` with gem metadata and an explicit
local dependency on `coding_adventures_matrix` 0.2.x. Use that native matrix
foundation for dense arithmetic, never standard-library Matrix or another
runtime. Validate and copy arrays before Matrix construction: its public data
and constructor inputs are mutable aliases, not a model validation boundary.

Expose an idiomatic namespaced DenseNetwork with constructor and seeded
constructor, prediction, full gradient trace, single SGD step, exact-count fit,
read-only step inspection, owned checkpoint save/load and JSON entry points.
Public layer/trace/checkpoint arrays must never alias network state. Keep NN37
input-by-output weights, all four activations, stable sigmoid, ReLU derivative
at zero, MSE reduction and unsigned-32-bit wrapping LCG/draw order identical.

Accept only finite integer/float numeric cells, not strings, nil, booleans or
complex numbers; metadata/counts/seeds require actual integers in range. Reject
empty/ragged/nonfinite batches, invalid/disconnected layers, invalid activations,
targets/rates/counts and unrepresentable element/byte capacities before allocation.
Step stays within 0..2^53-1, checked before updates. Candidate SGD commits all
parameters and increments the step only after candidate batch loss is finite;
failure leaves the full model unchanged. Fit performs exactly the requested
successful steps and returns initial plus post-step losses, including zero epochs.

The owned checkpoint uses exactly NN37 V1 string-keyed JSON fields. Missing or
unknown fields, wrong roots/types, unsafe/fractional/negative step metadata and
all NN40/NN43 rejected documents fail. Save/load preserve valid controls without
executing content. JSON-text input and resume stdin are bounded to 4 MiB; reject
malformed/nonfinite JSON with an explicit nesting envelope. Preserve NN40's
parsed-document contract rather than claiming identical duplicate-key semantics.
No paths, code, datasets, RNG/optimizer state or backend flags enter the format.

## Native numerical and portable evidence

Consume shared dense.json and all 55 checkpoints.json documents directly through
the public native API. Pin paper predictions/loss/every gradient/update at1e-12,
seed draw order and exact counts. Central finite differences cover every
parameter in unequal-width deep multiple-output networks and every activation.
Test ownership, malformed types/shapes, nil/boolean cells, stable extremes,
zero epochs, safe-step exhaustion, capacity/overflow and failed-update atomicity.
Library coverage target is at least95%; runnable-command tests target at least80%.

Provide a deterministic native XOR example emitting checkpoint/predictions/
initial_loss/final_loss. Train3000 steps below0.01, accept only --resume with
bounded checkpoint stdin, advance one update, and compare full native models
against uninterrupted baselines at3001/3002 within1e-9. A fixed shell-free
validator and receipt must name Ruby native training only, explicitly leaving
five-language interchange pending. No static roster or mock substitutes for
actual native example/save/reload/resume evidence.

## Build and delivery

Deliver publishable gem metadata, dependency ordering, both BUILD fronts, public
README/examples, changelog, capability metadata, lesson/trace and roadmap.
Install/test the pure-Ruby matrix dependency before the framework and reset
working directories explicitly. Run Standard, numerical/boundary/corpus suites,
coverage, production gem build/installed smoke, native example/resume, links,
lessons and diff checks before independent security review and every code push.
Keep the matrix dependency's retired orphan exemption absent; inspect applicable
repository build metadata rather than inventing exemptions for new code.

Publish one ready PR, then verify actual package BUILD, expected CI/Ruby CodeQL
and applicable selected gates at the exact head. Four-native conformance may be
unselected until the following Ruby integration tranche; do not claim five-native
portability here. Confirm merge before another item. Use sparse isolation and
recheck disk (about1.17GB free at specification); avoid full checkout/Rust builds.

## Local implementation evidence

The native Ruby library and command both reach 100% measured line coverage.
Paper predictions/loss/every gradient/update match the shared oracle at 1e-12;
central finite differences cover every parameter of all four activations in
deep unequal-width multiple-output networks. All 55 checkpoint cases pass
directly through the public loader, with six null-case IDs pinned. Ownership,
type/shape/rate/count, JSON/nesting, safe-step, capacity and atomic overflow
tests pass. Actual XOR loss is 0.00023307123378282036 at 3000; complete native
models/predictions match through 3001/3002. Both gems build/install in a private
temporary home and a fresh Ruby child verifies installed source and training.
The shell-free native validator's command tests cover 98% of its lines.
No training or checkpoint contract deviation; five-native exchange remains
explicitly pending, and only actual repository CI can establish portable BUILD.
