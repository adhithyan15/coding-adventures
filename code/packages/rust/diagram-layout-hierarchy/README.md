# diagram-layout-hierarchy

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


## Global curves and checked ranks

Global flowchart curve is applied only when an edge has no explicit curve. Rank propagation, optimizer increments/constraints and rank count use checked integer additions and return RankArithmeticOverflow through try_layout_swimlane before geometric arithmetic; the legacy wrapper explicitly refuses that error. Normal output, reference bits, work counters and parser limits remain unchanged.

Affected Windows suites/strict lint are source-attributed to e2fd7e9c;
producer native macOS rendering, general floating work and whole-PR/hosted
gates remain separate. Final-source Miri is **DEFERRED/PENDING**.
