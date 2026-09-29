### Added — an Android project beside every Compose project (UI89 step 4, §3.5)

- A Compose build with `--emit-project` also writes `compose/android/`: a
  Gradle project (Android Gradle Plugin 8.13.0, Kotlin 2.3.21, compile/target
  SDK 36, min SDK 26, Gradle 8.14.3 named in `gradle-wrapper.properties`),
  `AndroidManifest.xml` with one launcher activity, the label in
  `res/values/strings.xml`, `mosaic.android.MosaicActivity`, and the Android
  `MosaicPlatform.kt`.
- The shared Kotlin (`MosaicAppShell.kt`, `MosaicRuntimeHost.kt`, every
  exported component and its layout variants) is copied from
  `src/main/kotlin` after the installers run, so host-asset replacements
  reach Android; `Main.kt`, the AWT seams, the platform library and package
  effect handlers stay desktop-only.
- The activity keeps state in `filesDir` and loads the host the way the
  desktop `Main.kt` does: `MosaicStartup` for native-complete, the sample
  fallback otherwise.
- The two XML files' banners do not quote the command line: XML forbids `--`
  inside a comment, and aapt refused the file (a test now checks every
  generated XML comment).
- `android_application_id` makes `[app] bundle-identifier` legal for Android
  (`-` → `_`, an `x` before a part that starts with a digit, `_` after a Java
  keyword); `android_strings_xml` escapes the label for Android's string
  syntax and XML.

