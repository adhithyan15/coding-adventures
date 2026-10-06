### Added — XCUITest sources for the iOS app, `build_package_with_ios_ui_tests` (UI89 §4.3)

- `build_package_with_ios_ui_tests` is `build_package_with_profile_runtime_and_tokens`
  plus XCUITest sources. The older function now calls it with none, so its
  output is unchanged. Each source is copied to `swiftui/UITests/<name>`
  and listed in a UI test bundle in `iOS/App.xcodeproj`. The builder also
  writes `iOS/App.xcworkspace`, holding only that project, with the shared
  scheme `xcshareddata/xcschemes/AppUITests.xcscheme`. That is what
  `xcodebuild test -workspace App.xcworkspace -scheme AppUITests` needs.
  The scheme does not go in the project itself, because the project's
  `projectDirPath` is `..`, and `xcodebuild` found no buildable platforms
  for a scheme there.
- The sources are checked before anything is emitted:
  - an iOS app build only (`swiftui`, `--emit-project`, an `.xcframework`
    runtime);
  - a regular file, not a link or a directory;
  - a plain name, `^[A-Za-z0-9_]+\.swift$`, of which only the name reaches
    the project;
  - no name given twice.
