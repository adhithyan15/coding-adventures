### Fixed — a stalled Android document provider fails the request (UI89 §3.8)

- A stalled document provider fails the request instead of holding it
  (UI89 §3.8, which no longer lists this as a known limit). Before, a
  provider that stopped serving its pipe, never finished opening a document,
  or stopped taking a save's bytes held the one background thread, and with
  it the one file operation, until the process ended.
- `MosaicFileEffects.kt` gains `MosaicStallWatch`: when no byte has moved for
  `MOSAIC_STALL_MILLIS` (60 seconds) it runs the stop it was last given, once,
  and reports `stalled`. Progress, not total time: a slow transfer that keeps
  moving is never cut off. One daemon thread checks every watch.
  `mosaicWatchedInput` and `mosaicWriteWatched` (64 KiB pieces) feed it.
- Android reads and writes open through `openAssetFileDescriptor(uri, mode,
  signal)`: before the stream exists the watch cancels the
  `CancellationSignal`; after, it closes the descriptor with
  `closeWithError` -- so a provider reading a save through a reliable pipe
  learns it failed -- which wakes Android's blocked read or write. The
  descriptor is closed exactly once, by the watch or the transfer, and a
  transfer returning after the watch fired is failed anyway. A stopped
  transfer fails with "the selected file stopped arriving" / "the file
  stopped saving". The display-name query is watched too (`getType` takes no
  signal and is not).
- The desktop, which reads and writes local files, is unchanged.
- The Compose harness gains four checks (a stalled read stopped after the
  allowance, a slow moving read never stopped, a watch that already fired
  stopping what it is given next and a closed watch never firing, a watched
  write in pieces); 30 in all. The Android file type-checks against
  `android.jar` (API 36).
