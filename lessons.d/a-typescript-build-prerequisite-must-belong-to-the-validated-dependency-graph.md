---
category: BUILD files & dependency management
---

# A TypeScript BUILD prerequisite must belong to the validated dependency graph

General BUILD-file lessons do not override the repository build tool's
package-specific dependency graph. Adding a setup line for a local TypeScript
package that is not a declared direct or transitive dependency fails validation
as an undeclared local reference. Before copying a generally recommended
prerequisite, run the diff-based build validator; add the line only when the
package graph requires it. Conversely, when a library gains a local dependency,
update every reported standalone dependent BUILD in leaf-to-root order.
