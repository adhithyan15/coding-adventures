### Added — an installable iOS / iPadOS app target (UI89 §2.2)

- A SwiftUI build whose `--runtime-library` is an `.xcframework` also writes
  `iOS/App.xcodeproj/project.pbxproj` (from the new `mosaic-ios-project` crate),
  so any Mosaic package with a static runtime becomes an `.app` that installs
  on iPhone and iPad. It is written last, after host assets and host effects,
  and lists every `.swift` under `Sources/App` the way SwiftPM compiles it;
  symbolic links there are refused. The app's name and identity come from
  `[app] display-name` / `bundle-identifier`, defaulting to the root component
  and `dev.codingadventures.<package>`.
- `Sources/CMosaicRuntime/include/module.modulemap` is written with it, so the
  Xcode project and the Swift package import the loader as one module.

