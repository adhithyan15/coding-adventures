### Changed — the Flutter non-modal dialog gap names the dialog route

- The `interaction.dialog-placeholder` gap for a Flutter `HostDialog` with
  `modal: false` now says a Flutter dialog route is inherently modal: the
  emitter no longer calls `showDialog` (UI29-1 §3.3).
