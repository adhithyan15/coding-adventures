# CLOC37: bounded selected-subtree identity frontier

Status: selected-subtree slice locally validated; delivery and full completion OPEN.

CLOC36 stamps the actual root returned by a present in-bounds array selection.
Its four immediate operands do not establish ancestry to rewrites inside an
opaque selected composite. In the existing selected CallExpression witness,
the returned call retains a newly folded argument, but the original call CV
need not reach that newer argument. Keeping a child CV is not a root-to-child
lineage proof. CLOC37 closes this selected-subtree gap without changing JS.

## Boundary and existing provenance

Current CLOC27 leaf-token reuse is real: parse_javascript_typed_with_cv stamps
physical token CVs and the native bridge consumes them. CLOC33/CLOC35 also
certify closed original array/member intervals. The epic's earlier all-None
bridge audit is stale; do not replace this existing source provenance with
invented IDs or count physical sibling tokens as semantic selected operands.

Only the successful present in-bounds array-element selection path changes.
Numeric and coerced numeric-string keys share this path; keep the actual
folded string CV separate from the untraced temporary numeric value. Hole,
OOB, length, optional execution, spread, index conversion and sibling-effect
guards retain their existing contracts. Conditional selected-root ownership
is separate OPEN work. No new public AST field, wire schema, crate dependency,
CLI option, production safety limit or selection eligibility is introduced.
Base ESTree remains the canonical serialized boundary; use our own AST types.

## Exact semantic parents

After bottom-up object/key folding, before cloning or outer graph mutation,
visit these optional identities in the following order:

1. Current member root, folded array root, actual folded key root.
2. The actual selected subtree in depth-first preorder: each CV-bearing
   native struct's own CV before any children, including the selected root.

Children follow their AST struct's field declaration order, ignoring non-AST
payload fields; list elements follow source vector order. Enum wrappers add
no CV of their own and delegate to their selected variant. Visit every optional
child/list slot, including absent fields and array holes, for budget purposes;
an absent identity supplies no parent. Do not stop at a nonempty composite CV.
First-seen distinct IDs form the exact ordered frontier. Duplicate IDs are
deduplicated by string equality, not address or graph topology. No transitive
reduction, graph/registry scan, historical contribution search or invented
identity is permitted.

The borrowed walk must cover all types reachable from Expression, including
Property/ObjectMember/PropertyKey/PrivateName, template quasis, function
parameters/defaults/rest nodes, assignment/binding targets, function and arrow
bodies, class members, statements and declarations. Exhaustive enum matches
and full struct destructuring must force an explicit decision on added AST
variants/fields. Non-CV scalar payloads are explicitly ignored. Cover only
the selected subtree: no inspection of an unselected semantic sibling is
introduced by this collector. Original member/array physical construction
ancestry can legitimately contain sibling source tokens; preserve those tests.

Use the existing checked merge/derive/contribution path on this frontier:
multiple IDs produce one ordered merge then one derived replacement; one ID
derives directly; none creates no graph entry or folded contribution. Stamp
only the actual returned root. Preserve every nested CV and non-CV field,
including raw spellings, signed zero, optionality and effectful expressions.
Existing before/after, parent_cv/new_cv and clock/pass/sweep contracts remain.

## Work and temporary storage before outer mutation

Use an iterative borrowed walk with one resumable iterator frame per active
list, yielding one child at a time. No recursive AST visitor, production JSON
roundtrip, eager sibling queue, cloned AST or cloned frame stack may discover
parents. The existing recursive folding/cloning is outside this collector's
bound; do not claim the entire pass or caller-owned input AST is bounded here.

A permitted two-scan design counts optional IDs first, retaining only reusable
walk frames, then allocates the deduplication buffers and replays the identical
immutable traversal. It must not build an unmetered intermediate ID vector or
hide a frame-stack clone behind Iterator::clone. Deterministic visitation, not
hash table placement, controls parent order. A single-scan alternative needs
the same simultaneous-growth accounting and independent review.

