### Added — Anki export and import, tested through the picker on Android and iPadOS (UI89 §4.4)

- `conformance/compose-android/EngramAndroidUiTest.kt` (instrumented,
  through `mosaic-android-ui-test.sh` on the CI emulator). Espresso-Intents
  answers the system's document picker with a `file://` document in the
  app's cache. The test exports and waits for a whole zip (local header
  first, end record last). It then imports that same file, and the
  package's `Default` deck appears in the deck list. A second cold launch
  finds the deck still listed.
- `conformance/swiftui-ios/EngramUiTests.swift` (XCUITest on a landscape
  iPad simulator) does the same round trip. It launches the app with
  `-MosaicUITestPicker`, which selects the Debug-only fake picker that
  `--ios-ui-test` builds in.
- Both tap Import up to three times. The router allows one file operation
  at a time, and nothing on screen says when the export's answer has
  arrived, so an Import tapped first is refused, as it would be for a
  person.
