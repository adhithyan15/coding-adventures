---
category: Workspace & package metadata
---

# Lua rockspecs must pin immutable refs

(release tag or commit SHA) over `https://`, never moving branch tips. Patch flaky LuaRocks GitHub-archive URLs to the stable `archive/refs/tags/<tag>.tar.gz` form during CI install.
