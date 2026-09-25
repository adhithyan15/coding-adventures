# Changelog

## 0.1.0 — unreleased

- **New crate (UI89 §2.2).** `project_pbxproj` writes the Xcode project for a
  Mosaic app on iOS and iPadOS: one application target for iPhone and iPad,
  the generated Swift and the C runtime loader, the runtime `.xcframework`
  linked, `Info.plist` from build settings, and deterministic object ids.
  `module_map` makes the loader importable from Swift in both the Xcode
  project and the Swift package; `default_bundle_identifier` derives an
  identity from a package name. Strings are quoted and escaped, and absolute
  paths, `..`, `$(…)` and control characters are refused. Verified: Trestle
  built from the generated project installs and runs on the iPhone simulator.
