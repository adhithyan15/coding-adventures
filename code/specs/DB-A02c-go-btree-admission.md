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
