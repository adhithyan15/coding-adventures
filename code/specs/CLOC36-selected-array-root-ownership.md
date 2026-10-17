# CLOC36: actual selected array replacement roots

Status: contract first; implementation and validation pending.

CLOC35 certifies original member occurrences, including their complete physical
token intervals. It does not repair an in-bounds array selection that calls
`fork_cv`, discards its result and returns `selected.clone()` with its old CV.
A journal branch is not the returned node's identity. This slice repairs that
actual root and its available immediate operand identities.

## Bounded scope and explicit remaining gap

The preceding design selection combined root ownership with a general selected
subtree lineage collector. Keep these independently reviewable: CLOC36 handles
the root and four fixed operand positions; a subsequent contract must define
bounded descendant/frontier traversal through opaque composites, functions,
classes and statements. No recursive walk, serialization, registry search or
variable-width operand collector is introduced here.

All 36 existing Expression variants receive exhaustive private root CV access
and stamping decisions. This is accessor coverage, not permission to select all
36 from an array: the existing spread guard still refuses an array containing
a SpreadElement, and all other semantic guards remain authoritative. Actual
eligible selections preserve every non-CV field and every nested CV. An opaque
selected composite's rewritten descendants keep their own identities in the
returned subtree; CLOC36 does NOT claim the new root reaches every descendant
rewrite through parent_ids. That missing root-to-descendant closure remains
OPEN. Truthy conditional selection/discarded forks also remain OPEN.

## Original source and transformation identities

For a successful present in-bounds selection, collect these four optional CVs
in this exact order, after ordinary bottom-up object/property folding:

1. Original/current member root.
2. Folded array object root.
3. Actual folded numeric or string key root.
4. Actual selected element root, including an earlier fold's derived identity.

Keep first-seen distinct IDs, omitting absent positions. The existing metered
array frontier collector checks slot/ID work, storage arithmetic, fallible
allocation, hashing and equality before the outer graph mutation. Its limits
remain per collector; this does not establish a bound on the entire pass.
Do not use the unmetered variable-width Vec/contains path for a new traversal.

With multiple available IDs, create one ordered merge, derive one replacement
from it and attach the existing constant-fold/folded contribution to that
replacement. With one ID derive directly; with none preserve untraced behavior
and create no identity or contribution. The actual returned root MUST contain
the derived ID, never the selected element's previous root or an unused log ID.
The contribution retains before=array-literal[len][index], after=element[index]
and exact parent_cv/new_cv. No new contribution schema or event kind is added.

Numeric-string coercion must pass the ORIGINAL folded string key identity
separately to the shared index helper. The temporary numeric value remains an
untraced internal conversion, not a fictitious source numeric node. Direct
numeric access passes its actual folded numeric identity. This must preserve
key-fold history as well as selected-element fold history.

Exact immediate semantic parents exclude unselected sibling roots. Original
array/member construction intervals legitimately include sibling tokens: do
not confuse that physical ancestry with the four semantic operand positions,
and do not weaken complete physical-interval tests to manufacture exclusion.

## Semantics, failures and compatibility

Keep existing no-spread, finite integer, bounds, numeric-string conversion and
other-elements-side-effect guards. A selected call or mutation keeps its own
effect; an effect in another element prevents selection. Hole/out-of-bounds,
array length and other member families retain their existing behavior and
identity contracts in this slice. Optional-member execution rules are unchanged.

Private exhaustive accessors clone or borrow only the root CV; stamping changes
only that slot on the already selected clone. No public AST field/type, base
ESTree serialized boundary, dependency, lock, CLI flag, production safety limit
or tracing-disabled JavaScript behavior changes. Own types and no new crates.io
dependencies remain owner decisions.

Collector errors latch cv_error before any outer selection merge/derive/event.
Checked graph node/edge/event/payload failures reject the pass candidate through
the existing error path; earlier child graph work is not claimed rolled back.
The CLI must publish none of JS/map/manifest/trace after such a failure. No
partial returned Program or failed journal branch is counted as acceptance.

## Required executable evidence

- Tests committed before implementation. A compiled regression must fail from
  the ACTUAL returned typed Program root, not by searching for a folded record.
- Exhaustive 36-variant root coverage with distinct original CVs and non-CV
  equality; native eligible selected leaf/composite/effectful roots; unchanged
  spread refusal. Include absent root CVs and untraced inputs.
- Exact ordered merge/derive/contribution ownership for both key routes,
  deduplication/absent/single parents, earlier selected and key folds, and
  exclusion of unrelated semantic parents. Preserve nested child CVs and state
  the descendant-closure limit explicitly in its controls.
- Numeric/string coercion spellings, signed zero/raw strings, selected effects,
  sibling effects, spread/fractional/nonfinite/missing/hole/OOB controls. Compare
  traced/plain emitted bytes and source-map/manifest behavior where applicable.
- Checked graph failures and the reused collector's exact/one-less work/storage
  and collision controls; no new limits or weakened safety cases. Assert own
  merge/derive/event and pass/sweep context ordering in the final CLI trace.
- All-output sentinel preservation on applicable refused candidates; affected
  folder/parser/CLI/compiler/downstream suites and strict lint; independent
  exact-head source and terminal review before publication.

Keep execution head, later documentation head and published PR head distinct.
The existing sole Closure/shared-crate PR is retained; current-main conflicts,
required CI, review and exact merge reachability remain delivery gates.

Miri is DEFERRED/PENDING, never passed. Preserve its frozen checkout, run,
inputs, target and receipts; use the separate delivery worktree/target. Genuine
final-source Miri follows implementation delivery. Full transformation lineage,
output-byte/source joins and evidence-backed Closure parity remain OPEN.
