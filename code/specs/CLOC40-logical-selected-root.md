# CLOC40: literal logical selection owns its returned root

Status: contract selected; implementation and delivery OPEN.

At audited local source94fdd4e0, fold_logical folds left then right. Its
literal-choice arm forks a parent-only identity into unused _new_cv and returns
the chosen operand retaining its old root. CLOC36/CLOC37 already supply the
exhaustive native root slot and checked selected-subtree collector. CLOC38 and
CLOC39 repair distinct conditional paths; do not extend their admission here.

## Behavior and exact frontier

Change only the existing kept Some(Side::Left/Right) arm. Preserve every
literal_truthy/literal_nullish decision and existing eager left-then-right
folding, including successful unchosen right work and first checked failures.
Truthiness recognizes Boolean/Numeric/String/Null; NaN and either zero are
falsy. Nullishness recognizes Null as true and Boolean/Numeric/String as false.
Undefined, BigInt, RegExp and composites still decline these predicates; an old
comment mentioning undefined is not permission to broaden behavior. Keep
same-operator right-nest reassociation unchanged and explicitly OPEN for its
own synthetic inner-node/lineage contract.

For an admitted choice, first-seen distinct available parents are:

1. Original logical-expression CV.
2. Actual folded left operand CV (including a freshly derived arithmetic CV).
3. Complete actual chosen subtree in CLOC37 native declaration preorder.

Use the existing singleton collector with prefix
[logical.cv, folded_left.cv, None]. The chosen-left case aliases the second
prefix slot; first-seen deduplication must retain that identity once. A chosen
right opaque composite must retain its existing root, nested declaration/body
CVs and newly folded descendants as graph parents. The unchosen right subtree
is excluded from the immediate semantic frontier, although its prior fold
activity remains in the shared graph/journal. This is rewrite evidence, not a
claim that JavaScript evaluates a short-circuited operand.

Multiple distinct IDs merge once then derive/contribute; one derives directly;
zero creates neither identity nor fold contribution. Start ownership assertions
at the actual returned typed root and verify its own contribution/parent join.
Stamp only that root after checked success. Preserve every nested CV and all
non-CV data/raw spellings/numeric bits. Keep existing labels, including their
chosen-operand convention: before is `(lit_label(chosen)) operator (...)`, after
is literal_label_for_expr(chosen). Do not switch the before label to the left
test while fixing lineage. Untraced emitted JavaScript stays byte-identical.

## Refusal and resource boundaries

Reuse fork_selected_subtree_cvs and selected_root_identity::slot. Capture the
folded-left identity before moving either owned operand into chosen. If a child
already latched cv_error, skip outer collection/graph mutation and retain that
first error. Collector refusal must latch before the outer graph mutation;
graph merge/derive/contribution refusal rejects the pass candidate. Earlier
successful child work is retained; no rollback or deletion-event claim.

Use unchanged C37 default work/requested-storage caps, typed exhaustive walker,
count/replay, first-seen collision accounting and fallible allocation. Preserve
C39's three-tree controls and singleton exact costs. This slice introduces no
new collector API or traversal, no unmetered Vec/contains operand helper, AST
serialization/clone walk, new dependencies or production-limit changes. The
collector's own temporary-storage bound does not bound recursive folding,
cloning, later equality, whole-process memory or downstream consumers.

Failed pass candidates publish none of JavaScript/map/manifest/trace; all four
existing destination sentinel byte strings must survive. A partial returned
Program is not acceptance.

## Tests before implementation

- Compiled exact returned-root regression for all three operators and both
  kept sides, logical CV Some/None. Right-selected native36 root shapes,
  left-selected admitted literal families, raw payload and populated nested
  identities. No claim that every native shape is parser-admitted JavaScript.
- Distinct, duplicate, absent, single and no-ID controls, including chosen-left
  alias dedup. Hand-built exact expected parent lists, not production collector
  output as the test oracle. NaN/infinities/both zero signs/astral strings,
  unchanged declined Undefined/BigInt/RegExp/composite cases.
- Folded left test, selected opaque call/function body with a rewritten child,
  ordered frontier/root-to-child ancestry and exact child-before-outer clocks.
  Successful unchosen-right activity excluded from semantic parents.
- Genuine checked child failures from left and right, including the unchosen
  right operand and an ample-capacity missing-parent failure; assert first
  error/no outer graph work, preserving earlier successful left activity.
- Outer merge/derive/event refusals for every admitted side/operator; reuse
  independent existing exact/one-less collector work/storage/collision/
  allocation/overflow/deep/wide tests without lowering coverage or caps.
- SIMPLE/ADVANCED actual CLI physical-token intervals, exact chosen/left
  parents, own pass/sweep/merge/derive/contribution ordering, traced/plain byte
  equality, and all-output sentinel controls for outer and child refusals.
- Affected parser/folder/CLI/native/compiler/downstream suites, all-target
  strict checks and independent exact-head source/terminal reviews. Keep
  executed Rust head, later docs carrier and live published PR head distinct.

## Delivery and remaining contract

Fresh audit2026-10-17T23:18:37Z: origin/main7aa8396d2dd7b81687168722ba71e95b0650101f;
sole open Closure/shared PR17288 published6132a518b3990619eb904397ee0b21a69ce8ac20,
CONFLICTING/zero checks/autoOFF. Only maina2be13f7 has been reconciled locally;
newer main changes require their own safe reconciliation and affected tests.
No source/index merge is attempted through previously demonstrated unsafe
main-import findings. Contract selection is not publication or CI acceptance.

CLOC27 leaf-token provenance is real; the epic's old all-None bridge audit is
stale. Use actual available token/array/member CVs and never fabricate missing
logical/composite source identities. Base ESTree is the canonical serialized
boundary; own types/no new crates.io dependencies remain owner decisions.

Other folds/reassociation, typed role-rich folding/renaming/inlining/motion/
merging/deletion, stable identities/source spans, full output-byte/source and
pass/sweep lineage, evidence-backed Closure parity, current-main reconciliation,
conflict-free required CI, actual merge reachability and delivery remain OPEN.
Miri DEFERRED/PENDING, never passed; frozen run/checkout/inputs/target/receipts
remain untouched and unpolled. Genuine final-source Miri follows implementation
delivery. Full objective OPEN; heartbeat ACTIVE.
