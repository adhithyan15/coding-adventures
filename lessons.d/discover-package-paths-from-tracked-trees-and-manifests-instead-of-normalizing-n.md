---
category: BUILD files & dependency management
---

# Discover package paths from tracked trees and manifests instead of normalizing names across languages

I inferred a package directory by translating its name from another language's
convention. That made a real package appear missing even though it was tracked
under a different spelling. Before declaring a package absent or a dependency
uncovered, search the tracked tree and inspect the owning manifests. Keep the
language-specific path as evidence instead of assuming names normalize across
languages.
