# CLOC39: equal-branch conditionals join all three folded subtrees

Status: contract only; implementation, execution and delivery OPEN.

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
