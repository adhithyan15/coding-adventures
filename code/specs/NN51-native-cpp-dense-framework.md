# NN51: Independent Native C++ Dense Framework

Status: specified before implementation. Native C++ training and seven-native
checkpoint exchange are not delivered by the preceding parser prerequisite.

Implementation checkpoint: independent native library/command and shared ISO
fronts exist locally. GCC/LLVM Clang each passed 237 library checks and actual
fresh/resume/baseline command validation through step 3002. Both tested 93 real
allocation-failure positions; ASan's separate variant passed 236 checks without
the incompatible replacement hooks, plus actual command tests. Local coverage:
97.56% library lines, 90.91% command lines, 100% Python validator statements.
Installed CMake consumer compiled/linked/ran. These are Windows-local results,
not actual Linux/macOS/MSVC CI or seven-native exchange. CI selection audit,
teaching/curriculum, independent final review and publication remain pending.

## Selection after NN50

NN50 merged [PR #18333](https://github.com/adhithyan15/coding-adventures/pull/18333)
at exact reviewed head `099b92b2974425c8ae88d8c769c935303b81735e`, squash
`cbf9c2e8953487c51aaa121fb26b53a93483b079`, after all five workflows and both
required gates passed. Final CI log binds that head, required neural conformance,
all selected results and all platform builds to success. Actual Windows cl and
clang-cl each passed 498 parser checks; Linux/macOS package BUILDs each reported
2.4 seconds, Windows 4.3 seconds. Unix child stdout was suppressed, so no equal
per-compiler counts are inferred. No C++ CodeQL analyzer was configured.

Reassess reliability, prerequisites, portability and smallest complete delivery:
native C++ matrix, float-math, bounded-json and the shared ISO harness now exist.
Deliver an independent C++ dense learner above those owned foundations before
a separate seven-native exchange tranche. Do not wrap native C/Rust training,
add a binding, external ML/JSON dependency or new architecture in this delivery.

## Public ownership and mathematics

Create `code/packages/cpp/neural-framework` with an idiomatic C++17 public API in
`ca::neural_framework`. A network privately owns copied input-by-output matrices,
bias vectors, closed activations (`linear`, `sigmoid`, `tanh`, `relu`) and its
successful-update count. Public constructor inputs, layer inspection, predictions,
traces and checkpoints never provide writable aliases into model state.

Expose seeded construction, prediction, gradients, one train step, fit,
checkpoint load/save and copied layer inspection. Copying a network creates an
independent value; document move behavior. Wrong kinds/shapes/rates/counts and
nonfinite arithmetic fail explicitly, not by unchecked access. Errors identify
a closed category, not raw checkpoint fragments. Use unsigned 32-bit wrapping
LCG initialization in NN37's exact layer/input/output order; biases consume no
draws. Preserve NN37's full-batch MSE and every weight/bias derivative, including
unequal layer widths, multiple outputs and ReLU derivative zero at zero.

Use the existing native `ca::matrix::Matrix` for dense operations and the owned
float-math foundation for nonlinear operations. The matrix type exposes public
data and cached dimensions: validate both the nested row lengths and cached
rows/columns before invoking its operations. A corrupted caller-owned matrix
must fail, not cause an out-of-bounds access in an unchanged dependency.

Keep finite work and storage envelopes: at most 16 layers, widths 64, samples
256, total parameters 65536, fit epochs 100000 and step `9007199254740991`.
Validate capacities and products before allocations, traversal or conversion.
Zero epochs validates batch/rate and returns one initial loss without updating.
Fit returns `epochs + 1` losses; each successful epoch increments exactly once.

One train step is transactional: compute a pre-update trace and independent
candidate, reject any nonfinite parameter/intermediate/gradient or candidate
batch loss, then publish state and increment step together. Failed allocation,
overflow, shape/rate error or step exhaustion leaves parameters and step intact.
Fit is a series of such atomic steps, not an all-epochs transaction: completed
epochs may remain if a later epoch fails. Document that boundary and test it.

## Typed checkpoints and native command boundary

Consume NN50's public immutable JSON values; do not write another parser. Loading
copies all model data so the source document may immediately be destroyed.
Enforce NN37/NN40's exact closed V1 root/layer fields, format/version, lexical
integer version/step, safe nonnegative step, valid activations, nonempty finite
rectangular numeric matrices/vectors and consecutive shape compatibility.
Reject duplicate/unknown/NUL-altered names, booleans and null numeric cells.
Retain the shared 55-case corpus unchanged: three accepts, 52 rejects, six null
cases, exercised through the real public native loader rather than a mirror.

Serialize with locale-independent round-trippable finite double spelling and
integer metadata. Preserve negative floating zero explicitly (e.g. `-0.0`, not
an integer `-0` that loses its sign on reload), extremes and subnormals through
full save/load equality. Parsing/serialization never changes global locale.

Provide a native XOR command with fixed trusted argv: fresh training or bounded
JSON stdin under `--resume`. Check the loaded model's actual prediction/target
shape against XOR before using rows. Fail closed on oversized input, extra argv,
malformed/schema-invalid input, nonfinite computation and wrong dataset shape.
No checkpoint selects paths, commands, cwd or executable. Emit a closed native
NN51 receipt with full checkpoint, predictions, losses and exact step; mark
seven-native exchange pending. Use the existing shared ISO scheduler and package
cwd in both BUILD fronts; every produced binary must pass actual command tests.

## Validation, packaging and publication

Commit this specification before implementation. Then implement tests and native
code, followed by package changelog/README, teaching trace, curriculum and roadmap.
Consume the shared dense fixture directly for paper forward/loss/all gradients,
LCG initialization and updated parameters. Exercise finite differences for every
parameter in unequal-width/multiple-output networks; all four activations;
ownership isolation; corrupted matrices; resource/step/error atomicity; zero
epochs; strict checkpoint corpus; extreme/signed-zero round trips; actual XOR
3000 epochs and full fresh/resumed states through steps 3001 and 3002.

Measure library line coverage at least 95% and native command/validator at least
80%. Run ASan and supported UBSan with real exit status; do not turn unavailable
sanitizer linkage into a clean run. NN50's separate Windows UBSan evidence note
is reconciled into a repository lesson in this tranche. Header self-containment,
multi-translation-unit linkage and an installed standalone consumer must prove
native prediction, update and checkpoint reload, with all transitive public
headers/payload dependencies installed in leaf-to-root order.

Prove actual GCC/Clang, MSVC/clang-cl and Apple Clang execution using existing
ISO fronts. Inspect real dependency-aware CI selection: extend only a confirmed
native-training Windows ownership requirement using the existing runner/bootstrap
and required build roll-up; preserve NN50/C/OCaml predicates and toolchain pins.
If the owned planner changes, review its exact OCAML03 identity and rerun existing
mutation-rejection tests. Do not broaden unrelated C++ scheduling or weaken
inventory validation to accommodate a sparse local checkout.

Obtain mandatory independent read-only security review of the exact final head
before every push. Publish one ready attached PR, inspect all selected workflows,
actual native platform/package logs and final required roll-up, and verify exact
head plus merge commit before cleanup/next tranche. C++ native training alone
does not establish seven-native exchange; keep that runner expansion separate.
