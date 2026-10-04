# Flutter "Replace it?" dialog test (UI87 §7.7)

On Linux the Flutter platform library's save dialog (`file_selector`, over
GTK's file chooser) does not ask before saving over an existing file, so the
library asks itself: `mosaicConfirmReplacing` decides whether to ask (tested
headless in `../flutter-platform-effects/`) and `mosaicAskToReplace` asks, in a
Material dialog on the app's root navigator.

`replace_dialog_test.dart` drives that dialog with Flutter's widget tester:

- **Replace** answers true and closes the dialog, which names the file;
- **Cancel**, or dismissing the dialog, answers false (the save is a cancel);
- the root navigator is found without `main.dart` handing over a key;
- with no navigator on screen the question throws, which fails the save
  rather than replacing a file nobody was asked about.

It needs the Flutter engine, so it runs inside a generated project:

```bash
cp replace_dialog_test.dart <project>/flutter/test/mosaic_replace_dialog_test.dart
(cd <project>/flutter && flutter test test/mosaic_replace_dialog_test.dart)
```

CI does this in the Flutter TaskApp lane, against TaskApp's generated library.
