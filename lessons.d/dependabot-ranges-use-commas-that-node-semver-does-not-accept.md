---
category: Supply chain & CI pinning
---

# Dependabot ranges use commas that node-semver does not accept

The alert API reports ranges such as `>= 1.0.0, < 1.2.2`. Passing that string
directly to node-semver's `satisfies` returns false even for 1.2.1. An initial
local comparison therefore incorrectly reported no matching vulnerable entries.
Normalize comma separators into spaces and reject every unrecognized range with
`validRange` before inspecting the versions. Independently run `npm audit` on
the actual locks; after correction, all 409 API alerts were checked.
