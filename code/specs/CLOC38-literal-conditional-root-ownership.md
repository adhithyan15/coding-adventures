# CLOC38: literal conditional selection owns its returned root

Status: contract only; implementation, execution and delivery OPEN.

After folding a conditional's test and both branches, the literal-truthiness
case currently forks a CV into an unused local and returns the chosen branch
with its previous root. CLOC36/CLOC37 already provide exhaustive root access
and a bounded native selected-subtree frontier. Reuse those checked components
to repair this path; another traversal or identity schema is unnecessary.

## Bounded behavior

Change only the literal_truthy Some(true/false) branch of fold_conditional.
Preserve the current recognized Boolean/Numeric/String/Null cases, including
NaN and signed-zero numeric falsiness and empty/nonempty string behavior. Do
not add Undefined/BigInt/RegExp/object/array truthiness or broaden parsing.
Keep bottom-up test, consequent and alternate folding in that order, including
earlier work/errors in the branch not chosen. No rollback or deletion-event
claim is introduced. Equal-branch pure collapse and impure sequence lowering
retain their current behavior and separate OPEN lineage findings.

After those folds, gather optional identities in this exact order:

1. Original/current conditional root.
2. Actual folded literal test root, including an earlier test rewrite.
3. Actual chosen subtree in CLOC37 native field-declaration preorder, beginning
   with its current root and continuing through nonempty composite roots.

Use first-seen distinct IDs and the existing checked merge/derive/contribution
path. Multiple IDs merge once then derive; one derives directly; none creates
no identity or folded contribution. Preserve current before/after labels:
before is `(literal-test-label) ? ... : ...`, after is the chosen expression's
existing literal_label_for_expr value. Stamp only the actual returned chosen
root with that derived identity. Preserve all nested CVs and non-CV payload,
raw spellings, numeric bits, effects and branch identity. The returned root
must reach actual retained child rewrites, rather than an unrelated journal
record merely containing an after label.

Unchosen branch roots/rewritten descendants are excluded from the immediate
semantic frontier. Physical source ancestry may include the whole original
conditional interval where available; it is not a semantic sibling edge.
Do not invent original conditional/composite parse IDs: existing CLOC27 token
CV reuse and later certified array/member provenance are real, while some
other native composites still have no original root CV. That bridge gap in
the epic's old all-None audit is stale. Exact actual test/selected token CVs
and newly derived child IDs must remain connected even with no conditional CV.

## Limits and compatibility

Reuse CLOC37's borrowed collector with an unused third fixed-prefix slot and
unchanged default work/requested-storage caps. Its count/replay traversal,
all86 reachable type decisions,11 list frames, first-seen ordering, simultaneous
buffer/old-new growth accounting, collision charges and fallible allocation
remain unchanged. Do not substitute the older unmetered Vec/contains helper.
The helper may receive a generic private name shared with array selection;
keep that array path and its existing assertions behaviorally unchanged.

Collector refusals latch cv_error before the outer conditional graph mutation.
Checked graph limits still reject the pass candidate; earlier bottom-up child
graph work is not rolled back. A failed candidate must publish none of JS,
map, manifest or trace. No partial returned Program is acceptance. This slice
does not bound the existing recursive folder/cloning or total process memory.

No public AST/type/serialized boundary, crate dependency, lock, CLI flag,
production limit, pass/sweep order or tracing-disabled JavaScript behavior
changes. Base ESTree canonical serialization, own types and no new crates.io
dependencies remain owner decisions. Do not weaken production-limit tests.

## Tests before implementation and delivery evidence

- Commit a compiled returned-root regression before code. Assert exact own
  merge/derive/contribution and actual typed-root identity, not token presence
  or an unowned folded event. Both truthiness choices and root Some/None.
- Actual selected native root shapes; preserve non-CV structure and nested
  identities. Reuse the exhaustive accessor/collector fixtures without treating
  accessor coverage as a broader JavaScript syntax admission theorem.
- Distinct and duplicate/absent/single/no-ID positions, prior test rewrites,
  selected opaque call and function-body rewrites, exact preorder and root-to-
  rewritten-child ancestry. Prove unchosen semantic roots are not operands;
  retain earlier unchosen branch activity and error behavior explicitly.
- Existing Boolean/Numeric/String/Null truthiness, NaN/negative zero, empty/
  nonempty strings; unsupported test families still decline this path. Pure
  equal-branch and impure sequence cases retain their existing semantics.
- Checked outer merge/derive/event refusals; retain C37's independent exact/
  one-less work/storage/collision/overflow/allocation/deep/wide controls. CLI
  all-output sentinels and SIMPLE/ADVANCED traced/plain emitted-byte equality,
  exact parent order, child-before-outer and own pass/sweep/clock ordering.
- Affected parser/folder/CLI/compiler/downstream suites and all-target strict
  lint; independent exact-head source and terminal reviews. Keep execution,
  later documentation and current published PR heads distinct.

One active Closure/shared PR remains. Current-main reconciliation, conflict-
free review-clean required CI and actual merge reachability remain delivery
gates. Equal-branch/other folds, typed role-rich transformations, rename/inline/
motion/deletion, full output-byte/source joins and evidence-backed Closure
parity remain OPEN. Miri DEFERRED/PENDING, never passed; frozen run/checkout/
inputs/target/receipts untouched. Genuine final-source Miri follows implementation
delivery. Full objective OPEN; heartbeat ACTIVE.
