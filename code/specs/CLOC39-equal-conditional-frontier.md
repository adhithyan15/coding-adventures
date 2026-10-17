# CLOC39: equal-branch conditionals join all three folded subtrees

Status: equal-conditional slice locally validated; delivery and full completion OPEN.

CLOC38 covers literal truthiness selection. This separate slice covers only
fold_conditional's existing equal_fold_branches path after literal_truthy has
declined. The live equality helper calls the native borrowed eq_ignoring_cv
for all represented fields, including composites: stale comments suggesting
that only primitive equality ignores CVs are not the implementation contract.
Preserve the equality implementation, its bounds and signed-zero/NaN behavior,
and is_side_effect_free exactly. This contract broadens no JavaScript admission.

## Operand order and ownership

Fold test, consequent and alternate in that existing order before admission.
When the existing equality gate succeeds, collect available identities in this
order: original conditional CV, folded test native subtree preorder, folded
consequent native subtree preorder, folded alternate native subtree preorder.
Every subtree includes its own root and all descendants even when its root CV
is nonempty; native declaration order and first-seen dedup apply across all
three trees. Never invent missing original or token identities.

These parents are rewrite-evidence operands. Test identities retain evidence
for the dropped pure evaluation or retained impure evaluation; both equivalent
branch trees retain their distinct source/rewrite histories. Joining alternate
identities does not claim that both branches execute at runtime. This slice
does not add typed operand roles or deletion events; those broader contracts
remain OPEN. Do not reuse C38's literal-choice exclusion of the other branch.

For a pure test, return the existing folded consequent and stamp only its actual
root via the exhaustive native root accessor. All nested CVs and non-CV payload
remain exact. For an impure test, return the existing SequenceExpression form
with test then consequent, stamping the new sequence root and preserving all
nested test/consequent identities and payload. The alternate is not emitted.
Keep labels exactly before='t ? X : X', after='X' or '(t,X)'. No available IDs
means no identity/contribution; one unique ID derives directly; multiple IDs
use one ordered checked merge, then derive, then own folded contribution.

An already latched error must skip outer collection/graph work. Collection,
checked merge/derive/contribution errors latch and reject the PassOutput and
all publication destinations. Earlier successful child activity is retained;
there is no rollback or claim that a refused rewrite executed successfully.

## One budget across multiple roots

Reuse C37's exhaustive borrowed native scanner, resumable list frames and
first-seen hash table. Generalize its internal count/replay orchestration to a
borrowed fixed three-root input, not three independent collector calls with
three production allowances. Count every tree before allocating combined
ordered-reference/hash buffers. Reuse the same frame stack across trees and
replay. All scans, hashing/equality/initialization and actual prefix slots
share ONE checked work budget; all simultaneously live frame/reference/bucket
buffers and old/new frame migration peaks share ONE requested-storage cap.

For compatible scan reuse, each root scan retains three metered prefix slots.
The test scan receives [original_conditional_cv, None, None]; the consequent
scan receives [None, None, None]; the alternate scan receives [None, None, None].
These exact arrays apply in both count and replay. The original conditional
is visited once per complete count/replay scan, never once for each tree. This bounded constant work is explicit,
not an unmetered synthetic AST or a cloned/sibling queue. Single-subtree C37/C38
callers retain their exact prior work/storage/allocation behavior. No new
crates.io dependency, wire/public type, production limit or pass/sweep change.
This bounds collector work/requested buffers, not the existing recursive fold,
AST clone/equality, caller-owned AST or system allocator.

## Executable acceptance

- Tests first: pure and impure cases with distinct original/test/branch CVs,
  composite roots and child rewrites in both equal arms, exact ordered parents
  and actual returned-root ancestry. Preserve selected/test nested payload/CVs.
  Exercise available/missing/duplicate/one/no-ID positions and retained native
  root shapes. Existing unequal/literal/side-effect/equality guards remain.
- Independently derived multi-root exact/one-less work and storage controls,
  repeated/colliding IDs and aggregate-allocation failures. A budget sufficient
  for each independent subtree must still refuse the combined work. No-ID
  walks still charge all trees; prior C37 exhaustive/all-field/deep/wide and
  C38 Unicode/production-limit controls remain unchanged and passing.
- Actual checked child errors originating separately in test, consequent and
  alternate: exact first error, no later outer merge/derive/contribution,
  successful earlier child activity retained when present, and ample-capacity
  failure controls to avoid vacuous exhausted-budget assertions. Outer checked
  merge/derive/event failures and all-output byte sentinels also remain covered.
- SIMPLE/ADVANCED traced/plain output equality, distinct physical token source
  intervals for BOTH emitted-equivalent arms, exact child-before-outer and own
  pass/sweep merge/derive/contribution clocks. Prove all semantic parent IDs,
  not merely source token presence. No general output-byte/source theorem.
- Affected parser/folder/CLI/compiler/downstream tests and strict lint;
  independent exact-head source/security and consumed terminal review, distinct
  later documentation head and actual published PR. Preserve failed/withheld
  receipts with precise execution-versus-source attribution.