Use unchanged GraphLimits defaults as per-collector work/requested-storage
caps. Charge before every frame push/pop, source slot/iterator advance (also
the terminal advance), optional-child inspection, visited native node, ID byte
scan/hash/equality and table initialization/probe. Charge both scans together;
zero/absent/duplicate CVs still cost traversal work. Use checked arithmetic
for counts, capacities, byte totals and work subtraction. Hash collisions must
have a finite metered probe path. No fallback linear contains search may
escape the work budget.

Account together for simultaneously live frame storage, ordered ID references
and hash buckets. Before each fallible allocation/growth, check the combined
requested byte peak, including old and replacement buffers during migration;
charge initialized/copied slots before touching them. Reuse first-scan frame
capacity on replay or charge any later growth against all still-live buffers.
Omitting a temporary buffer from the peak because it belongs to another helper
is not acceptable. Empty-frontier work is still charged and unnecessary ID/hash
buffers are not allocated. State this as requested temporary-storage accounting,
not a theorem about the system allocator or total process memory.

Any collection failure latches cv_error before the outer selection's merge,
derive or folded contribution. No partially collected frontier is accepted.
Earlier bottom-up child graph activity is not rolled back by this contract.
Existing checked graph failures still reject the candidate and prevent JS,
map, manifest and trace publication. Preserve all production-limit controls.

## Tests first and acceptance

Commit compiled regression tests before implementing the collector. Native
tests must inspect the actual returned typed Program root, the actual retained
rewritten descendants and their exact graph edges. A folded journal entry or
token-presence search alone cannot establish this property.

- Root Some/None opaque calls with nonempty callee CVs and newly derived
  arguments; both key routes; exact ordered immediate merge parents including
  rewritten descendants, no unselected semantic roots, and actual root-to-child
  parent_ids reachability. Check child rewrite ownership and unchanged payload.
- An independent AST field/variant census, with nonempty distinct identities
  through wrappers, properties, templates, parameters/defaults, function/class
  bodies, statements and declarations. Empty fixtures alone do not prove
  nested traversal. Exercise duplicate, absent, single and no-ID cases.
- Deep and wide native trees using iterator frames; array holes and empty
  lists; no stack recursion or wide copied queue in the collector. Dispose
  synthetic deep inputs iteratively so fixture cleanup does not obscure it.
- Exact/one-less cumulative work and simultaneous requested-storage budgets,
  including frame growth while buffers coexist, ID-byte charges, forced hash
  collisions, overflow and injected allocation refusal. Check refusal precedes
  outer graph events. Private test injection must not alter production caps.
- Update CLOC36's historical immediate-only oracle explicitly where CLOC37
  intentionally adds parents. Keep its selection guards, root ownership,
  non-CV/nested-CV conservation and numeric-string identity assertions intact.
- CLI traced/plain emitted-byte equality, exact selected/child event and
  pass/sweep order, and all-output sentinel preservation for refused candidates.
  Run affected parser/folder/CLI/compiler/downstream suites and strict lint;
  obtain independent exact-head source and terminal security acceptance.

Contract acceptance is not implementation or runtime acceptance. Keep actual
execution, later documentation and live published PR heads distinct. Retain
the sole Closure/shared-crate PR; resolve actual current-main conflicts and
required CI before delivery under the existing review-clean merge rules.
General transformation records, conditional roots, deletion/motion/renaming/
inlining, output-byte/source joins and evidence-backed Closure parity remain
OPEN. Miri remains DEFERRED/PENDING; preserve its frozen checkout/run/inputs/
target/receipts untouched and obtain genuine final-source acceptance after
implementation delivery. Full objective OPEN; heartbeat ACTIVE.

## 2026-10-17: selected-subtree frontier locally validated

