---
category: Testing & coverage
---

# Dart and Flutter SDKs download through the agent proxy, so generated Dart can be analyzed locally

Cloud sessions had treated Dart and Flutter as CI-only toolchains. Generated
Dart (`mosaic_host.dart`, the emitted `main.dart`) was only checked by Rust
string tests, so a type or null-safety error first showed up in the Linux
Flutter lane, a full CI cycle later. `dl.google.com` is blocked for the
Android SDK, and that was wrongly taken to mean Flutter was blocked too.

In fact the Dart SDK zip and the Flutter SDK tarball both download through the
agent proxy from `storage.googleapis.com`. With them on `PATH`:

- `dart analyze`, `flutter analyze` and `flutter test` run locally;
- the Flutter effect-completion test in `mosaic-app-bindings` runs for real
  (it is skipped when `dart` is missing).

The UI48 ENV4-on-Flutter change was analyzed and widget-tested this way before
it was pushed.

**Do instead:** before relying on CI for a Flutter or Dart change, fetch the
SDKs into the scratchpad and run the analyzer and tests locally. They are
several GB, so delete them when the work is done if disk is tight.