Current main and sole Closure/shared PR require fresh audit before delivery.
Equal-branch input joining is a bounded fold slice; typed roles/deletion records,
other transformations/rename/inline/motion/merging, complete output byte/source
mappings and evidence-backed Closure parity remain OPEN. Base ESTree canonical
boundary/own types/no new crates.io preserved. Miri DEFERRED/PENDING; frozen run,
checkout/inputs/target/receipts untouched/unpolled. Genuine final-source Miri
follows authorized implementation delivery. Full objective OPEN; heartbeat ACTIVE.

## 2026-10-17: equal-conditional three-tree slice locally validated

Actual full/runtime execution `2cb61536f9e1a8408b6890df2b4afb4ea9ae6e42` follows CLOC39 contractc03ebe94, reviewed
before code. Existing literal-truthiness/equality/side-effect admission and
test/consequent/alternate folding order remain. The checked fold joins available
original conditional then complete folded test/consequent/alternate native
preorder, with first-seen dedup across all trees. Both equal arms provide
rewrite/source evidence, without claiming both execute. Pure tests return the
existing consequent with only its actual root stamped; impure tests retain
SequenceExpression[test,consequent] and own the new sequence root. Nested CVs
and non-CV payload remain; labels stay t ? X : X -> X or (t,X). No IDs yields
no identity/contribution; one ID derives directly. Typed roles/deletion events
and general output-byte/source mappings are not added by this slice.

One Budget/frame stack covers all count roots, combined raw-count reference
and bucket allocation, then all replay roots. Test prefix is conditional/None/
None and both branch prefixes are all None in each scan. The original conditional
is not recharged per tree. C37 singleton work/storage and exhaustive typed field
walker are unchanged. All live buffers and old/new frame migration peaks are
metered; no synthetic/cloned AST, root heap queue, new crates.io dependency or
production-limit change. This bounds collector work/requested buffers, not the
existing recursive folder/equality/clone or caller/system memory.

Tests-first811 compiled1P4F red: twelve child-error contexts completed, while
four failures stopped other loops early. Resources/CLI32bd were committed and
statically reviewed before production821. Focus821 really passed12 new and5
old array functions, but its strict101 chunks lint made aggregate acceptance
WITHHELD. The2cb tests-only array-chunk repair retained every assertion and
added an empty-remainder check; the failed receipt remains failed. An initial
2cb disk guard stopped before receipt/Cargo/target writes; reclaiming only
regenerable PDB symbols in the separate owned target restored space without
lowering the guard or touching executables, source, receipts or frozen Miri.
None of this older evidence transfers final acceptance to `2cb61536f9e1a8408b6890df2b4afb4ea9ae6e42`.

Actual full suites:282 parser+490 folder+1172 CLI+1006 native=2950P/0F,
two unchanged historical ignores,195 executable joins/three empty doctests;
both all-target strict checks complete0. Native144 root-shape cells,32 ID
availability/duplicate cells, both rewritten arms plus complete test descendants,
twelve first-child error contexts and ten outer graph/event refusals pass.
Seven new resource functions prove independently derived104/121/31/100 work,
exact/one-less requested storage, combined raw duplicate capacity, cumulative
rather than per-tree allowances, all three allocation refusals, and three-tree
4096-deep/100000-wide collection on144KiB stack with iterative disposal.
All old C37/C38 populated/Unicode/budget/allocation/guard controls remain.

SIMPLE/ADVANCED pure/impure CLI cells join BOTH distinct physical observe-token
intervals, both real rewritten argument IDs and test evidence in exact order;
child-before-outer and own merge/derive/contribution pass/sweep clocks and
traced/plain emitted bytes match. New48 outer and144 test/consequent/alternate
child merge/derive refusal cells preserve all four publication sentinel bytes.
Equal duplicate labels are selected by actual derive-event order; prior helper
assertions/default single-occurrence controls are preserved. Cell counts are
loop evidence, not additional unique test-function totals.
Core `0df045615db01c8109473df9b775279d6e58a1534222c2a24484e9e2cff3633f`, source `04d4ecd304484a6975b4468ac567921980d50992a9b0b06b6e64f1d4b2122c00`,
combined terminal `d2fbbab31e0f1456dbbcbb6b23171ea05b5c36026ddda560ffa6fd28933f7993` bind physical source/locks/logs/artifacts.

Separate same-head runtime `400860da6d60ac2b68002becadd8f592a37ee3a0fb4b5aad315db577e2982fe5` retains59 declared sources,
354 ordered six-backend observations/54 exact VM-JIT Float-bit captures;
integration4P/observer1P/refusal1P/strict0. LANG526 filtered, CLR absent/uncounted,
native/LLVM typed-main ABI OPEN; no full LANG/raw-output-byte/precision theorem.

Audit 2026-10-17T23:08:17.375524+00:00: main `7aa8396d2dd7b81687168722ba71e95b0650101f`; sole Closure/shared
PR#17288 published `6132a518b3990619eb904397ee0b21a69ce8ac20`, UNKNOWN/
0 checks/autoOFF. No push/hosted acceptance/merge.
Remaining fold/rename/inline/motion/merging/deletion roles, complete pass/output
byte/source joins, parity and current-main delivery remain OPEN. Base ESTree
canonical boundary/own types/no new crates.io preserved. Miri DEFERRED/PENDING;
frozen run/checkout/inputs/target/receipts untouched/unpolled. Genuine final-source
Miri follows implementation delivery. Full objective OPEN; heartbeat ACTIVE.
This later Markdown carrier claims no additional Rust execution.
