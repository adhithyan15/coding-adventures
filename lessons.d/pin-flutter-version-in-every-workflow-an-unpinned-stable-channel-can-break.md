---
category: Supply chain & CI pinning
---

# Pin flutter-version in every workflow; an unpinned stable channel can break a lane overnight

`release-engram.yml` set up Flutter with `channel: stable` and no
`flutter-version`, while `ci.yml`, `release-task-app.yml` and `visicalc.yml`
pin `3.44.0`. The workflow runs only when its paths change, so it sat idle
while stable moved to 3.47.6. The next PR that touched `mosaic-emit-xaml`
(#16593, a XAML-only change) ran it, and the macOS Flutter build died in
Flutter's own AOT snapshotter ("Class with illegal cid, full-aot" in
`flutter/src/widgets/_window_macos.dart`), with nothing in the diff involved.

The fix pinned the workflow to `flutter-version: '3.44.0'`, the Flutter every
other lane tests, and `test_engram_release.py` now checks the pin and that the
workflow has as many `flutter-version:` lines as `subosito/flutter-action`
steps.

**Do instead:** pin the toolchain version in every workflow, not just the
busy one. A path-filtered workflow is the one most likely to fall behind,
because nothing runs it until an unrelated PR does. Bump the pin
deliberately, in one PR, across every workflow at once.
