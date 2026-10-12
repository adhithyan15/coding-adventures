# CV04a — reconcile bounded local roots without stale assignment proofs

Status: bounded conflict repair for sole shared PR #17288; provenance epic
#15830/foundation #16970 remain open. Full local Miri **DEFERRED/PENDING**.

Main `e18c68942668e04b1993c8b3a25d3be2085d9568` (#17352) extends nine bounded
conditional result classifiers from selector-safe runtime-real expressions to
selector-safe unaliased local roots. Published reviewed candidate
`af3402db4f4b76afb30110171044041af19a5162` conflicts in adjacent compiler tests
and two README descriptions. Resolve by retaining both upstream local-root
behavior/tests/docs and the existing #17350 assignment guard: initial proof
AND a fresh proof after RHS lowering, since selector by-name actuals can promote
a root to shared storage. Do not discard either side's controls.

## Compatibility and validation

Preserve the existing refusal diagnostic for shared/captured/by-name roots and
all proof depth, built-in binding, finite-bound and output safety controls.
Extend exact compiler controls to representative single/double/triple families
in addition to the existing quadruple family, checking both root positions.
By-name selector aliases must not retain assignment proof. Value selectors
must preserve original roots, exactly one real formatter and no global root
loads/stores. Full result correctness must execute native controls rather than
being inferred from a formatter instruction. Use distinct -2.25/0.0 producers
and exact selected true/false bounded-family sums5/2; retain the existing
quadruple value-selector outputs2/3 and the new upstream native case.

Run the metadata-derived compiler/shared cone and strict all-target lint with
unknown-lint denial. Execute all11 unchanged LANG production BUILD selections
and every prior/new exact native ALGOL regression, with actual counts/source
heads and any failure retained. Original diagram752/32, lexical/compiler and
controlledQt results require complete unchanged-source proofs; do not call
retained execution fresh. Reconstruct documents, validate lessons and whitespace.

Independent exact-final security review precedes non-force publication of the
repaired same PR. Fresh exact-head required CI/whole3OS Closurec completion logs,
clean reviews/conflict-free state precede auto-merge, then actual fetched-main
merge and full payload identity before cleanup/CV04b. No new crates.io dependency,
limit/assertion weakening, numeric/UB claim without applicable evidence, or
frozen Miri run/input/receipt/target mutation. Full parity, ingestion, output maps
and complete transformation lineage remain open.
