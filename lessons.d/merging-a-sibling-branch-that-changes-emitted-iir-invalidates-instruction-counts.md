---
category: Compiler / VM / language pipeline
---

# Merging a sibling branch that changes emitted IIR invalidates instruction counts pinned in the other branch's tests

LANG84 4c stacks on 4b (local) and merges 4a (in review). After 4b was
written, 4a changed the Perl `main` epilogue from `ret_void` (one
instruction) to `const 0; ret` (two), so that CoreCLR and BEAM read a real
exit status. 4b's child-process hostile probes pin exact instruction counts
(`Expect::Compiles(2 * short + 1 + 2)`). The merge had no textual conflict in
those lines, and the parent test only reported "child failed". The child's
own panic (`flat chain: expected Compiles(4003), got Ok(4004 instructions)`)
was visible only after re-running the ignored child test by hand with its
environment variable set.

The fix was to name the epilogue (`const EPILOGUE: usize = 2`) and derive
every count from it.

What to do differently: after merging a sibling branch that changes emitted
code shape, run the whole test suite of every crate that pins instruction
counts or IIR dumps, child-process tests included. When a parent test wraps
a child, run the child directly (with its env var) to see the real assertion.
