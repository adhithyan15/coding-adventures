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

## Independent public operations

The exact candidate matched an ordered-map oracle over 65,536
five-command histories and 32,768 seeded transitions. It covered
forward and reverse comparators, minimum degrees 2, 3, 5 and 11,
insert and replacement, delete and lookup, full traversal, and
inclusive and inverted range boundaries. Get, count and extrema
agreed with the oracle. Race and vet passed on the external caller.

The independent witness source SHA-256 is
`7b98eb945a52b199b81c8607f1ffdc0b99211fee228fe74d637fd6e00155dde7`.
Admission also requires an independent private-node traversal and
a disposition for the public validator's structural claims.

## Historical fences and degree limits

A separate exact witness inserted keys 1, 2, 3 and 4 at degree two.
The root's separator was 3 and its right leaf held keys 3 and 4.
Deleting key 3 left separator 3 with right leaf 4. The remaining
lookups were correct and the separator still routed every key
within the documented inclusive lower and exclusive upper bounds.
Check these bounds independently of the current minimum of the
right subtree.

The original validator also refused a valid one-entry tree at
`MaxInt/2 + 2` and `MaxInt` on the 64-bit host. The witness passed
race and vet with the reproduced outcomes. Preserve large-degree
acceptance when repairing the validator's arithmetic.

## Candidate node and chain audit

The independent package-local audit is in `23264f9baec`. It checked
65,536 five-command histories with 327,680 public transitions and
32,768 seeded transitions. It compared every resulting tree to an
ordered map, checked inherited routes, compared actual leaf pointers
to the traversed leaves and exercised redistribution and root
contraction. Valid historical fences retained their routing meaning.

Both command lists passed on Linux at 95.7% statement coverage with
race, vet and module tidy. The independent walker refused malformed
bounds, key/value lengths, leaf links, nil and unexpected node types,
and shared or cyclic nodes. The public checker accepted synthetic
examples with a wrong separator, missing value or truncated chain.
The sequential validator repair is verified in `7790d7a1f49`.
[DB-A02d1](DB-A02d1-go-bplus-tree-validator.md)
