### Fixed — HostDialog pushes its own dialog route, not showDialog (UI29-1 §3.3)

- `_MosaicDialogHost`, the widget every `HostDialog` lowers to, pushes a
  `DialogRoute` on the root navigator instead of calling `showDialog`.
  `showDialog` goes through `showRawDialog` and Flutter's desktop windowing
  code, whose macOS FFI structs abort the AOT snapshotter ("Class with
  illegal cid"), the failure that broke every macOS release build of a
  Flutter app while the platform library carried the call (UI87 §7.7). The route keeps what `showDialog` gave it: the themed barrier
  colour, Tab held inside the dialog, and `barrierDismissible`.
- One route per open: several rebuilds before the frame no longer push a
  second dialog.
- A host-driven close (`open` set to false) removes this dialog's own route
  after the frame -- popped when it is on top, otherwise taken out from
  under what was pushed above it. Before, `maybePop` popped whatever route
  was on top.
- `onClose` fires once per open, and not for a component that has left the
  tree; such a component's dialog is now removed with it rather than left
  showing.
- A unit test refuses `showDialog`, `showRawDialog`, `showGeneralDialog` and
  `maybePop` in the helper. New `tests/flutter_dialog_host.rs` writes the
  helper exactly as emitted into a throwaway Flutter package and runs the
  nine widget tests in `conformance/dialog-host/` (barrier, Escape,
  in-dialog pop, host close, a route above the dialog, reopening, a double
  open, a non-dismissible barrier, disposal). It skips without `flutter`
  unless `MOSAIC_REQUIRE_FLUTTER` is set, as CI's Flutter lane now does,
  and names `--reporter expanded`, since under GitHub Actions `flutter test`
  would pick a reporter whose summary differs.
  `emit_dialog_helper` is public for it.
- The module docs no longer describe a `flutter_hooks` design that was
  never built. Spec: UI29-1 §3.3 now describes the Flutter lowering.
