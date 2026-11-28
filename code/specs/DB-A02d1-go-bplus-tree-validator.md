# DB-A02d1 — Verify Go B+ tree routes and the complete leaf chain

Make the public sequential validity check agree with the independent
B+ tree audit. Preserve existing lookups, writes, deletes and scan
results for reachable trees.
[Candidate audit](DB-A02d-go-bplus-tree-admission.md)

Keep `(*BPlusTree[K,V]).IsValid() bool` as the public entry point.
An empty tree has a nil root, nil first leaf and zero records.
An occupied tree has a nonempty root, known nonnil node types and
unique node identities. Reject missing children, aliases and cycles.
Check occupied roots and nonroot node counts using the existing
degree limits without integer overflow. Pair every leaf key with
one value and every internal key list with one additional child.

Keys and separators are strictly ordered under the supplied
comparator. Each descendant key is at or above its inherited lower
bound and strictly below its upper bound. A historical separator
can remain a valid fence after the smallest key in its right
subtree changes. Preserve valid historical fences in both comparator
directions.

Every leaf has the same depth. Its records contribute to the
reported size. The first leaf names the actual leftmost reachable
leaf. Every next link names the next reachable leaf in tree order.
The last reachable leaf has no next link. Check exact identities
and terminators even when a foreign leaf carries plausible keys.
Validation leaves the tree and linked list intact.

Compare with the retained independent walker after finite and
seeded operations, splits, merges, both rotations and root
contraction. Include single-fault controls for a misplaced separator,
lost value, short chain, foreign leaf, cyclic suffix, wrong first
leaf, typed nil and unknown nodes, repeated children and unequal
leaf depth. Test valid historical fences, noncomparable keys and the
largest supported degree. Run both build lists, race, vet, guided
inputs and clean controls for skipping an inherited bound or the
actual leaf link check.

## Local verification

The implementation is in `7790d7a1f49`. An independent caller
checked 2,940 isolated refusals across every leaf position in
eight reachable shapes. It included an otherwise ordered foreign
tail, cloned leaves with matching contents, short or cyclic links,
typed nil, unknown slice-backed nodes and noncomparable key types.
It accepted valid large-degree trees. These checks passed on
Linux/amd64 with race and vet and in emulated Linux/386.

Both command lists passed on Linux at 96.7% statement coverage
with race, vet and module tidy. The package passed 100,000 guided
inputs and both mutation controls. The retained audit passed all
327,680 bounded and 32,768 seeded transitions against the updated
validator. All 6,022 build graph rows and 6,021 emitted recipes
matched exactly.
