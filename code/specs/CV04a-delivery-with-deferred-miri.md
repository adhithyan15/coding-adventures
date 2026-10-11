# CV04a — delivery with deferred local Miri

Status: explicit human sequencing override, October 11, 2026. Linked to
[#16970](https://github.com/adhithyan15/coding-adventures/issues/16970) and
[#15830](https://github.com/adhithyan15/coding-adventures/issues/15830).

The user requested finishing implementation and delivery before returning to the
long-running local Miri run. The local canonical Miri pre-publication gate is
therefore deferred. Required CI and branch protection remain binding; this
specification changes delivery sequencing, not safety or compiler semantics.

## Isolation and source receipts

Preserve the running `closurec-source-spans` checkout, its 497 frozen input hashes,
original logs, partial targets and failed/stale receipts. Do not terminate that
run merely to continue. Make changes in `closurec-source-spans-delivery`, on a
separate branch, with an explicit separate Cargo target directory. Never write
delivery outputs into the old target or relabel old tested heads as current.

Reconcile fresh fetched main, including both range capture and the independently
merged division-only JavaScript lexer/parser APIs. Preserve every upstream
behavior and the original production-limit/safety assertions. Run current
affected shared/compiler/downstream suites and strict lint; execute newly added
native regression assertions explicitly when production BUILD filters omit them.
Run the current Go tool's full test suite and vet, rebuild it, and verify its
actual non-forced production plan selects Closurec on all three native platforms.
Planning is not CI success. Retain applicable known failures and source evidence.

## Delivery and completion

Before pushing, obtain independent security review of the exact final candidate.
Keep one active implementation PR and attach it. Every affected PR's validation
summary must say local full Miri **DEFERRED / PENDING**, never passed. Enable squash
auto-merge only with current conflict-free, review-clean, exact-head terminal
acceptable checks; verify actual three-platform native compiler CI and fetched
main merge reachability. Do not bypass required checks or a demonstrated applicable
undefined-behavior failure.

After implementation delivery, resolve the expensive Miri bottleneck and obtain
full genuine final-source Miri acceptance. Any changed relevant source/runner/
dependency inputs require a new run or explicit equivalence proof; the old
unfinished historical-source run cannot certify later delivery. Keep the backlog,
issues and heartbeat active until deferred Miri plus full Closure parity,
output-byte/source joins and transformation lineage contracts are achieved.
