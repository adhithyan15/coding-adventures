### Changed — the Qt open dialog starts in Pictures when only images are accepted (UI59 §2)

- New `mosaicOnlyImages(extensions)` in the Qt platform library: true when
  every extension the open dialog filters on belongs to an `image/*` row of
  the same MIME table the filter comes from. No filter at all is "any file",
  not images-only. It mirrors the XAML library's `OnlyImages`.
- `QtFileDialogs::chooseFileToOpen` passes
  `QStandardPaths::PicturesLocation` as the start directory when that is true
  and the folder exists. Anything else starts exactly where it did before,
  Qt's default (an empty directory). The filter, the read cap and the read
  path are unchanged.
- The headless Qt effect driver checks `mosaicOnlyImages` for images, svg,
  mixed types, no types and a document, and `qt_effect_completion` pins
  those five checks by name.
