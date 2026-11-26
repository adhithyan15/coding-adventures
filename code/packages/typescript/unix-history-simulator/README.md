# Unix History Simulator (TypeScript library)

Reusable in-memory file system, descriptor transitions, kernel dispatcher and
step-by-step shell model for [HOS00](../../../specs/HOS00-historical-os-simulator.md).

Use `resolvePath` and `readFile` independently of the descriptor table. Use
`openDescriptor`, `advanceDescriptor` and `closeDescriptor` with a table of
your own. `Kernel` combines both mechanisms, and `Session` runs a shell
command one event at a time.

`bash BUILD` checks the core, replays the cross-language traces and runs the
bounded exhaustive, property, fuzz and SAT tests.
