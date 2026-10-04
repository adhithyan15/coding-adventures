### Added — SwiftUI answers file effects on iOS and iPadOS through the document picker

- SwiftUI on iOS and iPadOS answers `files.open` and `files.save` through
  `UIDocumentPickerViewController` (UI89 §3.8), where every request used to
  fail with "… is not available on this platform yet". Open reads a copy
  (`asCopy`) with the desktop limits and removes it; save checks the request
  first, writes the bytes into a fresh owner-only temporary directory and
  exports a copy, answering with the name the picker reports. The picker is
  presented on the foreground scene's topmost view controller (no window
  fails the request), and always answers: a pick, a cancel, or a cancel from
  its `deinit` when its scene goes away under it. The read or export runs
  off the main queue; its outcome is handed back to the main queue before
  the effect is completed.
- The Swift library gains Compose's picker seam (`MosaicDocumentPicker`,
  `MosaicOpenedDocument`, `MosaicSaveTarget`, `MosaicSaveRequest`,
  `MosaicAccept`, `MosaicFileFailure`, `mosaicCheckSaveRequest`,
  `mosaicAnswerFilesOpen` / `mosaicAnswerFilesSave`); the macOS panels are
  adapted by `MosaicDialogPicker`, and `mosaicRunFilesOpen` /
  `mosaicRunFilesSave` keep their signatures and outcomes.
  `installMosaicPlatformEffects` adds `picker:` and `runInBackground:`.
- The Swift harness checks the asynchronous path and the router's
  background hand-off with fake pickers; a Rust test now fences UIKit to
  `#if os(iOS)` as well as AppKit to `#if os(macOS)`.
- The table of hosts already routed is a weak list compared by identity
  instead of `NSHashTable`, so the library compiles on Linux's Foundation
  as generated; a picker is not shown for a host that closed while the
  request was queued.
