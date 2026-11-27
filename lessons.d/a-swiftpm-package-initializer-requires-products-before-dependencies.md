---
category: Swift
---

# A SwiftPM Package initializer requires products before dependencies

`Package(...)` uses a declaration with fixed argument order. A fixture adapter
manifest that listed `dependencies` before `products` failed before SwiftPM
could build the adapter, even though the manifest fields themselves were
valid. The Swift Unix simulator's first external-adapter build showed the
compiler error; moving `products` before `dependencies` allowed the package to
compile and replay all nine cases.

When adding fields to a SwiftPM `Package` initializer, follow the declaration's
argument order and run `swift build` against the manifest before treating the
adapter sources as compiled.
