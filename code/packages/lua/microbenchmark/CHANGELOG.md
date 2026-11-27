# Changelog

## 0.1.0

- Define blank workload IDs by the six ASCII whitespace characters, accept other Unicode IDs, and reject NULs or malformed surrogate escapes.

- Add the paired collection/replay API, native array workload CLI, monotonic clock module, and report fixture.
- Preflight JSON depth and token density before recursive parsing, bounding work before AST allocation.
- Run the leaf-to-root LuaRocks build and tests from one package-local shell or batch script so the build tool's per-line process isolation cannot discard paths or environment state.
- Accept producer-supplied nonblank workload identifiers during strict report replay; the array CLI retains its own workload ID.
- Reject benchmark analysis above 10,000 pairs before ratio construction and read CLI replay paths through a bounded 4 MiB plus one byte.
- Cover signed 64-bit checksum underflow at the minimum integer in both array traversal strategies.
- Declare the native clock, environment, output, and replay-file runtime effects in the capability profile.
