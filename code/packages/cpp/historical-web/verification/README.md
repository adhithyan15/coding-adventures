# HWEB09 C++ arena SAT slice

`sat_cpp_arena_append.py` models one `append_node` transition from the HWEB09
C++ implementation using the shared Go CDCL solver. It uses symbolic 32-bit
and 64-bit `size_t` values. The transition equations are:

- the appended vector index equals the old node count;
- the exclusive count after a successful append is old count plus one;
- each root/child/sibling link target is assigned from the appended index;
- child parents and previous siblings come from indexes in the pre-state.

A successful-append precondition excludes increment wrap. It does not add an
extra `post_count != no_index` restriction: the count is exclusive, so the
last valid index is `post_count - 1`. Positive SAT witnesses cover root,
first-child, and previous-sibling append. The root query establishes only that
the root link target is in range. The child and previous-sibling cases also
check that their source index, assumed to come from the valid pre-state, is
below the target. Requiring a correct link target to be at or beyond post-count
is UNSAT; a mutant that links to post-count is SAT. Removing the source-bound
precondition permits the parent to equal the new child and creates a SAT
self-child cycle control. Removing no-wrap admits the maximal old count and a
wrapped post-count.

For the acyclicity explanation, forward traversal edges are
`first_child_index` and `next_sibling_index`; `parent_index` is a backward
reference. A new node begins with no first child or next sibling, and append
adds forward links from pre-existing nodes to the new node. Given an already
acyclic pre-state, adding a leaf cannot create a forward traversal cycle. The
valid stack/previous-child indexes are assumptions from the existing parser
state; `append_node` does not independently validate them.

Run from the repository root with the pinned tools:

```sh
HWEB_GO=/path/to/pinned/go/bin/go \
  /path/to/pinned/python/bin/python3.13 \
  code/packages/cpp/historical-web/verification/sat_cpp_arena_append.py
```

This is a one-step index/link model. It does not model `std::vector` allocation
or `max_size()`, prove every parser path preserves its preconditions, or
establish whole-parser correctness. The optional checker adds no runtime
library, build, or platform dependency.
