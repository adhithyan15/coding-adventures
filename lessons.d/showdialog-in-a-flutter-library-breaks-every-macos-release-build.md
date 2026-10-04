---
category: Supply chain & CI pinning
---

# showDialog in a Flutter library breaks every macOS release build

The Flutter platform library's "Replace it?" question (#16581) used
`showDialog`. Since the experimental desktop windowing API, `showDialog` goes
through `showRawDialog`, which can open a dialog as a window of its own and
so reaches `flutter/src/widgets/_window_macos.dart`. On macOS that file's FFI
structs abort the AOT snapshotter:

    Unexpected object (Class with illegal cid, full-aot): ... _window_macos.dart  Class: _Rect
    Dart snapshot generator failed with exit code -6

The failure showed up only in Engram's path-filtered release lane, on an
unrelated XAML PR (#16593), because nothing else builds a Flutter app for
macOS in release mode. Linux release AOT builds of the same app succeed. It
reproduced on Flutter 3.47.6 and on 3.44.0, so it was not the unpinned
`stable` channel the lane used, although that was the first suspect: pinning
the version did not fix it (the pin stays, for consistency with every other
lane).

The fix pushes a `DialogRoute` on the navigator, which is what `showDialog`
pushes when windowing is off, and a Rust test refuses `showDialog` and
`showRawDialog` in the library's code.

**Do instead:** in shared Flutter templates, avoid `showDialog` and
`showRawDialog` until Flutter's macOS windowing compiles AOT. Push a
`DialogRoute` instead. Before blaming a toolchain bump for a failure, check
which recent commits touched the failing app's inputs. The two green runs on
either side of #16581 pointed at it at once. The emitter's `HostDialog`
lowering (`_MosaicDialogHost`) made the same call and now pushes its own
`DialogRoute` too (UI29-1 §3.3).
