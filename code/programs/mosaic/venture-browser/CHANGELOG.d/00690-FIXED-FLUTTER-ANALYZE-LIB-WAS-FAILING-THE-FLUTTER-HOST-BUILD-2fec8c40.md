- Fixed: `flutter analyze lib` was failing the Flutter host build over an
  `unnecessary_import` on `dart:typed_data` in `mosaic_host.dart` -- its only
  use, `Uint8List`, is already re-exported transitively by
  `package:flutter/services.dart`. `flutter analyze lib` has no severity
  flags, so any reported issue, including this info-level one, failed the
  step. Removed the redundant import.
