### Fixed - HostDialog lowers to a real native dialog, not a placeholder (#13010)

`emit_host_dialog` unconditionally emitted `const SizedBox.shrink() /*
TODO: HostDialog showDialog wiring */` — every Mosaic `Modal`/`HostDialog`
on Flutter rendered nothing at all. Genuinely incomplete, not a
documented platform limitation the way XAML's Modal gap is (#13008).

Implemented the `modal: true` (default, and the only value the
toolkit's own `Modal` component ever authors) case for real: a new
shared `_MosaicDialogHost` `StatefulWidget` (vanilla Flutter, no new
package dependency) bridges the declarative `open: bool` contract
every backend shares to Flutter's imperative `showDialog`/`Navigator`
API — `open` flipping false→true schedules `showDialog` on the next
frame; flipping true→false while still showing (the host closed it via
its own slot, not backdrop-tap) pops the route programmatically. Either
dismissal path resolves `showDialog`'s `Future`, which is where
`onClose` dispatches exactly once regardless of which side closed it.
`title`, `dismiss-on-backdrop` (→ `barrierDismissible`), and children
(→ `AlertDialog.content`, wrapped in a `Column` for multiple children)
are all wired following the exact property table every other backend
already implements.

`modal: false` is NOT implemented and deliberately out of scope here:
Flutter's `showDialog` is inherently modal (a full-screen route +
barrier), with no vanilla-Flutter equivalent to SwiftUI's `.popover`/
Qt's non-modal `Popup` short of a custom `Overlay`. That shape keeps
the old placeholder and keeps reporting `interaction.dialog-placeholder`
(now gated on a new `host_dialog_has_native_semantics` predicate,
mirroring the existing `host_table_has_native_semantics` pattern, so
`mosaic-package-artifact-builder` only reports the degradation for the
still-unimplemented shape). Removed the now-stale
`(Backend::Flutter, "Modal", "interaction.dialog-placeholder")`
allowlist entry from `mosaic-pkg-toolkit`'s native-complete gate — the
toolkit's `Modal` always authors `modal: true`, so it's genuinely clean
on Flutter now.

Verified with 7 new tests plus a real `flutter analyze` (0 issues) and
`flutter pub get` against the actual generated project shell for all 23
`mosaic-pkg-toolkit` components — not just Rust-level string assertions.

