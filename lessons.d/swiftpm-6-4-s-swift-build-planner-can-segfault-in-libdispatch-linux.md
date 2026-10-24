---
category: Swift
---

# SwiftPM 6.4's swift-build planner can segfault in libdispatch; Linux CI pins --build-system native

Twice on 2026-10-22/23, a CI shard on a PR that touches no Swift failed in
a Swift package. Both times `swift-package` (Swift 6.4, ubuntu-24.04 image)
crashed while planning:

```
[Pre-planning 1 / N]
*** Program crashed: Bad pointer dereference at 0xffff... ***
Thread 2 "DispatchWorker" crashed:
0  _dispatch_event_loop_drain + 1130 in libdispatch.so
Segmentation fault (core dumped)
```

A second thread was in swift-build's `SWBMacro.MacroNamespace
.lookupMacroDeclaration`. That is the new swift-build planner in 6.4.
Failures: `swift/ITF` during `swift test` planning (#17824), and `swift/zip`
during `swift build -c release` planning after 36 passing tests (#17914).

**Fix:** the Go build tool's `swiftNativeBuildSystemCommands` adds
`--build-system native` to every `swift build|test|run` on Linux at run
time. BUILD files, their hashes and the scaffold generator are unchanged.
macOS, Windows, and commands that already choose a build system are left
alone.

**Recognise it:** a `--- FAILED: swift/<pkg> ---` section showing this
backtrace and no compiler or XCTest error. **Undo it:** delete the function
once a toolchain without the race ships, then check that a full build stays
green.
