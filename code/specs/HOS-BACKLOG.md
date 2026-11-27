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
| HOS-001 | P0 | Foundation: First Edition Unix file read, portable fixtures, reusable filesystem, descriptor table and kernel; TypeScript browser demo and hosted library ports. | Merged to main as `593bf8dfca8c5ed41df0c3acf18af926a5cfda9c` after the browser prerequisite fix; hosted build validation passed. |
| HOS-002 | P0 | Finish four reusable library ports: C, C++, Java and Lua, independent of command-line adapters. | **Local validation complete; Windows execution pending:** all four Unix BUILD recipes passed on 2026-11-27; each ran reusable component checks and replayed all 9 shared traces using temporary fixture adapters outside the package roots. The shipped package APIs use in-memory images and have explicit empty capability profiles. User-facing CLI adapters are not part of this release. Windows recipes were reviewed but cannot be executed in this Linux environment; retain ordered CI validation before marking complete. |
| HOS-002a | P1 | Optional C, C++, Java and Lua user-facing CLI adapters. | **Deferred:** stdout output requires `stdout:write:*` package capabilities and hardware-key approval under D21 / 13-capability-security. The current fixture adapters live outside package roots and are compiled only to temporary build outputs; they are not public package commands. |
| HOS-002b | P1 | Route historical-unix-v1 fixture changes to affected-only CI consumers. | **Implemented in ordered increments:** merged #20206 routes `file-read.json` to the original 10 consumers (C, C++, Go, Java, Lua, Perl, Python, Ruby, Rust and TypeScript), with exact rows for the shared `replay_cli.py`, `build_native.py` and compiled C/C++/Java/Lua adapters. #20214 adds C# and F# routes for `replay_cli.py`, `build_dotnet.py` and each adapter source/project. HOS003b adds Dart for `replay_cli.py` and `test-adapters/dart/fixture_cli.dart`, plus Kotlin for `test-adapters/kotlin/FixtureReplayTest.kt`; both also hash `file-read.json`. HOS003d adds OCaml and Swift consumers for the shared replay helper/driver and their fixture-adjacent adapters. Exact path, language-filter, lookalike and hash invalidation tests cover the registry. Ordered hosted validation remains pending. |
| HOS-003a | P1 | Add C# and F# reusable ports. | **Merged:** PR #20214 squash `de07e200f704c7e77780aa6cdbe15cce3f45df08`; required CI checks passed, including Windows and metadata. Local SDK 9.0.319 builds had 0 warnings/errors, component checks passed, and each package replayed all 9 shared cases. |
| HOS-003b | P1 | Add Dart and Kotlin reusable ports. | **Merged:** PR #20215 squash `5591986d5cf7b998974b4e075ab461a4a7d6520b`; required CI checks passed. Local Linux component tests, all 9 shared fixture replays, Dart analysis, and exact repository-input tests passed. |
| HOS-003c | P1 | Add Elixir and Haskell reusable ports. | **Merged:** PR #20216 squash `9fa7c4300975337ae87eae1cecbdc4cb522740d8`; required CI checks passed. Local validation replayed all 9 canonical traces for each external adapter. Local Elixir used cached OTP/ERTS 27.3.4.1 rather than CI's 27.3.4.11; local Haskell used GHC 9.6.6 rather than CI's 9.4.8. Cabal 3.10.3.0 matched. |
| HOS-003d | P1 | Add OCaml and Swift reusable ports. | **Local Linux validation complete; ordered CI pending:** OCaml 5.2.1/Dune 3.17.2 passed component tests, formatting, package build and all 9 canonical fixture replays; Swift 6.3.3 passed 3 XCTest cases and all 9 replays. Both packages have empty capability profiles and keep fixture I/O in external adapters. Windows execution is pending; Swift 6.3.3 differs from CI's 6.4.2. |
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
