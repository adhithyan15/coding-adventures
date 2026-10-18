# CLOC41: unary Math replacements retain their available evidence

Status: specification selected; implementation, delivery and full objective OPEN.

Read-only source51cfed62 confirms that fold_call folds its callee then every
argument before the existing abs/floor/ceil/round/trunc/sign handler. That
handler derives only from the original call.cv. The ordinary parser bridge
constructs CallExpression.cv=None, while CLOC27 token CVs and CLOC35 member
occurrences already exist. A successful Math.abs(-5) replacement can therefore
lose its own event and its child's history. Do not repeat the epic's stale
all-None leaf audit or manufacture a new call/property identity here.

## Fixed frontier and unchanged semantics

Change only the existing admitted six-name, one-NumericLiteral result arm.
Preserve folding callee then arguments, admission guards, labels, numeric
value bits, raw formatting, finite checks, negative-zero refusals and all
declined cases. The existing textual Math matcher does not certify a global
binding; this slice neither broadens admission nor claims scope certification.
The finite check applies to the result, not every input: existing Math.sign
of positive/negative Infinity yields positive/negative one and remains admitted.
Retain both complete cases as exact positive value/lineage tests; do not turn
their input nonfiniteness into a new refusal or assert a blanket decline.

For the actual folded shapes, first-seen distinct available parents are:

1. Original call.cv.
2. Folded MemberExpression.cv.
3. Folded Math receiver Identifier.cv.
4. Folded property Identifier.cv.
5. Folded NumericLiteral argument.cv.

On the ordinary CLI dot-member bridge, the call and property fields are None.
The three immediate parents are the observed member occurrence, actual Math
receiver token and folded numeric argument. The property's physical token
still reaches the result through the member's exact prefix parents; its
presence in the log is not permission to invent a direct property-field CV.
Hand-built native nodes with a real property CV retain that fourth field.

Use five borrowed stack slots and at most 25 equality comparisons. No arbitrary
subtree walk, unbounded frontier allocation, AST serialization or general call
collector is needed. Deduplicate aliases in declared order, including repeated
equal spellings whose distinct IDs must remain distinct. Zero available IDs
creates no graph identity/event; one derives directly; multiple IDs merge once,
then derive. The returned NumericLiteral must own exactly one folded
contribution whose parent_cv/new_cv fields agree with actual edges. Preserve
every nested child rewrite and exclude unrelated token/source identities.

## Failure and chronology

If child folding already latched cv_error, perform no outer collection or
graph work and preserve the first error. Use existing checked merge, derive
and contribution APIs/error flow. Missing parents and entry/event/payload
exhaustion reject the pass candidate; do not silently publish a value whose
lineage failed. Earlier successful child graph activity is retained, with no
rollback or deletion claim. For admitted success verify child event < outer
merge (if any) < derive < own contribution, including real pass/sweep scopes.

All four pre-existing JS/map/manifest/trace destination sentinel bytes survive
failed CLI candidates. Normal typed pipeline failure, rather than a panic,
missing binary or arbitrary nonzero status, is required. Existing graph,
production-limit, parser/source-ownership and CLOC31–40 controls stay intact.
The existing typed CLI stage is spelled "optimization pass". Outer merge and
derive node refusals must retain that stage and their distinct checked causes.

## Tests before implementation

- Compiled semantic red for absent call.cv with available member/receiver/
  property/argument IDs, all six methods, typed numeric bits/raw fields and
  exact returned root -> merge -> declared parent list.
- All 32 available-slot masks, duplicate IDs and first-seen permutations,
  zero/single-parent behavior, distinct same-spelling roots, nested unary and
  binary child history, unrelated exclusion and acyclic/dangling-free graphs.
- Genuine missing-parent, merge/derive/contribution budget refusals and prior
  child failure; assert no outer work after a latched error. Preserve wrong
  arity/nonliteral/non-Math/nonfinite-result/signed-zero declines and child
  effects, with the existing sign-of-Infinity admissions above preserved.
- Actual SIMPLE/ADVANCED CLI ingestion with Unicode/CRLF prefixes and repeated
  Math expressions: exact physical source intervals/coordinate ownership,
  member-prefix evidence, result-owned events, child/outer pass/sweep clocks,
  traced/plain JS/map equality and all-artifact failure sentinels.
  Keep the emitter's existing multi-statement formatting: concatenated report
  statements and one final LF. The independent numeric/order table must not
  invent per-statement newlines and fail before the lineage assertions.
- Full affected folder/parser/CLI/downstream suites, strict all-target lint,
  independent exact-head source and terminal security review. Keep actual Rust
  execution, later documentation carrier and published PR head distinct.

## Delivery boundary

