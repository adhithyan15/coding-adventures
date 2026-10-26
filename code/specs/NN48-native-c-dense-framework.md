# NN48: Native C Dense Framework

Status: specified before implementation; native implementation and CI pending.

## Reassessment after the C JSON prerequisite

NN47 PR #18261 merged exact head `26bacfea3fd2` as `65aaf912bc7` after all
five selected workflows passed. Actual C JSON package BUILDs were Linux 0.7s,
macOS 0.8s and Windows 1.2s. Windows cl and clang-cl each passed 1605 checks,
positive C/C++ fixtures and negative-extension rejection. The final required CI
roll-up recorded contracts/build success and required:true/result:success for
actual five-native checkpoint exchange. This prerequisite is parsing, not C
neural training. CodeQL had no configured C/C++ analyzer; neutral analysis
selection and no open alerts do not establish a C code-scanning result.

Reprioritize reliability, prerequisites, portability and smallest complete
delivery. C now has native matrix, float-math, bounded JSON and a strict ISO
harness. Implement one independent native C dense library above those packages,
then a separate six-native conformance tranche. Do not substitute the existing
topology graph, Rust bindings, external JSON/ML, or teaching-only training code.

## Public C17 API, ownership and arithmetic

Create `code/packages/c/neural-framework`, a pure ISO C17 header/source library
with namespaced functions, explicit status/out-parameter errors, an opaque owned
network, and resetting idempotent release functions. Use native `Mat` operations
for dense arithmetic and native float-math for nonlinear functions; no libm,
foreign runtime or external ML supplies training. Never change dependency
numerical or ownership semantics as part of this port.

Expose configured and uint32-seeded construction, read-only safe step count,
prediction, full loss/prediction/weight/bias gradient traces, transactional
candidate SGD, exact-count fit/history including zero epochs, and owned V1
checkpoint save/load/JSON entry points. Constructors copy all caller arrays;
traces, predictions and checkpoints must not alias network parameters. State
can only change through successful optimizer updates. Document caller-readable
buffer lengths and view lifetimes; do not promise safety for forged C pointers.

Validate nonempty connected shapes, finite numeric cells, activation/rate/count
ranges, and element/byte arithmetic before constructing Mat or copying data.
Use explicit documented closed model/batch/workspace/output capacity limits,
checked size arithmetic and bounded cleanup. Report allocation failures without
leaks or partially published handles; exercise partial-allocation cleanup with
deterministic fault injection where allocations are controlled by this package.
Step is 0..2^53-1; an exhausted step or nonfinite arithmetic rejects updates.
Candidate parameters and candidate batch loss must all be finite before one
atomic commit and one counter increment. Fit returns initial plus post-update
losses after exactly the requested successful steps; zero epochs is read-only.

Preserve NN37 input-by-output weights, four activations, stable sigmoid, ReLU's
zero derivative, MSE over every sample/output coordinate, full analytic backward
pass, uint32 wrapping LCG and layer/input/output draw order. Biases are zero and
consume no draws. Seeded initialization is reproducibility, not cryptography.

## Strict typed checkpoints

Use NN47 public JSON values for schema loading, not a new parser. Closed V1 root
and layer fields must match NN37 exactly, with owned copied parameters. Metadata
requires lexical BJ_INTEGER values in range: floats such as 1.0/1e0, booleans,
nulls and strings cannot become version/step counts. Numeric parameter cells
accept finite JSON integers or floats, never booleans/nulls. Reject unknown,
missing, duplicate or embedded-NUL field names and disconnected/ragged arrays.
Do not claim duplicate-key text semantics identical to parsers that discard
duplicates before schema loading.

Text input and resume stdin are bounded to 4 MiB/128 JSON nesting levels via
NN47; honor its node/allocation ceilings. Serialization must produce ordinary
finite locale-independent JSON without changing the process locale, use enough
digits for double round trips, and check the output envelope before publication.
If the host locale is consulted for decimal conversion, document the normal C
requirement to serialize concurrent process-global locale changes. No arbitrary
paths, executable strings, datasets, RNG/backend/optimizer state enter V1.

## Direct numerical, rejection and runtime evidence

Consume shared dense.json through the public API for paper predictions, loss,
every gradient and update at 1e-12, and seeded/XOR data. Consume all 55 NN40/NN43
checkpoint cases through the real public schema loader (3 accept/52 reject,
including all six null-cell IDs), not merely through JSON syntax parsing.
Central finite differences cover every parameter for all four activations in
deep unequal-width multiple-output networks. Require active nonzero ReLU paths
away from its kink and separately test its zero derivative.

Test shape/type/finite/capacity/count/rate boundaries, ownership and view
isolation, safe-step exhaustion, exact fit including zero, overflow and failed
candidate atomicity. Target >=95% actual library coverage with meaningful branch
evidence; runnable commands/validators target >=80%. Run ASan/UBSan and available
native static analysis. Do not derive coverage from a failed fixture run.

Supply a native XOR executable with a fixed `--resume` mode and bounded stdin,
emitting a native-only JSON receipt/checkpoint/predictions/initial/final loss.
Actually train 3000 steps below 0.01, then compare full uninterrupted native
baselines and save/load/resumes through 3001 and 3002 at 1e-9. A shell-free Python
validator must execute actual fixed native argv/cwd with bounded timeouts and
test failures, malformed receipts and full numerical/shape drift. No checkpoint
content controls commands or filenames. State explicitly that six-native
exchange remains pending in a separate following tranche.

## Package, build and publication

Use the existing ISO harness, explicit deps=c/iso-harness,c/matrix,c/float-math,
c/bounded-json metadata in the repository's accepted syntax, both BUILD fronts,
strict GCC+Clang on Linux, Apple Clang on macOS and cl/clang-cl on Windows.
Deliver appropriate static-library install/export metadata, explicit payload,
an actual separate installed-consumer prediction/training/checkpoint smoke,
license/capability manifest, README/API example/changelog, plain-language lesson
and trace, curriculum and roadmap. Do not create a competing compiler scheduler.

The actual Windows native training package must be selected and executed in
repository CI; NN47 proved generic C-only toolchain detection alone did not
retain that lane. Prefer the smallest existing matrix/gate integration, test its
actual planner and required-rollup ownership, and preserve independent OCaml
selection. Any edit to the shared matrix planner must update its exact reviewed
OCAML03 digest after semantic review and all mutation-rejection tests, never
weaken its identity guard. Selected conformance jobs must succeed, not skip.

Before every push, run affected native/build/coverage/fixture/command/resume,
lint/links/lessons/diff checks and mandatory independent read-only security
review at the exact final head. Publish one ready PR and attach it, inspect real
package logs/required and expected checks/annotations/applicable scanning, and
verify exact-head merge before another item. A neutral unconfigured C analyzer
is not scanning evidence. Do not cancel runs or invent changes for queued
runners. Preserve dirty primary/unrelated work and old residual directories.

Disk is about 270 MiB free after sparse initialization (external usage reduced
it despite successful NN47 cleanup). Recheck before builds; keep C outputs
bounded, no full checkout, new large Rust builds, or unrelated cleanup. All new
package paths were sparse-added before creation. Native C conformance remains
distinct from parsing, bindings, demos and later architectures.
