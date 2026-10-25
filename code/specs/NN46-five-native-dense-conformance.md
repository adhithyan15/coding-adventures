# NN46: Five Native Dense Framework Conformance

Status: specified before implementation; runtime and exact-head CI pending.

## Prioritized scope

NN45 merged in PR #18249 at exact head `539db944794`, squash `9f4c55a0e7e`,
after Linux actually built `ruby/neural-framework` (9.9s) and `ruby/matrix`
(15.0s), repo-wide contracts, required CI gate and Ruby CodeQL succeeded.
There were no open code-scanning alerts; the sole Ruby annotation was an
informational runner-image migration notice. Four-native conformance correctly
remained unselected for that Ruby-only native implementation.

Reassess reliability, prerequisites, portability and smallest complete delivery:
connect Ruby to required actual interchange before extending another native
port. Preserve every public training API, algorithm and V1 checkpoint field.
Native framework training, bindings, demonstrations and architectures remain
distinct. This is conformance integration, not another library implementation.

## Runtime evidence

Extend the existing fixed shell-free Python/TypeScript/Rust/Go runner with the
independent Ruby command. Execute five fresh models, ten native baseline
resumes, then all twenty directed source/destination pairs with return trips:
55 actual child processes in total. Compare every receipt field, parameter,
prediction, loss and exact step at 3000/3001/3002 within NN37's 1e-9 tolerance.
Receipts must name five actually executed native lanes and zero binding lanes.

Ruby uses a trusted discovered executable and fixed source/load paths/cwd;
checkpoint data enters only through JSON stdin with `--resume` after the
program argument. Do not derive executable arguments or paths from checkpoint
content. Retain fixed timeouts, Cargo's `-- --resume` separator, Go module cwd,
finite/nonboolean admission, closed receipts, XOR success and loss continuity.
No static lane list or mocked test is actual five-runtime evidence.

## Required gate ownership

Add Ruby 3.4 setup using the repository's pinned ruby/setup-ruby action to the
existing reusable workflow. Run the existing matrix-first Ruby build helper,
library/corpus/coverage/Standard tests, isolated installed-gem training smoke,
command coverage, validator tests and actual native validator. Preserve actual
root Rust workspace validation and every prior Go/Python/TypeScript check.
Keep one bounded reusable workflow, not a duplicate automatic PR workflow.

In code/specs/data/ci-gates.json register ruby/neural-framework, ruby/matrix,
both source paths, the native Ruby validator and its tests. Preserve all prior
fixture/workspace/dependency inputs and fail-closed roll-up dependency ordering.
Selected conformance must succeed, never skip. Regression tests must exercise
the actual roll-up shell and prove Ruby input selection.

Pin exact Ruby argv/cwd/stdin/timeout and every pair/process count. Add dedicated
Ruby fresh, native baseline, destination and return-path numerical drift
regressions; include malformed JSON, nonfinite/bool leaves, exact-step/shape
errors, absent tools, nonzero children and timeouts. Update old lane-sensitive
drift tests so adding a lane cannot silently move their intended boundary.

## Validation and delivery

Run actual all-twenty-direction exchange, native numerical/boundary/55-case
corpus suites, production builds, coverage and lint. Local Rust may use retained
exact-copy minimal workspace sources with explicit trusted manifest override;
verify source/test/example hashes. Never claim full repository workspace locally:
actual root-workspace CI remains mandatory. Use explicit installed Node entry
points if the documented Windows npm child PATH issue recurs; CI must prove
normal npm scripts. Recheck disk before installs/builds (about 900 MiB free at
specification), avoid full checkout/large builds and preserve unrelated work.

Update current docs, gate contract, lesson/curriculum, package changelogs and
roadmap while preserving historical four-native evidence. Validate links,
lessons and branch diff. Independent read-only security review is mandatory
before every push. Publish one ready PR, inspect actual exact-head checks,
selected nested conformance logs and CodeQL findings, and confirm exact-head
merge before another item. Queued runners require waiting, not source changes.
