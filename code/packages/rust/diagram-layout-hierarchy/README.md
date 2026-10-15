# diagram-layout-hierarchy

`try_layout_railroad` validates all seven numeric spacing/style values before
font clamp or derived arithmetic, including empty diagrams. Invalid font size,
stroke width, marker radius or arc radius returns `UnsupportedStyle`; spacing
errors stay unchanged. Font measurement retains the producer's 14px default
(44.1px minimum leaf content width and 16.1px height before padding).

Radius-driven Optional and both Repetition variants reserve twice the arc
radius above and below child content. Matching measurement and placement keep
their main ports on the containing box's center through nested sequences and
choices. Independent wrapper geometry and complete path goldens cover zero,
tiny, default and nondefault radii. Checked derived coordinates remain finite
and rectangle sizes nonnegative; no global derived-layout cap is introduced.
The inherited 4px rule-marker/root-port mismatch remains open.

`try_layout_railroad` checks each finite0..65536 spacing setting before
arithmetic and returns a typed field error or derived-geometry refusal.
`layout_railroad` retains its infallible signature as a compatibility wrapper
that panics on unsupported IR. Sequence extents sum item widths and exactly
n-1 gaps; ordered flat/nested connectors and two-rule vertical gaps have
independent geometry goldens. Arbitrary recursive IR resource bounds remain open.

Deterministic backend-neutral layout for hierarchical diagrams. The initial
implementation partitions Mermaid treemap values into stable nested rectangles.

Swimlane layout indexes minimum-link constraints once using borrowed canonical
endpoint pairs. Duplicate and reversed edges keep their original drawing order,
labels and markers; spacing uses the pair's maximum length and defaults to one
gap unit. Constraint work is O((E + P) log(Q + 2)) comparisons for E edges,
P adjacent lane pairs and Q distinct endpoint pairs, with O(Q) index storage.
Identifier comparison costs, existing node searches and output-edge lowering
are separate. Geometric gap accumulation avoids integer overflow for caller-owned
IR; arbitrary downstream image allocations are not certified by this guarantee.

The duplicate-gap controls retain alternating Linear/Basis curve metadata and
compare each post-layout value alongside all prior edge metadata and work counts.

Cross-lane ranking reuses that same constraint index. It does not rebuild it or
scan all edges for each rank constraint. With 512 duplicate/reversed edges, the
enabled rank control observes exactly 512 index visits and 14 gap queries in all
four directions, while checking the distinct rank-grid spacing and every edge.
Automatic lane ordering and crossing optimization have separate work costs;
the constraint-index bound does not certify either heuristic.

For canonical unique-lane automatic ordering, each adjacent trial uses lane
balances and one borrowed pair-weight lookup. The original nine starts, UTF16
seeds, strict cost improvement and source-distance ties are preserved. Greedy
evaluation visits weighted pairs to initialize cost, balances and the pair index;
all nine evaluations require at most27W such visits. Initialization/index work
is O(9 (L + W log(W + 2))); trials use O(9 L² log(W + 2)) lookups, with
O(L + W) auxiliary storage. The unchanged restart seed/signature construction,
identifier comparison and outer source/edge construction remain separate costs. Malformed duplicate-lane direct IR retains
the original full-cost fallback and is outside that optimized work guarantee;
authored duplicate subgraph IDs remain refused by the parser.


## Bounded spacing and transport reconciliation

try_layout_swimlane returns own typed UnsupportedSpacing, InvalidDerivedGeometry or RankArithmeticOverflow errors. The legacy layout_swimlane wrapper explicitly panics on unsupported input. Supported large padding expands minimum canvas extent so lane dimensions stay nonnegative. Coincident node centers use exact right-side/source and top-side/target anchors; other endpoint formulas remain unchanged. This checked boundary covers spacing/derived geometry on previously supported structural/numeric IR; checked rank arithmetic now refuses overflow, while arbitrary cyclic/malformed caller IR is not certified.

Validation is source-attributed: Windows applicable packages and standalone
checks are recorded separately from Unix-only/hosted and whole-PR gates.
Final-source Miri is **DEFERRED/PENDING**, never passed.

The imported node-padding field is admitted by the same finite0..=65536
checked spacing boundary. Large accepted node heights expand horizontal lanes
with the existing cross-axis inset, preserving default dimensions and keeping
nodes inside lanes/canvas. Direct invalid-value red controls and complete
four-direction golden layouts precede these repairs. Current efd2e023 affected
cone passes848 tests/32targets plus strict all-target lint. General malformed
caller IR and whole-PR/hosted acceptance remain separate.

Imported titleTopMargin also uses the finite0..=65536 typed admission boundary,
including inputs without a title. Its direct red control fails all40 invalid
cells before the guard; the repair returns the exact UnsupportedSpacing field.
Eighty complete-layout controls preserve input/config bits and all fields while
checking title-only vertical translation across four directions and real groups.
The resolved main conflict retains the checked wrapper and constraint indexes.
Final affected suites/strict and exact-head review remain required; Miri is
**DEFERRED/PENDING**.


## Global curves and checked ranks

Global flowchart curve is applied only when an edge has no explicit curve. Rank propagation, optimizer increments/constraints and rank count use checked integer additions and return RankArithmeticOverflow through try_layout_swimlane before geometric arithmetic; the legacy wrapper explicitly refuses that error. Normal output, reference bits, work counters and parser limits remain unchanged.

Affected Windows suites/strict lint are source-attributed to e2fd7e9c;
producer native macOS rendering, general floating work and whole-PR/hosted
gates remain separate. Final-source Miri is **DEFERRED/PENDING**.

Checked subgraph title margins retain finite inclusive0..65536 top/bottom
admission, with space for every nested title band in all directions. Positive
margins account for admitted node/outer padding and fixed-height shapes;
complete default/zero layouts remain exact. Spec-first80 invalid controls and
16128 geometry cells pass. Final3d full affected14-package validation passes
855/0/0 over32targets with strict alltargets EXIT0. Miri DEFERRED/PENDING.
