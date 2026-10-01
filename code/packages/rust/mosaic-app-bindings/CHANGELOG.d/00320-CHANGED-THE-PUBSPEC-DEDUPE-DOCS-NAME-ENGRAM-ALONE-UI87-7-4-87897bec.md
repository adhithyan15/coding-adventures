### Changed — the pubspec dedupe docs name Engram alone (UI87 §7.4)

- `flutter_pubspec_with_host_asset_dependencies` and its test now name only
  Engram as a package that declares `file_selector` for its own handler.
  photo-picker-app retired its Flutter handler and that coordinate for the
  Flutter platform library (UI87 §7.4), so it no longer declares one.
  Documentation only; behaviour is unchanged.
