# NN42: Four Native Dense Framework Conformance

Status: local four-runtime execution implemented and passed; selected required
exact-head repository CI and confirmed merge remain pending.

## Prioritized scope

NN41 merged in PR #18226 at exact head `020d21876294`, squash `74682a93cee`,
after the real Go package BUILD, metadata, required CI and Go CodeQL passed.
Reassess reliability, prerequisites, portability and smallest complete delivery:
first connect the delivered Go port to required executable conformance, before
another native port. Literal-null shared-corpus expansion remains a separate
reliability item: the existing Go loader already rejects those cells directly.
No production training algorithm, public library API or checkpoint field changes.

## Runtime evidence

Extend NN39's fixed shell-free runner with the independent Go XOR command.
Execute four fresh native models, eight native baseline resumes, and all twelve
directed source/destination pairs with a return trip (36 child executions).
Compare full receipts, every parameter, predictions, losses and exact steps at
3000/3001/3002 within 1e-9. Retain finite/nonboolean values, exact receipt fields,
XOR success, loss continuity, missing-tool/process failure/timeout rejection.
Go executes a fixed trusted module command with `--resume` only after its
program argument; Cargo retains its separate `-- --resume` convention. Checkpoint
data travels only through JSON stdin; never derive commands or paths from it.
Receipts identify four native lanes and zero binding lanes, not a static roster.

## Required gate and validation

Add Go setup, actual matrix/framework build/tests/vet, native validator and unit
tests to the existing reusable workflow; retain actual root Rust workspace CI,
Python/TypeScript builds/coverage and every existing selected-gate fail-closed
roll-up rule. Register Go framework/matrix, native validator and regression-test
inputs alongside all prior source/workspace/fixture inputs. Selected conformance
must succeed, never skip. Do not add a duplicate standalone automatic workflow.

Pin exact Go argv/cwd/timeouts and all twelve directed pairs in regressions,
including drift introduced specifically by the new lane and both return steps.
Run actual four-runtime execution, native numerical/boundary/corpus suites,
production builds, coverage, formatting/lint, links/lessons and diff checks.
Local Rust validation may use the documented exact-copy minimal workspace with
an explicit trusted manifest override; real repository workspace CI is mandatory.
Update lesson, curriculum, gate contract, package changelogs and roadmap; obtain
independent security review before every push. One ready PR, exact-head CI and
confirmed merge are required before recording delivery or choosing another item.

Local evidence: all twelve actual directed exchanges passed with XOR loss
0.0002330712337828204 and return step 3002. Python 117 tests and TypeScript 114
tests pass at 100% library coverage; Go passes numerical/race suites (96.9%
library, 84% command coverage), vet/build; Rust passes 10 integration tests,
README doctest, fmt/Clippy and release example in the exact-copy minimal workspace.
TypeScript production build and coverage commands used the explicit installed
Node executable after npm's child-shell PATH lookup failed on this Windows host.
This is not full local repository-workspace validation; CI must supply that proof.
