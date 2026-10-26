# NN39: Three Native Dense Framework Conformance

Status: local three-runtime execution implemented and passed; required
current-head CI and merge remain pending.

## Smallest complete deliverable

After NN38's Rust implementation merged in PR #18212, prioritize reliability,
prerequisites, portability, then smallest complete delivery: prove the existing
three native dense frameworks interoperate before adding another port or ABI.
Do not add architectures, optimizers, bindings, or new checkpoint fields here.

Extend the shell-free interchange validator to execute Python, TypeScript, and
Rust independently. Each must train seeded XOR for 3000 updates, match the NN37
receipt within 1e-9, and load every other lane's actual checkpoint. Exercise all
six directed pairs, resume one update in the destination, then return to the
source for a second update. Compare every checkpoint parameter, prediction,
loss, and exact step count with uninterrupted native baselines at 3001/3002.
Receipts name all three lanes as native, with zero binding lanes. Never infer
execution from registry membership or a mock subprocess.

## Execution boundary

Use fixed argument vectors, trusted source paths, finite JSON receipts, bounded
process timeouts, and stdin checkpoint transport. Cargo's own argument separator
precedes `--resume`. Unknown lanes, missing tools, child failures/timeouts,
malformed receipts, non-finite numbers, numeric booleans, wrong steps, or numeric
drift must fail. Unit tests may substitute subprocess results, but publication
requires the actual three-runtime command as well.

The reusable CI job must build/test Rust and run the three-language validator,
using the real repository Rust workspace. Because that workspace references
many crates, the CI checkout is complete; local disk-constrained validation may
use exact source copies with an explicit trusted manifest override. Report that
local boundary honestly. Keep only the existing reusable required gate, not a
duplicate automatic workflow.

## Gate ownership

Register Rust framework, matrix, bounded-JSON, workspace manifest, native Rust
validator and tests, and interchange regression tests as selecting inputs.
Retain Python/TypeScript and all existing fixture/workflow inputs. Pin these
paths in tests. Selected conformance must succeed in the existing CI roll-up;
preserve the roll-up's failure/skip/cancellation semantics and dependency order.

Deliver updated lesson, roadmap, gate contract, changelogs and workflow along
with numerical execution evidence. Independent security review precedes every
push. Actual required conformance and CI/CodeQL at the exact PR head gate merge.

## Discovered follow-up

Cross-language invalid-checkpoint fixtures remain a separate reliability item:
each native library already owns negative tests, but this tranche proves valid
checkpoint exchange rather than a shared rejection corpus.
