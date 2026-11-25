---
category: Lua
---

# Lua numeric literals do not allow digit separators

Lua 5.4 does not accept underscore digit separators such as `1_000_000`.
Write integer literals without separators and run `luac -p` before executing
tests so syntax errors are caught before package integration.
