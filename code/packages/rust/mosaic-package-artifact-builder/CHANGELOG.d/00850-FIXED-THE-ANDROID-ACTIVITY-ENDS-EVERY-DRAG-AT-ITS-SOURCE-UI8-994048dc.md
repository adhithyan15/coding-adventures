### Fixed — the Android activity ends every drag at its source (UI89 §3.5)

- `MosaicActivity` wraps its content -- `MosaicStartup` in a native-complete
  app, `MosaicApp` in a sample one -- in `MosaicDragEndWatcher`, so every
  drag's end reaches its source on Android, including a drag no component
  was interested in (UI89 §3.5).
- `MosaicDragEndWatcher` joins the Compose shell's reserved names, so a
  layout variant or an export cannot take it.
