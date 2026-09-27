### Added — the Qt platform library, and one routed handler on the Qt host (UI87 §7.4a)

- `MosaicHost` (Qt) gains `setEffectHandler` / `effectHandler`: one routed
  handler that, when set, receives each effect **instead of** the
  `effectRequested` signal (never both, so an effect has one owner). When
  empty, the signal is emitted as before. The handler is called through a
  copy, since it may replace itself.
- `deferEffect` refuses an id another handler already owns ("already owned by
  another handler"): a second owner would show a second dialog and have its
  answer refused.
- `qt_platform_effects()` returns `MosaicPlatformEffects.{h,cpp}`: `files.open`
  and `files.save` through `QFileDialog` (when Widgets is linked), answered
  inline like the existing Qt handlers, with the Compose/SwiftUI contract --
  limits, MIME table, the plain-name rule and executable list (a Rust test
  pins both to the Kotlin library). On Unix the chosen file is opened once
  (`O_NONBLOCK | O_NOFOLLOW`, type from `fstat`), and a save is written to an
  owner-only `O_EXCL` temporary, given the replaced file's rwx bits with
  `fchmod`, and renamed into place; elsewhere `QSaveFile`.
- `installMosaicPlatformEffects(host, appKinds, dialogs)` sets the router:
  app-claimed kinds go to the handler it replaced, or else to the old signal,
  so `connect`ed package handlers (Engram, photo-picker) keep working
  unchanged; unclaimed standard kinds are answered here; one file operation at
  a time; idempotent.
- The Qt effect driver (`tests/qt_effect_driver`, Qt Core only, headless)
  gains checks for the routing through the real host (an app kind reaches a
  connected handler once; an unclaimed one is failed), the single owner, and
  the library itself with fake dialogs.

