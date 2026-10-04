### Changed — reserve the Flutter platform library's MosaicReplaceQuestion

- `SHELL_RESERVED_NAMES` gains `MosaicReplaceQuestion`, the typedef the
  Flutter platform library's core now declares (UI87 §7.7), so a layout
  variant cannot take a name `main.dart` also imports.
