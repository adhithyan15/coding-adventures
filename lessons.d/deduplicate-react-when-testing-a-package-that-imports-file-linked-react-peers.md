---
category: TypeScript / JavaScript
---

# Deduplicate React when testing a package that imports file-linked React peers

An npm `file:` dependency is a symlink to the sibling package, so imports from
that sibling resolve its own installed `react` peer before the consumer's copy.
When a React component package is tested through another file-linked package,
this creates two React realms and fails at runtime with `Invalid hook call`
even though every version constraint agrees. Keep React and React DOM as peers,
install the complete local dependency graph for typechecking, and configure the
consumer bundler/test runner to deduplicate `react` and `react-dom` so all
components share the renderer's realm. Exercise at least one real child
component in the consumer test; a shallow or mocked composition will not catch
the duplicate-runtime failure.
