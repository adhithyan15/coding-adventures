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
