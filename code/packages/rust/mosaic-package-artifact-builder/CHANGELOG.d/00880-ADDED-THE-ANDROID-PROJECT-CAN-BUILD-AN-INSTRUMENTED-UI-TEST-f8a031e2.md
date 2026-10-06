### Added — the Android project can build an instrumented UI test (UI89 §4.2)

- The generated Android `build.gradle.kts` now names
  `androidx.test.runner.AndroidJUnitRunner` as its
  `testInstrumentationRunner`. It also declares three exactly pinned
  `androidTestImplementation` dependencies: JetBrains' `ui-test-junit4` at
  the app's own Compose version (androidx `ui-test-junit4` 1.11.2 on
  Android), `androidx.test:runner` 1.6.2 and `androidx.test.ext:junit` 1.2.1.
  An app's test placed under `src/androidTest` can then drive its real
  `MosaicActivity` on a device.
- The app APK is unchanged. `assembleDebug` never resolves the
  `androidTest` classpath, and no `debugImplementation` test manifest is
  added. A builder test checks that no test dependency sits outside
  `androidTest`.
