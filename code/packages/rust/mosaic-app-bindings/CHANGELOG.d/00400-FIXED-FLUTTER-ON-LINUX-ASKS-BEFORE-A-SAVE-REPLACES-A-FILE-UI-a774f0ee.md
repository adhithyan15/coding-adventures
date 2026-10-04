### Fixed — Flutter on Linux asks before a save replaces a file (UI87 §7.7)

- On Linux the Flutter platform library now asks before a save replaces a
  file. GTK's chooser, as `file_selector_linux` opens it, never asks;
  NSSavePanel and the Windows dialog do. `mosaicConfirmReplacing` (core,
  plain Dart) asks only when the dialog did not and something is at the
  chosen path -- a dangling link counts -- and a "keep" answer is a cancel.
  `mosaicAskToReplace` asks in a Material dialog on the app's root navigator,
  found by walking the widget tree, so `main.dart` hands over no key; with no
  navigator it throws, and the save fails rather than replacing a file
  nobody was asked about.
- The headless Flutter harness checks when the question is asked, that it
  names the file and not the path, and that an unaskable question fails the
  save without touching the file. New `conformance/flutter-replace-dialog/`
  drives the real dialog with the widget tester (Replace, Cancel, dismissal,
  no navigator); CI runs it in the Flutter TaskApp lane.
