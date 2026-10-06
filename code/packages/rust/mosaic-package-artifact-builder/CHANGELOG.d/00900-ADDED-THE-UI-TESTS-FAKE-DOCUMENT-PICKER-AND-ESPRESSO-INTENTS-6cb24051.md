### Added — the UI tests' fake document picker and Espresso-Intents (UI89 §4.4)

- **iOS.** With `--ios-ui-test`, the builder also writes
  `swiftui/UITestSupport/MosaicUITestPicker.swift`
  (`mosaic_app_bindings::swift_ios_ui_test_picker`). It lists that file in
  the app target of `iOS/App.xcodeproj` and defines `MOSAIC_UI_TEST_PICKER`
  in the app target's Debug configuration only. A UI test that launches the
  app with `-MosaicUITestPicker` then gets a fake document picker, which
  saves to and opens one fixed file in the app's temporary directory. Its
  export-then-import is a round trip with no fixture. The file is outside
  `Sources/App`, so the Swift package never compiles it, and all of it is
  inside the condition. A Release build, or any build without the flag,
  therefore has no code that reads the argument. The builder test checks
  each of these.
- **Android.** The generated `build.gradle.kts` adds
  `androidTestImplementation("androidx.test.espresso:espresso-intents:3.6.1")`,
  from the same androidx.test release as the runner. A UI test uses it to
  answer `ACTION_OPEN_DOCUMENT` and `ACTION_CREATE_DOCUMENT` with a file in
  the app's own cache. No provider is declared, and the builder test checks
  that the app's manifest has none and the project has no androidTest or
  debug manifest.
