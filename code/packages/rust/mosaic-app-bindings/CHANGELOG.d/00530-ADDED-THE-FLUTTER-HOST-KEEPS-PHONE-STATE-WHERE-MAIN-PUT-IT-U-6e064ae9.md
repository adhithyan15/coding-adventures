### Added — the Flutter host keeps phone state where `main()` put it (UI89 §7.4)

- `mosaic_host.dart` has a top-level `String? mosaicStateRoot`, which a
  phone build's `main()` sets from `path_provider`.
- On Android and iOS, `_statePath` uses only `MOSAIC_APP_STATE_PATH` or
  `mosaicStateRoot`, never `HOME` or `XDG_DATA_HOME`. With neither,
  persistence is off and the host warns: "Mosaic state is not saved: this
  phone has no app-support directory for it."
- `MOSAIC_APP_LIBRARY` is ignored on a phone: the bundled runtime is
  always used.
- iOS no longer shares macOS's `HOME` root. Desktop roots are unchanged.
- `FLUTTER_PATH_PROVIDER_VERSION` (2.1.6, exact) and
  `flutter_pubspec_with_path_provider` add the dependency a phone build
  needs.
