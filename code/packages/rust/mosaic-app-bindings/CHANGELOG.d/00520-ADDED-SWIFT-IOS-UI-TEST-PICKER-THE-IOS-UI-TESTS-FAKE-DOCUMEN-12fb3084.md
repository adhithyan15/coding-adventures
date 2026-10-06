### Added — `swift_ios_ui_test_picker`, the iOS UI tests' fake document picker (UI89 §4.4)

- `templates/swiftui/ios-ui-test/MosaicUITestPicker.swift` is a
  `MosaicDocumentPicker` for XCUITests. `create` writes the bytes being
  saved to `tmp/mosaic-ui-test-picker/document` in the app's container.
  `open` reads that file back, and answers as a cancel does when the file
  does not exist. Both answer later, on the main queue, as the system
  picker does. Every other rule of the router still runs.
- All of the file is inside `#if MOSAIC_UI_TEST_PICKER && os(iOS)`.
  `mosaicSystemPicker` returns the fake only under the same condition, and
  only when the app was launched with `-MosaicUITestPicker`. The argument
  only selects the fake and carries no path. No build defines the
  condition except the Debug configuration of an iOS app built with
  `--ios-ui-test`. A test pins the guard, the hook and the argument.