Actual full/runtime execution head `3be4a9c9e58c37a6e5fc6681eaa003063bbc68fe` extends CLOC36 root ownership to the
complete selected-subtree identity frontier. Member/array/actual-key operands
precede native field-declaration preorder; nonempty composite roots do not hide
newer rewritten descendants. First-seen deduplication preserves exact order.
Numeric-string keys retain their actual original folded identity. Only the
returned root is stamped; nested CVs, raw payload and semantic guards remain.

The borrowed iterative collector has86 reachable native types and11 resumable
list-frame kinds,72 full struct decisions and14 exhaustive enum decisions.
Its count/replay scans share one work budget; frame storage, ordered references,
hash buckets and old/new migration peaks share one requested-storage cap.
Missing IDs, holes, terminal advances, ID bytes, hashing and collided equality
are charged; fallible allocations and checked arithmetic precede mutation.
This bounds collector work/requested storage, not the recursive folder/clone,
system allocator or caller-owned AST. Default production limits are unchanged.

Contract3684 received independent PASS before code. Tests-first23cb compiled
0P2F expected semantic red; the first failure was the first numeric/root-None
call cell and the second BinaryExpression census cell. No all-case positive
claim follows from red. Tests-onlyc34/501 pinned independent leaf33/absent11
work formulas and the new descendant oracle before implementation817.
Later tests-only897/3be strengthen all-field, deep/wide and CLI evidence.

Actual full suites:282 parser +470 folder +1166 CLI +1006 native =2924P/0F,
two unchanged historical ignores,195 executables and three empty doctests;
both all-target strict checks complete0. Controls include161 populated native
field/variant cases across86 types;70 actual selected expression cases;
selected call and function/default/body rewrite ancestry; independent161-work
collision boundary, exact/one-less work/storage, old/new frame peak, combined
overflow, injected allocation failures,4096 depth/100000 width on144KiB stack
with iterative fixture disposal. SIMPLE/ADVANCED numeric/string CLI cells have
exact ordered semantic parents, child-before-outer and own merge/derive/event
clocks and traced/plain emitted-byte equality. Existing24 all-output sentinel
graph-refusal cells and physical source-interval controls remain passing.
Core receipt `99f99b16e35b9fb34ea65f870a1a3e462272d515da9c5bdd9ee6126f27cb5926`; source `d093e6b7be18c998f78d4bc13ad1753bf5d2ce00a1f859ee50c1f13eefd919ca`;
combined terminal `5d6ccdd1fa9e63c248ffb2cd5a8eb3dfba2259df823e588a4da113b0d20405b1`. Complete source/lock/log/manifest/artifact
and physical executable joins are bound; focused filtered harnesses add no
unique suite coverage and do not inflate these totals.

Separate same-head runtime `548691bd0c819b4307118841715badd4a1016d56e98be26c77c5e8e811ef31d5`:59 declared sources/354 ordered
six-backend observations/54 VM-JIT Float-bit captures; integration4P/observer1P/
refusal1P/strict0. LANG526 filtered, CLR absent/uncounted and native/LLVM
typed-main ABI OPEN; no full LANG, raw-stdout-byte or general precision theorem.

Audit 2026-10-17T21:55:12.594889+00:00: main `9f67440bb9a3271455e19f27a6e5743bbf6e051c`; sole Closure/shared
PR#17288 published `6132a518b3990619eb904397ee0b21a69ce8ac20`, CONFLICTING/
0 checks/autoOFF. No push/merge/hosted acceptance.
Next literal conditional still discards its checked fork; equal-branch and
other folds, typed role-rich records, rename/inlining/motion/deletion, complete
pass/output-byte/source joins, current-main reconciliation/delivery and full
Closure parity remain OPEN. Base ESTree/own types/no new crates.io preserved.
Miri DEFERRED/PENDING, never passed; frozen run/source/inputs/target/receipts
untouched and unpolled. Genuine final-source Miri follows implementation
delivery. Full objective OPEN; heartbeat ACTIVE. A later documentation carrier
does not claim a new Rust execution.
