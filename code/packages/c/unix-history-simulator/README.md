# Unix History Simulator (C)

[HOS00](../../../specs/HOS00-historical-os-simulator.md) kernel model with a reusable
C interface in `include/unix_history.h`. The filesystem owns lookup and byte
ranges. The caller provides the descriptor storage and can reuse it on its own.

The package is a library and does not ship a command-line program. Run `bash BUILD`
for component checks and the shared fixture replay; that recipe compiles a
temporary fixture adapter from `code/specs/fixtures/historical-unix-v1/` and
removes it when the checks finish.
