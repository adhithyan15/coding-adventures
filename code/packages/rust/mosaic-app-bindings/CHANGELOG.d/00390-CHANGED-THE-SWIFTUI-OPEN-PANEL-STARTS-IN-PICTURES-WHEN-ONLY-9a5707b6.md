### Changed — the SwiftUI open panel starts in Pictures when only images are accepted (UI59 §2)

- New `mosaicOnlyImages(_:)` in the SwiftUI platform library. It uses the same
  rule as the XAML, Qt and Compose libraries: true when every extension the
  open panel filters on belongs to an `image/*` row of the MIME table. No
  filter at all means "any file", not images-only.
- On macOS, `MosaicSystemFileDialogs.chooseFileToOpen` sets the `NSOpenPanel`'s
  `directoryURL` to the user's Pictures folder (`.picturesDirectory`,
  `.userDomainMask`) when that is true. Anything else keeps the panel's default
  start. The filter and the read path are unchanged.
- The headless SwiftUI platform-effects harness gains `checkStartLocation`
  (images, svg, mixed, none, a document), run by CI's SwiftUI runtime lane.
