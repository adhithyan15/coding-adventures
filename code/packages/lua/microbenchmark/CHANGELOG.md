# Changelog

## 0.1.0

- Add the paired collection/replay API, native array workload CLI, monotonic clock module, and report fixture.
- Preflight JSON depth and token density before recursive parsing, bounding work before AST allocation.
- Add regressions for signed-64-bit checksum underflow in both public array traversal helpers.
- Record Windows LuaCov line coverage of the installed Lua module: 96.03% (363 hit, 15 missed); native C clock coverage is separate.
