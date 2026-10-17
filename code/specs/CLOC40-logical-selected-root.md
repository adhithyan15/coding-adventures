# CLOC40: literal logical selection owns its returned root

Status: logical literal-choice slice locally validated; delivery and full completion OPEN.

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

## 2026-10-17: logical literal-choice slice locally validated

Actual Rust full/runtime source `a3ec4ef38dcf5cbc8856ab6dc9c99fcff5de0794` follows reviewed specbfcbb67f and compiled
tests-first076db132 red: actual buildtrue/Cargo101/2P7F463filtered; only child-
failure and declined-family functions completed. Failing loops receive no
coverage credit. Pre-code test review withheld0cc5fe45 for missing typed bits/raw
assertions on the right-native NumericLiteral fixture; the separate left-bit
table did not prove that direction. The tests-only repair preserves every JSON
payload/ancestry assertion and adds typed right value.to_bits()/raw checks plus
a filled lesson; the earlier withheld report remains withheld. Production
changes only the existing logical kept arm: capture
folded-left identity, collect original logical/left/chosen complete subtree with
first-seen alias dedup, then stamp the actual chosen root after checked success.
Existing labels, literal guards, left/right folding, nested payload and limits
remain. One/no available ID derives directly/creates nothing. Reassociation
synthetic identities and typed deletion/role events remain OPEN.

At2f1cd7f5 all2962 tests genuinely passed, but package-strict101/buildfalse
reported two unused_parens warnings in new test fixtures; CLIstrict was not
executed and aggregate acceptance stayed WITHHELD. Test-only a3ec4ef3 removes
only those two parentheses pairs and adds a filled lesson, retaining all literal
values/assertions/production code. Final full/strict/runtime results below are
new executions at `a3ec4ef38dcf5cbc8856ab6dc9c99fcff5de0794`, with no older-head acceptance transfer.

Actual full suites:282parser+499folder+1175CLI+1006native=2962P/0F,
two unchanged historical ignores/195 executable joins/three empty doctests;
both all-target strict checks actual0. Native nine functions complete216 native
right-root cells,36 explicit literal-side rows with typed numeric bits/raw and
astral UTF-8,48 availability/alias cells, populated call/function descendants,
unchosen-right history exclusion,24 child-failure cells/22 distinct configs,
and30 outer checked refusals. The two duplicated ??-right left-error cells
fail while a numeric left is being folded; they do not prove a nullish-right
admission result. Existing C37/C39 exact resource and Unicode controls remain.

SIMPLE/ADVANCED12 actual physical-token/pass/sweep/clock/traced-plain-byte cells
pass. Current CLI left-only selection has no invented logical CV and therefore
derives directly; right selection merges actual left/callee/rewritten-child
parents. New108 outer and264 child-origin all-output refusal cells preserve
JS/map/manifest/trace sentinel bytes. Every old default merge/derive assertion
remains. Cell counts are not additional unique test-function totals. The CLI
test writer's preparation guard gap was caught before execution and repaired
with exact HEAD/both preimages/all-proposals-before-write; a filled lesson
records that review finding and the earlier pre-write quoting syntax error.

The first docs writer then refused an LF-only raw title anchor against the
backlog's existing CRLF bytes. It failed before creating the plan or mutating
any source file; no docs commit or acceptance resulted. The byte-aware successor
preserves the exact existing title/EOL prefix and records this caught failure
in the existing preimage lesson. This seven-Markdown carrier explicitly excepts
that lesson as well as the six spec/backlog/README/CHANGELOG inputs; it does
not assume Markdown inputs are inert or assign Rust execution to the carrier.

Core `acf6c943b72fc3e53f8cdcd0859860ce0b84f4fa9dd5088f137d28c2723463f3`; exact source `bce0bb70fdc93eb5f5fdeb72691c027e2fda1ba520ba8988a6bf8ec36bad56d3`;
combined terminal `61c60ac7b03428ec71a74f86174f4d32ebcb79d8d4849f272c6cb27960357a2f` bind real source/locks/logs/artifacts.
Separate same-head runtime `407ccd7892464bdc3f11adeeaab96ad7cc4530397e9272ed9a446e40c4b1e138` retains59 declared sources,
354 ordered six-backend observations/54 VM-JIT Float-bit captures;
integration4P/observer1P/refusal1P/strict0. LANG526 filtered, CLR absent/uncounted,
native/LLVM typed-main ABI OPEN; no full LANG/raw-output-byte/precision theorem.

Audit 2026-10-17T23:52:43.212328+00:00: main `7aa8396d2dd7b81687168722ba71e95b0650101f`; sole Closure/shared
PR#17288 published `6132a518b3990619eb904397ee0b21a69ce8ac20`, CONFLICTING/
0 checks/autoOFF. No push/hosted acceptance/merge.
Remaining fold/reassociation/rename/inline/motion/merging/deletion roles, full
pass/output-byte/source joins/parity/current-main reconciliation/delivery OPEN.
Base ESTree/own types/no new crates.io preserved. Miri DEFERRED/PENDING;
frozen run/checkout/inputs/target/receipts untouched/unpolled. Genuine final-source
Miri follows implementation delivery. Full objective OPEN; heartbeat ACTIVE.
This later Markdown carrier claims no additional Rust execution.


The intermediate carrier16236e2b actually passed doc-shards but failed lessons
with two canonical filename/title slug mismatches; diff was not executed and
both independent carrier reports stayed WITHHELD. The final Markdown-only
successor uses the repository slugify function to rename those two lessons,
preserving their complete bodies and recording this correction in the preimage
lesson. Its closed inverse includes eight materialized Markdown bodies and both
removed old names (ten changed paths), rooted at the actual a3 Rust execution.
This is not an inert-Markdown assumption or Rust execution at either carrier.
