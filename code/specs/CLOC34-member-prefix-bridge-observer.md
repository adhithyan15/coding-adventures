# CLOC34: borrowed member prefix occurrences at typed construction

## Bounded purpose and existing evidence

CLOC27 already reuses stamped physical token identities on typed leaves; CLOC33
now certifies captured CLI array occurrences with shared SourceRange and a
closed bounded ledger. Flat member chains create several typed members from
one grammar node, so a node-address registry cannot identify every prefix.
Provide the missing borrowed library construction evidence first. This slice
does not wire CLI member identities/spans, extend CLOC33's closed node schema,
certify arbitrary observer strings, or complete transformation/output parity.

## API and compatibility contract

- Preserve grammar_to_program and CLOC32 grammar_to_program_with_array_observer,
  their diagnostics, existing leaf/array CVs and all non-CV AST fields. The old
  no-observer and array-only routes must do no extra source/CST traversal,
  registry allocation, endpoint validation or member callback.
- Add CompositeOccurrenceObserver: ArrayOccurrenceObserver, adding
  member_constructed(owner: &GrammarASTNode, end_child: usize, computed: bool,
  optional: bool) -> Result<Option<String>, String>, and a separate
  grammar_to_program_with_composite_observer entry. One borrowed composite sink
  receives arrays and members, so an eventual trusted CLI adapter can own its
  unique mutable log/ledger without two competing log borrows.
- Each successfully constructed MemberExpression or OptionalMemberExpression
  calls the new member method exactly once, after its object/property children
  are converted and before any parent callback. Only that member's cv may be
  assigned from its result. Calls/new/ChainExpression/property Identifier CVs
  are not annotated by this slice. Existing array callbacks retain postorder.
- The borrowed owner is the exact live grammar node used by the factory.
  end_child is the inclusive index of the actual suffix endpoint in its raw
  children; owner.children[..=end_child] is the original prefix. Dot/optional
  dot ends at its direct property token; computed access ends at its direct
  RBRACKET, not the key node. Thus one owner for a.b.c produces distinct a.b
  and a.b.c prefixes, and prefixes include grouping/call/new syntax and prior
  suffixes. No source spelling/value/coordinate search or cloned subtree is
  authority. Pointer equality is permitted only in tests, never serialized.
- Cover all eight existing member factories in member_expression,
  call_expression and optional_chain_expression, including optional computed
  and optional dot arms. Preserve existing base/suffix conversion bodies and
  cursor advancement. Hook insertion may assign cv at construction or directly
  afterward once the existing loop has consumed the closing bracket. No new
  prefix traversal is needed: indexed endpoint checks are constant work.
- On the composite route only, require the recorded endpoint to exist as a
  direct Token and, for computed access, spell the existing closing bracket.
  Missing/wrong endpoints reject through the run-local first-error latch
  before invoking the sink. This is a structural hook check, not certification
  of physical identity/source ranges or arbitrary caller-built CST validity.
  A trusted adapter must separately meter any prefix/terminal traversal and
  validate physical identity, source replay and a closed receipt domain.
- Reuse CLOC32's observed error wrapper and borrowed scoped 128 MiB worker.
  Its historical array-labelled Display strings remain compatible. Preserve
  first sink error across both methods, stop all subsequent callbacks, and
  check the latch at the public boundary even if an internal sentinel is caught.
  Drop rejected partial candidates on the worker before returning to a small
  caller; creation/join/panic failure refuses a Program. Successful AST/caller
  CST ownership/depth and arbitrary Rust callback work remain caller duties.
- No TLS/global state, generic AST/Token fields, new source/span representation,
  source clone/index, persistent registry or new crates.io dependencies.

## Tests committed before implementation

Commit tests first. An explicit delegating scaffold may expose the new API and
provide a compiled behavioral red; absence-of-API compilation is not a red.
Use real parser/lexer CVs and a test sink allocating fresh compact Merge nodes.
Assert exact typed member/optional member ownership, original ordered physical
token parents from the borrowed prefix, distinct same-owner suffix endpoints,
no later-suffix/sibling parents, and child-before-parent array/member order.
Cover dot/computed/mixed chains, grouping, call/new/super/optional prefixes,
arrays in computed keys and repeated identical source occurrences. No-op sinks
must preserve complete Program equality; annotated results preserve native
CV-erased equality and existing leaf CVs. Old APIs remain unchanged after runs.

Exercise member error under supported declaration/function/arrow/class/computed/
statement parents, array-to-member and member-to-array refusal, graph exhaustion,
ordinary Bridge errors, sink panic, nested independent observed runs, and direct
wrong/missing endpoint controls. A caught-sentinel unit test must retain the first
error and suppress both methods. A subprocess with a 128 KiB caller and a 4,096-
member prefix must refuse a late member callback without transferring deep
candidate destruction to that caller; unexpected scaffold success is leaked
before failing so its red is an ordinary assertion failure.