Fresh2026-10-18T02:36 audit: origin/main35765565386d8d49223531d73eccf02499afd2bc;
sole Closure/shared PR17288 publishes51cfed62c7a2291fcce798b5861a3ca7b74d591c,
conflicting/zero hosted checks/auto-merge OFF. Main native33 and the existing
shared-crate reconciliation inventory remain unaccepted; no raw merge/import
is justified by this lineage slice. CLOC41 was selected above the current
tracked CLOC40 maximum after repository/issue allocation audit.

Base ESTree remains the canonical serialized boundary, with own types and no
new crates.io dependencies. Other Math/call/composite families, stable complete
node occurrences, folding/rename/inline/motion/merge/delete roles, full
pass/sweep/output-byte/source joins, Closure parity, current-main reconciliation,
required conflict-free exact-head CI, actual merge reachability and delivery
remain OPEN. Miri DEFERRED/PENDING, never passed; frozen checkout/run/inputs/
receipts/target untouched and unpolled. Genuine final-source Miri follows
implementation delivery. Full objective OPEN; heartbeat ACTIVE.


## CLOC41 bounded local validation

Actual Rust execution `fa8b81c2a8c7408ec31114617a7c176f7a5cbffd` completed 3277 native tests/doctests,
zero failures or filtered tests and 41 existing ignored cases,
across 248 metadata-declared executable harness targets and
14 separate doctest targets. Both affected all-target strict checks
passed. The selected reverse cone plus parser/CV covers fourteen shared
packages and the CLI, with thirty-six forward input packages; all 1,293 Rust
manifests and complete dependency declarations were re-audited. Focused CLOC41
native/CLI tests are included in those totals, not additional counts.
Source review `6fa6977512742b052856543517bcca112d157c5009e4a3d28701466194d0d0b6` and terminal review
`c1dd1b03d23a33f239537877374f04695097f082c901244c81d2f7198a85f86d` bind this bounded local execution. Core receipt
`31126ef9572a1297bfba5e9d8ebf32eb799e54c161dc5de58494e3561d12e382`. This later Markdown update claims no Rust re-execution.

The existing abs/floor/ceil/round/trunc/sign numeric handler now joins
available original-call, folded-member, receiver, property and folded-argument
identities in that order, with first-seen alias deduplication. Five borrowed
stack slots bound frontier storage and comparison count; this is not a bound
on arbitrary ID byte lengths, recursive folding or whole-process memory.
The returned numeric node owns its checked fold contribution; nested child
history and real pass/sweep order survive. Missing IDs are not manufactured.

Admission, numeric bits/raw formatting, labels, negative-zero/result-finite
guards, eager child folding and first-error behavior remain. Native
sign-of-Infinity inputs still yield positive/negative one. Textual Math
matching does not certify a global binding. Actual ingested source intervals,
Unicode/CRLF and repeated spellings, traced/plain JS/map/manifest equality,
checked graph/resource failures and all-artifact refusal sentinels are tested.

This is bounded local CLOC41 lineage, not complete source/output mapping,
all call/composite families, full transform-role lineage or Closure parity.
Current-main reconciliation, conflict-free exact-head required CI and actual
merge/delivery remain OPEN. Base ESTree stays the canonical serialized
boundary, with own types and no new crates.io dependencies. Miri
DEFERRED/PENDING, never passed; frozen run/checkout/inputs/target/receipts remain
untouched and unpolled. Genuine final-source Miri follows implementation
delivery. Full objective OPEN; heartbeat ACTIVE.

Compiled causal red at638573d6 built actual native4P9F472filtered and CLI1P3F;
independent terminal0a6ec2e4 verified the intended missing-result/parent causes
and passing safety/decline controls. Earlier test-oracle/setup failures remain
unaccepted: sign-Infinity admission, Windows verbatim glob paths, and expected
per-statement LF. Exact failure names/counts alone did not prove failure cause.

The first full-green attempt at35781 remained unaccepted after new CLI oracles
mistook a transitive property token for a populated immediate node field and
used the wrong typed stage text. Production and complete native tests stayed
unchanged; corrected CLI assertions preserve exact property ancestry and
distinct checked merge/derive causes. The successor executes the new target
first, then the same complete affected target/doctest/strict set.

The second full-green attempt at49b completed all 248 executable harnesses and
14 doctest targets, but shared strict failed on two replace_box test-fixture
assignments and CLI strict was not executed; its aggregate remains unaccepted.
The test-only fa8b successor changes only those two existing Box inner
assignments plus a filled lesson. Its quick two-strict run passed, separately
from the new full-suite attempt; neither old test results nor quick lint alone
accept the successor. Final results below belong only to their exact executed
head and actual consumed full receipt.
