### Fixed — macOS release builds of Flutter apps compile again

- Flutter's "Replace it?" question is a `DialogRoute` pushed on the root
  navigator instead of `showDialog`. `showDialog` goes through
  `showRawDialog`, which reaches Flutter's desktop windowing code, and on
  macOS its FFI structs abort the AOT snapshotter ("Class with illegal
  cid"). Every macOS release build of a Flutter app carrying the library
  failed (Engram's release lane). The question looks and behaves as
  before; a test refuses `showDialog` and `showRawDialog` in the library.
