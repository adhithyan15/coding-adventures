### Added — package handlers can answer their own kinds through the Swift picker (UI89 §2.6)

- `installMosaicPlatformEffects` now records each host's router.
  `mosaicPlatformRouter(for:)` returns it, or nil when the library was never
  installed on that host.
- `MosaicPlatformRouter.openForApp(_:accept:limit:ok:)` and
  `saveForApp(_:suggestedName:bytes:accept:ok:)` answer an app's own Await
  through the library's picker. They keep `files.*`'s rules: one file
  operation at a time, deferred before anything is shown, slow work off the
  main queue, exactly one answer, and the same save-name checks (now
  `mosaicCheckSaveName`). The app supplies the accepted extensions, the read
  limit and the `ok` answer.
- `files.open` and `files.save` now run on the same two operations
  (`mosaicAnswerOpen`, `mosaicAnswerSave`), so each has one implementation.
  Their behaviour is unchanged.
- The Swift harness gained checks of the app path:
  - lookup of an uninstalled host;
  - app kinds still reaching the app first;
  - the app's extensions, limit and `ok` shape;
  - one operation at a time;
  - refused names showing no picker;
  - no picker on the OS.
