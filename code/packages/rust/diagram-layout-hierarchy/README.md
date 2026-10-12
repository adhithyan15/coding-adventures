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
