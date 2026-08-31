# TaskApp macOS application bundle v1

Issue: [#13612](https://github.com/adhithyan15/coding-adventures/issues/13612)

## Decision

The product-scoped release lane publishes the strict SwiftUI build as a directly
runnable `Trestle.app` ZIP. This is an incremental development release: it is not
Developer ID signed or notarized, and the release notes must say so plainly.

The generated SwiftPM project remains a separate source artifact. Its guarded
SwiftUI sources retain iOS portability, but `Trestle.app` and the bundled
`libmosaic_app.dylib` are macOS artifacts only.

## Stable application metadata

`Trestle.app/Contents/Info.plist` carries:

- bundle identifier `org.codingadventures.trestle`;
- display, bundle, and executable name `Trestle`;
- the exact product SemVer as `CFBundleShortVersionString`;
- the numeric SemVer core as `CFBundleVersion`;
- minimum system version macOS 13;
- productivity application category and high-resolution support;
- `Trestle.icns` icon metadata; and
- Mosaic application identity `task-app`.

The live state remains at
`~/Library/Application Support/task-app/mosaic-state.v1.json`; upgrade, backup,
restore, uninstall/purge, and quarantine behavior is defined by the shared
[local-data operations contract](task-app-local-data-operations-v1.md) and copied
into the bundle as `Contents/Resources/LOCAL-DATA.txt`.

The archive records its actual release-runner architecture in `BUNDLE.json`
instead of claiming universal or Apple-silicon coverage that was not built.

## Bundle layout

The SwiftPM executable is installed at
`Trestle.app/Contents/MacOS/Trestle`. SwiftPM's `App_App.bundle` stays at the app
root because its generated `Bundle.module` accessor resolves that resource bundle
relative to `Bundle.main.bundleURL`. The exact selected Rust release library stays
at `App_App.bundle/Runtime/libmosaic_app.dylib`.

Release-owned provenance and disclosure files live under `Contents/Resources`:
`SOURCE_COMMIT`, `BUNDLE.json`, and `INSTALL.txt`. The latter tells users that
the app is unsigned/not notarized and recommends an explicit first-open approval
without weakening system-wide Gatekeeper.

## Verification

The release workflow must:

1. build `task-mosaic-app` and the generated Swift package in release mode;
2. compare SwiftPM's installed dylib byte-for-byte with the Rust artifact;
3. archive and extract two independent copies of `Trestle.app`;
4. validate the property list, stable identity, display name, version, executable,
   icon, and the extracted dylib bytes;
5. use the standard SwiftUI runtime harness to create a real persisted TaskApp
   snapshot at an isolated state path;
6. launch both extracted apps from `/` without `MOSAIC_APP_LIBRARY`; and
7. use the replacement app's runtime to restore the same persisted snapshot.

The exact release gate also seeds the committed v0.1.0 fixture at the standard
Application Support path, launches the extracted app without relocating it,
checks the sentinel task through the replacement runtime, and proves invalid
bytes are preserved under the `.corrupt` sibling.

Signing, notarization, universal binaries, DMG packaging, and automatic updates
require explicit future release engineering and credentials. None is implied by
this v1 ZIP.
