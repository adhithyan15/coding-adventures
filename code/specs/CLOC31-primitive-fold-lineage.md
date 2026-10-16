# CLOC31 — Primitive fold lineage must reach its operands

## Problem and scope

CCR-065 (#15830) requires output-to-source transformation lineage. CLOC27
already propagates lexer identities onto leaf literals and runs the optimizer
against the enabled CV log. Its end-to-end tests, however, only locate source
tokens anywhere in the sidecar; disconnected tokens pass those assertions.
The member, binary and unary folders derive from the composite node's optional
identity, which the bridge often leaves absent. Their replacement then has no
identity. Fold contributions are also returned for the program summary, without
being recorded on the replacement itself.

This bounded slice repairs string-literal `.length`, primitive binary folds and
primitive unary folds. Other transformations, composite parse identities, exact
byte ranges, source maps, output mappings, typed motion/inlining/deletion edges
and graph queries remain required by CCR-065 and CCR-007. This slice does not
claim to complete either epic or change the canonical ESTree boundary decision.

## Contract

1. A successful primitive fold derives a fresh replacement identity from every
   available immediate operand identity, including identities produced by an
   earlier child fold. Include the original composite identity when present.
   Deduplicate identical parents without changing their order.
2. One parent uses `derive`; several parents use `merge` followed by `derive`.
   No parent means untraced output, with the same optimization behavior.
3. Record `constant-fold` / `folded` on the replacement identity, with the
   existing `before`, `after`, `parent_cv` and `new_cv` metadata. Retain returned
   contributions for compatibility with program-level summaries.
4. For string `.length`, preserve the folded receiver and property identities.
   Binary folds preserve both folded operands; primitive unary folds preserve
   the folded argument. Retaining a source token in the log is insufficient.
5. Tracing must not alter JavaScript bytes for these operations. Declined folds
   must not create fictitious `folded` records. Nested operations must retain
   child transformation records through parent links.
6. Branch equality must compare every AST field recursively except CV identity.
   `flag ? (1+1) : (1+1)` and equal composite array/call branches must compile
   identically with tracing on/off. Primitive replacements retain both branch
   histories and available test/composite identities. Numeric comparison must
   distinguish signed zero, reject NaN equality, and retain raw representation.
   Provide a borrowed native comparator in `javascript-ast`; do not clone,
   serialize, strip JSON metadata or use a hash as equality. Exhaustive node
   destructuring and variant coverage must make AST additions require review.

## Verification

Run actual SIMPLE and ADVANCED compilations with tracing. Locate the replacement
by its own `constant-fold` contribution, traverse `parent_ids`, reject dangling
links and cycles, and assert the exact source file and line/column ancestors.
An unrelated token must not appear in that ancestry. A nested arithmetic case
must retain both the intermediate fold and all three operand origins. Compare
traced and untraced output. Exercise a declined division-by-zero fold.

Run the affected constant-fold and closurec suites, all-target lint, differential
ladder and oracle manifest checks. Preserve captured upstream expectations;
this repair does not change the oracle or reduce known gaps. The equality
guard also corrects an existing signed-zero branch misfold; pin both orders
and nested composite cases rather than preserving that incorrect behavior.

## Completion sequence

Reconcile stale coordination state first. Complete this primitive-fold repair,
then audit every remaining fold family with graph assertions. Complete stable
composite node identities and shared source spans on the ESTree boundary;
version typed lineage events and record actual pass/sweep order; adopt events
for every mutating pass, including one-to-many inline, many-to-one merge and
deletion tombstones; join emitter output ranges and real source maps; expose
graph queries. Every compiler parity slice must preserve the resulting evidence.
Full completion also requires diagnostics, syntax/lowering, modules/chunks,
externs/reports and representative real-world differential builds under the
backlog's completion contract. A green curated corpus alone is insufficient.

## Resource boundary from independent review

CLI unary-chain probes compile at depths 10/20/40/60 and reject 100/400/1000
cleanly at the parser nesting guard. Direct library callers can construct ASTs
outside that guard; successive single-parent CV derivations encode ancestor IDs
and grow with depth. Compact graph identities and explicit graph/serialization
resource limits are required follow-up acceptance work in CCR-065, alongside
bounded traversal, rather than an implied guarantee of this primitive slice.

## CI compatibility repair

The macOS affected-package build on head `19a5592117` rejected the scheduler's
existing `CountingPass` unit-test helper because its atomic `fetch_update`
method is deprecated under the CI Rust toolchain and warnings are denied.
Replace that call with a compare-and-exchange loop using the same sequentially
consistent ordering. The change budget must atomically decrement while positive,
remain zero once exhausted, and retain existing fixed-point/cap test behavior.
Use APIs supported by the local and CI toolchains; do not suppress the warning
or change production scheduling. Run the pipeline's own tests and all-target
strict lint, since consumer lint does not compile dependency unit-test targets.

The next affected-package build on head `a8f6a304ef` passed the scheduler but
rejected the oracle-refresh example's aggregate output counter for the same
deprecated API, on both macOS and Linux. Replace that counter update with a
checked compare-and-exchange reservation retaining AcqRel success / Acquire
failure ordering. Concurrent reservations must never exceed the existing
32 MiB cap, failed reservations must leave the counter untouched, and integer
overflow must fail closed. Keep the oracle command, trust pins, captured bytes
and classification rules unchanged. Verify exact-cap and overflow rejection,
concurrent saturation, the example's own tests and all-target strict lint.

## Bounded follow-up: array-literal length operand frontier (2026-10-16)

The current-source audit confirms CLOC27 physical leaf identities and CLOC31
primitive folds already work. Array-literal `.length` and `["length"]` still
derive only from the often absent member identity. Consequently
`report([1+2,4].length)` retains the child fold to 3 but loses the final count's
own identity and history. Repair these two existing folds without changing
their side-effect/spread guards, JavaScript bytes, count or hole semantics.

For each successful length fold, collect available identities in this order:
member, array, property, then the already folded immediate primitive elements
in element order. Omit holes and unavailable identities. Deduplicate by first
occurrence; use bounded ordered membership work (O(n log n) comparisons), not
a quadratic scan when the frontier is wide. Reuse the checked merge/derive/
contribution path and its first-error latch. Keep collection out of the hot
recursive frame. One parent derives directly; multiple parents merge then
derive; zero parents retain the existing untraced behavior. Do not mint fake
source origins or recursively harvest descendants of opaque composite cells.

This is explicitly partial operand lineage. Empty/hole-only arrays, untraced
composites and nonprimitive cells still need shared composite occurrence
identities/source spans. Dot-property tokens are not currently bridged into
identifiers. This repair does not establish complete array/source coverage,
output-byte mappings, deletion history or full CCR-065 acceptance.

Commit regressions before implementation and record an actual compiled red.
Run SIMPLE and ADVANCED traced/untraced CLI compilations for both spellings
of `[1+2,4]` length. Assert the exact emitted bytes, a result-owned count fold,
reachability of the earlier 1+2 fold, and the exact closed set of physical
operand origins (including the computed string key). Typed tests must pin
first-seen parent order/dedup, holes, missing identities, a wide frontier,
declined effect/spread cases and checked node/edge/event exhaustion with no
accepted candidate. Preserve existing deep-stack and production limit tests.
Run complete constant-fold and closurec suites, affected strict all-target
lint, differential ladder and oracle manifest checks; obtain independent
exact-head security review before publication. No new crates.io dependencies.
Miri remains DEFERRED/PENDING under the October 11 sequencing authorization;
genuine final-source Miri acceptance follows implementation delivery.
