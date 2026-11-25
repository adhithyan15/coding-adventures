---
category: Haskell
---

# A Haskell package fixture path must name the code directory after climbing to the repository root

When a package under `code/packages/<language>/<name>` climbs four levels to
the repository root, the shared fixture still lives under `code/specs`, not
`specs`. Compute the path from the test's actual working directory and verify
the copied fixture byte-for-byte against the canonical file before parsing.
