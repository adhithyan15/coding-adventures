# CLOC38: literal conditional selection owns its returned root

Status: literal conditional slice locally validated; delivery and full completion OPEN.

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

## 2026-10-17: literal conditional root slice locally validated

Actual full/runtime execution `e6c9cbd8e3089d3063b8c06204b6e1681fc37629` retains the existing literal truthiness
admission and test/consequent/alternate fold order. The selected expression's
actual root owns the checked fold when identities are available; ordered
semantic parents are available original conditional, actual folded literal test,
then complete selected native subtree preorder with first-seen deduplication.
No IDs means no new identity or contribution; one unique ID derives directly. Only the selected root is stamped;
nested CVs and non-CV payload remain. Unchosen branch activity precedes outer
selection but does not become a semantic operand. C37's bounded iterative
collector and C36's exhaustive root accessor are reused without limit changes.

Contract6e3 preceded code and independent review. Native69b compiled1P6F
expected red (first failures do not prove all144 shape cells). Production3c693
changes only the literal arm and private shared helper name; equal-branch path,
truthiness, labels, array guards, collector and root accessor remain unchanged.
Source review initially withheld the missing unchosen-error control; tests-only
successors add exact checked failure-origin evidence for both truthy choices.
A later test compile failure (JournalRecord lacks Serialize) and an unintended
Windows decoding rewrite of the astral fixture were repaired through the
supported complete canonical export and exact UTF-8 restoration. The2ba
compile receipt remains failed;14e full suites actually passed with an accepted
execution receipt, while independent SOURCE acceptance stayed WITHHELD for
encoding. Neither older result transfers acceptance to the corrected e6 head.

Actual full suites:282 parser+478 folder+1169 CLI+1006 native=2935P/0F,
two unchanged historical ignores,195 executable joins and three empty doctests;
both all-target strict checks complete0. Native144 selected-root shape cells,
16 duplicate/availability controls,12 truthiness families including exact astral
UTF-8, NaN/infinities/signed zero, prior test/call/function child rewrites,
unsupported unequal branches and checked outer refusals pass. Both truthy
choices also exercise real unchosen-child node-limit and ample-capacity forged
missing-parent failures: exact latched error and entire graph/chronology equality,
then successful outer merge/derive capacity in the latter control. No rollback
of earlier successful child activity is claimed.

SIMPLE/ADVANCED CLI cells join real physical token source intervals to exact
ordered parent IDs, preserve child-before-outer and branch order, own
merge/derive/contribution pass/sweep clocks and traced/plain emitted bytes.
48 outer conditional and48 unchosen-child merge/derive publication-refusal
cells preserve all four output sentinel byte strings, alongside24 retained C36
cells. C37 exact work/storage/overflow/allocation/deep/wide controls remain.
Core `33bb23c5fd25bf4e7e7ec315439f727b634b37eae4212aed32841da49cf8f284`, source `a6e2668d39c1d53824c32923331189e6226c71ec1878a0df186aedb7e0ead2e2`,
combined terminal `c4efba8a690a7c89b5cc22af22e455752107031d68b44196ab375cd512a69581` bind physical source/locks/logs/artifacts.

Separate same-head runtime `06eaba6c397b383f49992ac2b67339d6750056e6f0a219e19b5c6bcf22c6eafb` retains59 declared sources,
354 ordered six-backend observations and54 VM-JIT Float-bit captures;
integration4P/observer1P/refusal1P/strict0. LANG526 filtered, CLR absent/uncounted,
native/LLVM typed-main ABI OPEN; no full LANG or raw-output-byte theorem.

Audit 2026-10-17T22:33:28.981551+00:00: main `a5f78efb7188e06adfbd10432dc2387d54a35fd3`; sole Closure/shared
PR#17288 published `6132a518b3990619eb904397ee0b21a69ce8ac20`, UNKNOWN/
0 checks/autoOFF. No push/merge/hosted acceptance.
Equal-branch and other transformations, rename/inlining/motion/merging/deletion,
full pass/output-byte/source mappings, parity and current-main delivery remain
OPEN. Base ESTree/own types/no new crates.io preserved. Miri DEFERRED/PENDING;
frozen run/checkout/inputs/target/receipts untouched/unpolled. Genuine final-source
Miri follows implementation delivery. Full objective OPEN; heartbeat ACTIVE.
This later Markdown carrier does not claim another Rust execution.