Run complete javascript-parser/constant-fold/closurec/native consumer suites,
affected all-target strict lint and six-backend downstream companions. Freeze
exact inputs/helpers/logs/heads and consume producers before independent terminal
review or edits. Independent source and terminal security review are required;
old failed receipts remain immutable. Preserve production-limit/safety controls.

## Delivery boundaries

Next bounded slice is a separately specified trusted CLI member-prefix adapter
with existing shared SourceRange and CLOC33 replay/ledger publication schedule.
Other synthetic/composite identities, full fold/rename/inline/motion/merge/delete
and pass/sweep lineage, output-byte/source joins and evidence-backed Closure
parity remain OPEN. Base ESTree canonical serialized boundary and own types stay.
One active implementation PR; current-main reconciliation/CI/actual merge remain
pending. Miri DEFERRED/PENDING: frozen checkout/run/inputs/target/receipts stay
untouched and unpolled. Genuine final-source Miri follows implementation delivery.

## 2026-10-16: library member prefix observation locally accepted

Contract5a4b passed independent review before tests15a/scaffold2212/unit8c99 and
implementation11ac. All eight actual factories now forward the borrowed owner,
inclusive suffix endpoint and computed/optional flags to one composite sink.
Four computed hooks follow existing closing-bracket cursor advancement; checks
are gated to the composite route before any extra indexed read. No prefix/source
walk or registry is added. Existing legacy/array-only bodies are preserved by
an exact whole-file bridge inverse; only member CVs change on the new route.

Compiled scaffold2212 red has eight outer failed tests; the nested child failed
one/filtered seven and is not additive. Focus11ac had7P1F because the new sink
expected max_events=0 to reject a bare merge. Explicit checked contribution
repairs that fixture without changing any three budget cases. Focusdbe then
passed8 member/8 old array/3 unit tests but strict rejected its tuple return
type. Exact tuple aliases repair lint; positive array-in-key, optional suffix
and new-computed prefix/order controls are stronger. Placeholder/EOF lesson
gate failures were corrected and remain failures in their original evidence.
Production after11ac is unchanged; failed receipts are never retro-accepted.

At exact execution head `24f28ca8c0997a9b2425847d6dcd5f98196ec9ea`, complete parser282P, folder451P/two historical
ignored, closurec1152P and nativecompiler1006P total2891 passed across194
executables plus three empty doctests. Both affected all-target strict checks
pass. Tests assert exact borrowed same-owner prefix endpoints, typed CV ownership,
ordered original token parents, sibling/later-suffix exclusion, optional flags,
array-key postorder, unchanged leaf CVs/native AST equality, all supported parent
paths, malformed-endpoint old-route controls, graph/refusal/panic/nested runs,
caught first-error latch and late4097th-member refusal on a128KiB caller.
Full core receipt `fa0279b4881f76444b76c9ac49b3358b7055f2fe6c225baadff954984d62190f`; source review `061ed305d73ca825ce53849486366af6ec5c10c42fc358df5eddd3b4a7c3af4a`;
combined terminal review `93112cd1ca2d02c400c306391d6b6b7edb54fcd1ce4586275c33bf4a9a4bb1bd`. All frozen inputs/locks, actual command
exits, complete logs and executable joins/hashes are independently bound.

Separate downstream run at the same head passes59 exact sources/354 ordered
six-backend tuples/54 VM-JIT Float-bit observations, integration4P, observer1P,
compile-refusal1P and strict. Receipt `ce05ef050409480cd0f4195b76616d9d0045f48aae534709eee9e66e8f007704`. Focused LANG selectors
each filter526 unrelated tests; CLR absent/uncounted; native/LLVM typed-main ABI
OPEN. This is not full LANG runtime or full Closure parity acceptance.

Next is a separately specified trusted CLI member-prefix adapter using existing
shared SourceRange and CLOC33 final replay/closed-ledger schedule. This library
slice alone assigns no CLI member identities and certifies no source spans or
output-byte mapping. Full transformation/pass/sweep history, other composites,
rename/inline/motion/merge/deletion and parity remain OPEN. Base ESTree, own types
and no new crates.io dependencies preserved. Audited mainb7f5f264befe2d3c7bdd029a4b019212e36a13c1
reconciliation beyonda2 (latest native29) remains pending. SolePR17288
publishes6132a518b3990619eb904397ee0b21a69ce8ac20, mergeabilityUNKNOWN/zero checks/autoOFF
at2026-10-17T00:14:20.245833+00:00; delivery pending, no new push/merge claim. Miri DEFERRED/PENDING; frozen run/checkout/
inputs/target/receipts untouched/unpolled. Genuine final-source Miri follows
implementation delivery. Full objective/heartbeat remain ACTIVE.
