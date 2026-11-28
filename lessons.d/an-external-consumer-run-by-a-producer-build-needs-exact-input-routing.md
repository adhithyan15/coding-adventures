---
category: BUILD files & dependency management
---

# An external consumer run by a producer BUILD needs exact input routing, not its own unknown package root

The WASM microbenchmark package already builds the `.wasm` artifact and runs
the independent Node host against it from both supported BUILD fronts. Giving
that host directory a second standalone BUILD caused discovery to invent
`unknown/wasm-microbenchmark-consumer`; the source has no supported language
identity, and the hasher correctly rejects arbitrary unknown packages.

When an external test host is already run by a supported package BUILD,
register its exact source file as a repository input of that package. Keep the
test assertions and the strict unknown-package rule; do not add a generic
unknown-package exception or a duplicate build root. Verify the exact input
selects the owner, changes its hash, and appears in the Kiln input export.
