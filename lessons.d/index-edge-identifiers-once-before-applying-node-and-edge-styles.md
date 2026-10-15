---
category: Rust
---

# Index edge identifiers once before applying node and edge styles

Main e32e5788 added an all-edge filter for every class/style target. Existing
node-only inputs now scanned every unrelated edge even when all edge IDs were
absent. Independent review found the regression; actual public-parser red at
f2cc672d7d executed 1,024 predicates for32 edges/32 node targets in each of the
class and style cases. Two work tests failed and one exact semantic control
passed. Preserve the failure and commit the work/metadata contract first.

Build one ordered ID-to-all-authored-edge-indices lookup, skipping absent IDs
and copying each distinct key once. Node/edge-ID collisions still update both;
duplicate edge IDs retain all matches in source order. Count index traversal,
target lookups and unavoidable matching updates separately. Do not call repeated
real class/style application, existing node lookup or the whole parser linear.

The same upstream IR change added classes/style fields and broke our preserved
duplicate-constraint fixture. Actual isolated dd9b81c8 failed compile with zero
executed tests; that is neither runtime red nor native success. Supply distinct
authored classes/styles and assert exact layout preservation alongside IDs,
labels, kinds, markers, lengths and unchanged operation-count bounds.
