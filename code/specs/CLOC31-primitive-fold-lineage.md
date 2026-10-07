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
