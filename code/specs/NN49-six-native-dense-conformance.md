# NN49: Six Native Dense Framework Conformance

Status: specification committed before implementation; runtime evidence pending.

## Selection and prerequisites

NN48 merged in [PR #18285](https://github.com/adhithyan15/coding-adventures/pull/18285)
at exact reviewed head `437c4c2d01bb06dcb618ef57421882e70860537f`, squash
`c91b44a06e70dcd86fa43fbf1c66230597524de1`. All five selected workflows and
both required gates passed. Actual Linux/macOS package BUILD summaries and
Windows cl/clang-cl training, command-boundary and resume logs establish native
C training. CodeQL detection was green but no C analyzer was configured; this
is not C scanning evidence. Existing NN46 exchanges cover only five languages.

Reprioritize reliability, prerequisites, portability and smallest complete
delivery: connect the delivered C implementation to actual required checkpoint
exchange before starting another port, binding or architecture. Preserve all
native library APIs, math, V1 schema and historical native-only receipts.

## Runtime contract

Extend the existing shell-free interchange runner with a trusted, explicitly
selected native C executable compiled by the existing ISO harness. C checkpoints
enter only through JSON stdin with fixed `--resume`; no checkpoint-derived
arguments, executable paths or working directories. Retain fixed timeouts,
finite/nonboolean admission, closed receipt validation, exact integer steps,
Cargo argument separation and the other five lane commands and directories.
Missing or unusable C executables fail closed, never silently omit the lane.

Execute six fresh native models, twelve same-lane baseline resumes and all
thirty directed source/destination pairs with return trips: 78 actual child
executions. Compare complete receipts, every weight/bias, prediction, initial
and final loss and exact steps at 3000/3001/3002 using NN37's 1e-9 tolerance.
Require XOR loss below 0.01. The NN49 receipt names six actually executed native
lanes, zero binding lanes and all thirty distinct directed exchanges. A static
lane list or mocked regression alone does not establish runtime conformance.

## Build and required gate ownership

Use the existing reusable neural-framework workflow, not another automatic PR
workflow. Run the actual C package BUILD front with GCC and Clang required on
Linux, its command tests and native validator regressions, then pass a known
produced native executable to the shared exchange runner. Preserve actual root
Rust workspace validation and all prior Python/TypeScript/Rust/Go/Ruby checks.
Do not add a competing C compiler scheduler or weaken ISO flags.

Extend the existing conformance registry inputs with the C package, native
matrix/float-math/bounded-json/ISO-harness dependencies, native validator and
tests. Preserve all prior fixture/workspace inputs and fail-closed required
roll-up ownership. Selected conformance must succeed, never skip. Exercise the
actual gate shell and dependency selection in executable regressions. Review
any workflow-dependent pinned planner digest if its owned text changes; do not
refresh unrelated pins or hashes without a corresponding semantic change.

## Regression and delivery evidence

Pin C argv/cwd/stdin/timeout and all pair/process counts. Add C fresh-model,
same-lane baseline, destination and return-path numerical-drift regressions;
retain the intended drift boundaries in prior lane-sensitive tests. Cover
missing tools, nonzero child exits, timeouts, malformed/extra receipt fields,
nonfinite and boolean numeric leaves, incorrect shapes and noninteger steps.
Target at least 95% interchange-runner coverage and retain native loader corpus
and numerical/boundary tests; schema parsing alone is not training evidence.

At specification time the new sparse checkout has about 220 MiB free. Do not
perform a full checkout or large local Rust workspace build. Preserve NN38's
retained exact-copy sources and NN48's archived coverage/build evidence. Any
local minimal Rust workspace requires current source/test/example hash
verification and an explicit trusted manifest override; report it honestly as
minimal-workspace evidence. Full actual repository workspace execution remains
mandatory in CI. Measure disk before builds and preserve unrelated work.

Update the gate contract, teaching trace/curriculum, changelogs and roadmap;
retain historical five-native evidence as historical. Validate numerical runs,
coverage, lint, relative links, lesson shards and diff cleanliness. Require an
independent read-only security review on the exact final head before every code
push. Publish one READY PR, inspect exact-head selected conformance receipts,
actual package BUILDs, annotations and required roll-up, and verify the exact
head merged before marking NN49 complete. Queued runners require waiting, not
invented source changes or cancelled runs.
