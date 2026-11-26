# Historical OS Simulator Backlog

**Status:** active; acceptance claims below are explicitly unverified until
reviewed against the shared fixtures and each lane's documented runner.

This queue tracks the implementation order for [HOS00](HOS00-historical-os-simulator.md).
Work one backlog item per pull request. Keep components reusable independently
of command-line or browser adapters, and use the portable fixtures as the
cross-language contract. Treat this as a simulator and teaching model: it does
not claim historical binary compatibility or execute a historical CPU.

**Operating rule:** priorities and PR publication/merge remain ordered. During
CI shortages, prepare and commit independent future items in isolated local
worktrees; do not publish or merge them ahead of the queue.

| ID | Priority | Scope | Status / exit evidence |
|---|---:|---|---|
| HOS-001 | P0 | Foundation: First Edition Unix file read, portable fixtures, reusable filesystem, descriptor table and kernel; TypeScript browser demo and hosted library ports. | Baseline implementation is present in TypeScript, Python, Ruby, Go, Perl and Rust. CI found missing prerequisite refs in the browser program's standalone BUILD recipes; the transitive paths are now declared, pending CI rerun. |
| HOS-002 | P0 | Finish the four staged reusable ports: C, C++, Java and Lua. Keep their package interfaces independent of their CLI adapters. | **Local validation complete; Windows execution pending:** all four Unix BUILD recipes passed on 2026-11-26; each ran reusable component checks and replayed all 9 shared traces. Review found Java's clean build needed to create its output directory and needed typed filesystem read errors; Lua's 5.3 minimum was undocumented. These were fixed and retested locally before publication. The component checks exercise kernel open/read/close independently of the CLI. Windows recipes were reviewed but cannot be executed in this Linux environment; retain ordered CI validation before marking complete. |
| HOS-003a | P1 | Add C# and F# reusable ports. | **Unverified / queued:** replay shared cases; document runner limits. |
| HOS-003b | P1 | Add Dart and Kotlin reusable ports. | **Unverified / queued:** replay shared cases; document runner limits. |
| HOS-003c | P1 | Add Elixir and Haskell reusable ports. | **Unverified / queued:** replay shared cases; document runner limits. |
| HOS-003d | P1 | Add OCaml and Swift reusable ports. | **Unverified / queued:** replay shared cases; document runner limits. |
| HOS-003e | P1 | Add a WebAssembly-facing lane. | **Unverified / queued:** define its reusable component boundary and document what is actually compiled and run; replay shared cases. |
| HOS-004 | P1 | Evolve the Unix model through process IDs, `fork`, `exec` and `wait`; next model descriptor inheritance and redirection; add pipes only after those transitions are covered. | **Unverified / queued:** preserve atomic failure behavior and add fixtures that expose process and descriptor state transitions. |
| HOS-005 | P2 | Add an older batch-monitor scenario, followed by a time-sharing scenario, with traces that compare job sequencing and task switching. | **Unverified / queued:** ground behavior in cited sources and keep each system milestone bounded. |
| HOS-006 | P2 | Add DOS-style console and file operations to compare application, OS and BIOS boundaries. | **Unverified / queued:** publish a separate portable contract and scenario fixtures before ports. |
| HOS-007 | Ongoing | Maintain evidence across completed lanes and milestones; investigate whether independently reusable components merit Rust candidates. | **Unverified / ongoing:** record exact checks, runner limits and supporting evidence. Candidate reuse is evidence-led and does not authorize changes to VOS. |

## Specialized language buckets

Track Mosaic, Twig and Starlark separately from the general-purpose language
port queue. First investigate a useful native boundary and a feasible runner for
each: Mosaic is a UI/application environment, Twig is a language toolchain, and
Starlark is used for build configuration. Do not count these as completed ports
without a defined contract and fixture runner. This backlog makes no VOS
changes; any future candidate component reuse requires its own evidence and
review.

## Completion evidence

For each item, record the exact commands and results for unit tests, shared
fixture replay, strict compilation/linting, and the repository build path where
applicable. State unsupported runtimes or platform-specific limitations
explicitly. A lane is complete only when its reusable component behavior and
adapter replay are both demonstrated; an implementation directory alone is not
evidence of completion.

HOS-001 CI findings are tracked here: the browser program's Unix and Windows
BUILD recipes must materialize the transitive `file:` dependency chain
(`fuzz`, `sat-solver`, reusable Unix simulator) before program install, and
the Rust shell tokenizer must pass strict Clippy. Standalone build metadata and
all-platform warnings-as-errors checks are part of completion evidence.
