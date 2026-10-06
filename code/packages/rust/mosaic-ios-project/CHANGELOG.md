# Changelog

## 0.1.0 — unreleased

- **UI test bundle and shared scheme (UI89 §4.3).** `IosApp.ui_test_sources`
  (empty by default) adds a `com.apple.product-type.bundle.ui-testing` target.
  It is named `<product>UITests`, its bundle identifier is the app's plus
  `.uitests`, and `TEST_TARGET_NAME` names the app. A
  `PBXTargetDependency` on the app means `xcodebuild test` builds the app
  first. Its sources compile in its own phase, never the app's. The paths
  are checked like every other path.
  - `shared_scheme` returns the scheme XML (build, launch, and test with the
    bundle), with every attribute value XML-escaped, or `None` without UI
    tests. Without them the project is byte-for-byte what it was.
  - `cargo fmt` was applied to the whole file with this change.
- **New crate (UI89 §2.2).** `project_pbxproj` writes the Xcode project for a
  Mosaic app on iOS and iPadOS: one application target for iPhone and iPad,
  the generated Swift and the C runtime loader, the runtime `.xcframework`
  linked, `Info.plist` from build settings, and deterministic object ids.
  `module_map` makes the loader importable from Swift in both the Xcode
  project and the Swift package; `default_bundle_identifier` derives an
  identity from a package name. Strings are quoted and escaped, and absolute
  paths, `..`, `$(…)` and control characters are refused. `source_root`
  (`""` or `".."`) becomes `projectDirPath`, so the project can live in a
  subdirectory of the sources. Verified: Trestle
  built from the generated project installs and runs on the iPhone simulator.
