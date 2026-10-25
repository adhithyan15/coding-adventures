---
category: BUILD files & dependency management
---

# Explicit transitive bootstrap paths in BUILD need declared package metadata edges

NN37's numerical tests and its dedicated workflow passed, but repository CI's
build-plan detector rejected the TypeScript BUILD reference to matrix-rust-napi.
The intermediate matrix directory has a package.json but no BUILD file, so the
discovered build graph does not carry its transitive npm dependency to the
framework. The validator therefore sees an undeclared sibling bootstrap.

Use matrix's own `npm ci --omit=dev --ignore-scripts` to materialize its
dependency-free addon shim through its locked manifest, then install the
framework. This needs no separate wrapper BUILD reference or native addon
build. A `build-tool: deps` declaration would also select the addon's native
build, which is unnecessary for the pure Matrix methods used here. Run the
repository's build-file validator as well as the package tests; package tests
cannot establish build-plan consistency.
