# DB-A02d — Audit the Go in-memory B+ tree for the reference engine

Use the [Go B-tree admission](DB-A02c-go-btree-admission.md) to
establish the common ordered-map test vocabulary. Audit
`code/packages/go/b-plus-tree` separately. The candidate package
tree is `c4a4cf97bfeade74024d7868dd8e971459067390`.

Replay public insert, replace, delete, lookup, minimum, maximum,
full scan and bounded inclusive scan against an independent
ordered map. Exercise low degree, larger degree, descending order,
duplicate updates, empty and inverted ranges, leaf and internal
splits, redistribution in both directions and root contraction.
Verify record ordering and key/value association in every scan.

An independent structural walker must check occupancy, equal leaf
depth, key/value pairing and exact child cardinality, then verify all
descendants against the documented upper-bound routing rule. Collect
the actual leaf identities reached from the root. Follow the next
pointers from the declared first leaf and compare each identity and
order with the traversal. Detect skipped or foreign leaves and cycles
without walking an unbounded chain.

Use finite and seeded histories with precise bounds and coverage.
Include controlled malformed separator, missing value and leaf-link
states to verify the independent walker. Record race and vet results
and disposition each required claim at the exact revision. A
page-backed B+ tree has additional durability and concurrency
contracts.

## Preliminary reproduction

The candidate's native suite passed at 95.7% statement coverage,
with race and vet. A scratch probe moved one root separator above
the actual right subtree. `IsValid` returned true and lookup missed
a key from that subtree. It also returned true for a detached next
link that truncated a full scan and for a leaf with unequal key and
value counts. The independent admission must check these properties.
