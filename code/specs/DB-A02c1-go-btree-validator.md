# DB-A02c1 — Check the full Go B-tree structure in IsValid

Bring the public sequential validity check into agreement with the
independent structural audit. Preserve the behavior of insertion,
replacement, search, deletion and scans.
[Candidate audit](DB-A02c-go-btree-admission.md)

Keep `(*BTree[K,V]).IsValid() bool` as the public entry point.
A nil root is valid exactly when the tree reports zero entries.
An occupied root contains at least one key. Every other node uses
the existing degree-dependent occupancy limits. All node key and
value counts must agree. Leaves have no children. Internal nodes
have one more child than keys.

A node's keys follow the configured comparator in strict order.
Each descendant key stays strictly inside the ancestor separators
that route to it. Missing, repeated or cyclic child identities
fail validation. Every leaf is at the same depth, and the total
number of stored pairs equals the reported size. The check has no
write effects on the tree.

Run the independent walker and public checker against the same
reachable histories. Compare both with controlled malformed nodes
that violate one condition at a time. Include the established
degree-two counterexamples, an ancestor violation several levels
below the root, nil and shared children, cycles, mismatched
key/value lengths, a leaf with a child, and divergent leaf depth.
Verify ascending and descending comparators and root contraction.
Use both build lists, coverage, race, vet, guided inputs and clean
controls for dropping the inherited bound or the key/value pairing
check. Retain exact evidence before admitting the validator for the
reference engine.
