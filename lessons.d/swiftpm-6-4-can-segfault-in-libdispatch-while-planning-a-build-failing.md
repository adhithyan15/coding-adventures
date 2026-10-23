---
category: Swift
---

# SwiftPM 6.4 can segfault in libdispatch while planning a build, failing a random Swift package on an unrelated PR

On 2026-10-22/23 two `build (shard-N-of-15)` jobs on PRs that touch no
Swift code failed in a Swift package. Both times it was the same crash, in
the SwiftPM 6.4 driver (`swift-package` / `swift-build`, swift-6.4-RELEASE,
Ubuntu 24.04) while it planned a build:

```
[Pre-planning 1 / N]
*** Signal 11: Backtracing ... ***
*** Program crashed: Bad pointer dereference at 0xffff... ***
Thread 2 "DispatchWorker" crashed:
0  _dispatch_event_loop_drain + 1130 in libdispatch.so
...
Segmentation fault (core dumped)
```

- `swift/ITF` (#17824): the crash came during `swift test`'s planning. A
  second thread was in `SWBMacro/MacroNamespace.lookupMacroDeclaration`.
- `swift/zip` (#17914): all 36 tests passed, then it crashed in the
  `swift build -c release` planning step.

Every other package in those shards passed, and the build tool only
dep-skipped the crashed package's dependents. Nothing in the diff can
cause or fix this: it is a data race in the toolchain's new build-system
backend (swift-build's `SWBMacro`/`SWBCore` on libdispatch).

**What to do:** don't chase it in the PR. Read the shard log for the
`--- FAILED: swift/<pkg> ---` section. If it shows this backtrace and no
compiler or XCTest error, it is this crash. Re-run the job, or push a
base merge if you can't re-run. If it keeps recurring, the repo-level
mitigation to propose is a pinned toolchain without the race, or running
SwiftPM with `--build-system native` (the older, pre-swift-build planner)
in the Swift BUILD files or CI. That is a CI-wide change for its own PR,
not something to slip into an unrelated one.
