### Added — every Flutter app gets the platform library (UI87 §7.7)

- `lib/mosaic_platform_effects.dart` and `lib/mosaic_platform_effects_core.dart`
  are written beside `lib/mosaic_host.dart` in every generated Flutter project,
  byte for byte as `mosaic-app-bindings` emits them, and `pubspec.yaml` gains
  `file_selector: 1.0.4` (before the package's `[host_assets]` dependencies,
  which skip a package already declared).
- `flutter_main_with_host_effects` now also installs the library, after the
  package's `[host_effects]` handler: `installMosaicPlatformEffects(host,
  appKinds: …)` on the strict shell's local before the first props read, or
  through the permissive shell's single guarded `mosaicEffectHost` local, and
  imports `mosaic_platform_effects.dart`. The handler's `kinds` become
  `const <String>['…']` (re-checked against the dotted-name shape, so a quote,
  backslash or `$` is a build error), an empty list `const <String>[]`, none
  `null`. An entry point with no host anchor and no declared handler is left
  as it is; a declared handler there is still refused.
- Tests: the platform install alone, after the package handler with and
  without kinds, a Qt-only handler's kinds not leaking into Flutter, the kind
  check (including Dart interpolation), both emitted shells wired in order,
  the native-complete project's files, pubspec pin and install, and the
  emitted file list.
- CI (Flutter runtime lane): after the TaskApp build, a check that its
  `main.dart` installs the library on the started host, then the headless
  harness against TaskApp's generated core and host.

