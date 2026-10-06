---
category: Mosaic compiler pipeline
---

# A package-owned host file must keep up with every member the generated host gains, because the build installs libraries that call them

**What went wrong.** PR #16322 made every generated Flutter project install
Mosaic's platform library (UI87 §7.7). The library adapts the host through
`MosaicHost.effectHandler`, `deferEffect` and `completeEffect`, and those
members were added to the generated host template. Venture replaces the
generated host with its own `host/flutter/mosaic_host.dart` (a `host_files`
entry targeting `lib/mosaic_host.dart`), which had none of them. `main`
stayed green only because nothing rebuilt Venture there. The next Venture
PR, #16178, failed `flutter analyze lib` with four undefined members in
`mosaic_platform_effects_core.dart`, a file that PR never touched.

**The fix.** Venture's host gained the three members, with the semantics
the generated host has without a runtime. Its native session answers its
own effects, so the handler is stored and never called, `deferEffect` is
false, and `completeEffect` is null.

**Next time.** When a change makes a generated host's members load-bearing
(a library the build installs calls them), search the packages for
`host_files` entries that replace that host file
(`rg 'target = "lib/mosaic_host.dart"' code/programs`; likewise
`MosaicHost.kt`, `MosaicHost.swift`, `MosaicHost.cs`, `MosaicHost.h`). Build
each one, or add the members to it in the same PR. CI only rebuilds packages
the diff touches, so a broken package-owned host stays hidden until its
owner's next PR.
