# Unix History Simulator (C++)

[HOS00](../../../specs/HOS00-historical-os-simulator.md) kernel model with a
reusable C++17 filesystem, descriptor table and syscall dispatcher. Run
`bash BUILD` for component checks and shared fixture replay. This package ships
the reusable library API only. Its fixture recipe compiles a temporary adapter
from `code/specs/fixtures/historical-unix-v1/` and removes it after replay.
