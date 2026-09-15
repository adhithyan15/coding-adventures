---
category: Lua
---

# `--deps-mode=none` consistency

If your BUILD bootstraps sibling rocks first, the final `luarocks make` should also use `--deps-mode=none`. Don't bootstrap rocks that your tests reach via `package.path` rather than declared rockspec deps.
