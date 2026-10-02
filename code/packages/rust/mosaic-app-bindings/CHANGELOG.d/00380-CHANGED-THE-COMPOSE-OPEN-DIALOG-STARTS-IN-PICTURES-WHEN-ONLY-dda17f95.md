### Changed — the Compose open dialog starts in Pictures when only images are accepted (UI59 §2)

- New `mosaicOnlyImages(extensions)` in the Compose platform library, the same
  rule as XAML's `OnlyImages` and Qt's `mosaicOnlyImages`: true when every
  extension the open dialog filters on belongs to an `image/*` row of the
  MIME table. No filter at all is "any file", not images-only.
- `AwtMosaicFileDialogs.chooseFileToOpen` sets the `FileDialog`'s directory to
  `~/Pictures` when that is true and the folder exists. The JVM has no
  Pictures-folder API; `~/Pictures` is what macOS and Windows create and most
  Linux desktops use. Anything else keeps the dialog's default start, and the
  filter and read path are unchanged.
- `conformance/compose/MosaicPlatformEffectsTest.kt` gains
  `onlyAnImageOnlyOpenStartsInPictures` (images, svg, mixed, none, a
  document). CI's Journal Compose lane runs it.
