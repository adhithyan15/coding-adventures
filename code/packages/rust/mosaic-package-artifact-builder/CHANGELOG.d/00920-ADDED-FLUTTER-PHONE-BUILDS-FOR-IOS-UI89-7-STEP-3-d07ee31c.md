### Added — Flutter phone builds for iOS (UI89 §7, step 3)

- **`ios/` in a Flutter phone runtime**, as
  `ios/iphoneos/libmosaic_app.dylib` and
  `ios/iphonesimulator/libmosaic_app.dylib`, is read as strictly as
  `android/`. Each library's Mach-O is checked before it is copied to
  `runtime/ios/<sdk>/`:
  - the device library is thin arm64, platform 2;
  - the simulator library is fat, holding exactly arm64 and x86_64, with
    every slice's CPU type agreeing with its header and platform 7;
  - every offset is bounds-checked;
  - a library without `LC_BUILD_VERSION` is refused, because it cannot
    tell device from simulator.
- **The phone `hook/build.dart` gains an iOS branch.** It chooses by
  `IOSSdk`, slices with `lipo` through `Process.run`, and checks the
  slice's platform again.
- **The README's `flutter create` command** names only the platforms the
  runtime has (`android`, `ios`, or `android,ios`).
