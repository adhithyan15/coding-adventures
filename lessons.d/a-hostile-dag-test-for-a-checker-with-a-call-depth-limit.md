---
category: Rust
---

# A hostile-DAG test for a checker with a call-depth limit must assert rejection at depth, and accept only a smaller doubling

VER32 section 7 asks for a corpus file shaped `eapp i (i-1) (i-1)` repeated: 1,000 records
denoting a tree of 2^1000 nodes. The first PIF-002 test built a *well-typed* version
(`P p p` with `P : Prop -> Prop -> Prop`, 499 doublings) and asserted the file was accepted
in linear time. It was refused instead, with `ResourceLimit("recursion depth")`: the
kernel's `infer` recurses once per nesting level and its call-depth limit is 200, so a
well-typed doubling DAG can be accepted only up to about 100 doublings, whatever the
reader's own depth limit (1,000) allows.

Fix: two tests. A 90-doubling file (2^90 paths) must be *accepted* quickly, which proves the
checker memoises per node and the reader builds the DAG without recursion; a 499-doubling
file must be *rejected* quickly with exactly `ResourceLimit("recursion depth")`, which
proves the depth limit fires before any path enumeration. The literal ill-typed spec shape
must also finish fast (with a kernel error). All run on a 256 MiB stack so that "returns an
error" cannot be confused with "overflowed a big stack".

Rule: when a spec says "must finish within the kernel's limits", derive what the limits
actually permit (here, depth 200 not 1,000) before choosing the accept/reject boundary,
and assert the *specific* limit error so a different failure cannot satisfy the test.
