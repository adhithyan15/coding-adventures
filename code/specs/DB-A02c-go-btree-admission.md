# DB-A02c — Audit the Go in-memory B-tree for the reference engine

Audit `code/packages/go/b-tree` before the Go engine relies on it.
The candidate package tree is `77793a2b9284cd09edb430f28bc5d3f8ed3a82f2`.
Keep this admission scoped to sequential in-memory operations.
[Tree backlog](DB00-database-backlog.md#tree-fundamentals)

Trace construction, insertion and replacement, lookup, deletion,
minimum and maximum, ordered traversal and inclusive range queries.
Compare the returned keys, values and absence outcomes with a simple
ordered map. Pin degree two and higher degrees, statically valid
comparator orderings, duplicate updates, descending comparators,
empty and inverted ranges, missing deletes, root contraction, splits,
borrowing from either side and merges. Describe whether
changing a returned entry can change the stored key or value.

Add a package-local independent walker for tests. It must propagate
strict lower and upper separator bounds into every subtree, check
key/value lengths and child counts, enforce root and nonroot occupancy,
detect shared or duplicate node identities, and verify equal leaf
depth and total distinct key count. Compare keys and values through
the walker as well as through the public API. Use corrupted synthetic
trees as negative controls for the walker. In particular, the current
`IsValid` checks local key order, occupancy, leaf depth and count, but
does not propagate separator bounds or compare key and value lengths.

Use bounded exhaustive histories and seeded mixed histories. Record
operation counts, degree and key bounds, command lines, coverage
and any reproduced failures. Run the native build, race and vet.
Compare the public and independent validity results on reachable
trees. If a public operation or a validity claim fails, retain the
smallest witness and file the required change before reuse. Record
admitted claims by exact package revision.

The B+ leaf chain, on-disk nodes, crash-safe publication, concurrent
commands and page latching have their own contracts and audits.

## Preliminary reproduction

The exact candidate's native suite passed at 95.3% statement coverage.
Race and vet passed. A scratch package-local probe inserted four keys
at degree two, then moved a left-child key above the root separator.
`IsValid` returned true, traversal lost global order and lookup missed
the displaced key. A second synthetic shape removed one stored value
while retaining its key, and `IsValid` returned true again. These
controls establish the baseline checker boundaries for the independent
walker. Full sequential admission remains queued.

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

## Candidate node audit

The independent package-local audit is in `0e470aefba9`. It checks the
private node graph and public operations after each of 5,184 bounded
transitions across 1,296 degree-two histories, then 16,384 seeded
transitions across degrees 2, 3, 5 and 11 in both comparator
directions. Targeted sequences exercise root contraction, borrowing
from either side and returned entry ownership. The test walker refuses
out-of-range descendants, mismatched key/value lengths, repeated
children and cycles.

Both command lists passed at 96.0% statement coverage on Linux.
Race, vet and module tidy passed. The runtime files still match the
original candidate revision. A separate exact witness reproduced
both public validator gaps. [DB-A02c1](DB-A02c1-go-btree-validator.md)
tracks the repair before the reference engine relies on that
validation result.
