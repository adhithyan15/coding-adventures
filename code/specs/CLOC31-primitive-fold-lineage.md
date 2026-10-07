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

## Verification

Run actual SIMPLE and ADVANCED compilations with tracing. Locate the replacement
by its own `constant-fold` contribution, traverse `parent_ids`, reject dangling
links and cycles, and assert the exact source file and line/column ancestors.
An unrelated token must not appear in that ancestry. A nested arithmetic case
must retain both the intermediate fold and all three operand origins. Compare
traced and untraced output. Exercise a declined division-by-zero fold.

Run the affected constant-fold and closurec suites, all-target lint, differential
ladder and oracle manifest checks. Preserve captured upstream expectations;
this provenance-only repair does not change the oracle or reduce known gaps.

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
