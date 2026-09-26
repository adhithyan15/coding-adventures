# UI89 — Mobile targets: iOS, iPadOS and Android

**Status:** proposed (2026-09-25). Owner direction: add iOS, iPadOS and
Android and start building them, so every Mosaic app (Journal, Engram,
Trestle, Venture) ships on phones and tablets from the same sources.

**Builds on:**
- [UI38 — native application runtime](UI38-mosaic-native-application-runtime.md),
  which kept iOS as a compile-only gate "rather than packaging a macOS dylib";
- [UI32 — project shells](UI32-cross-backend-project-shells.md), which put
  iOS/Android scaffolds out of scope;
- [UI48 — host environment](UI48-host-environment.md) (size classes, back
  navigation; decided, not implemented);
- [UI87](UI87-standard-file-effects.md), whose shared per-OS host libraries
  supply the mobile file pickers.

---

## 1. Why Mosaic doesn't already ship to phones

The UI code is mostly portable. What stops at the desktop is **packaging**:

| | iOS / iPadOS | Android |
|---|---|---|
| UI source | SwiftUI emitter already writes `#if os(iOS)` / UIKit paths; CI compiles it for `generic/platform=iOS` | Compose emitter writes Compose Kotlin, but some output imports `java.awt` and Swing |
| Project | SwiftPM package for macOS; no app target | Compose **Desktop** Gradle project (`kotlin("jvm")`, `Window`, Dmg/Msi/Deb) |
| Rust runtime | `dlopen` of a `.dylib` from a resource bundle, which iOS does not allow | JNA `Native.load` of a desktop `.so`; no Android ABIs built |
| App crates | `crate-type = ["cdylib", "rlib"]` only | same |
| State path | `~/Library/Application Support`, `HOME` | `HOME` / `XDG_DATA_HOME`, which don't exist on Android |
| Host effects | `NSOpenPanel` / `NSSavePanel`; iOS branch answers "not available" | Swing `JFileChooser` |
| CI | one unsigned `xcodebuild` compile | none |

Flutter's loader already knows Android and iOS, but its projects are created
for desktop only; it follows the native targets (§6).

## 2. iOS and iPadOS

### 2.1 The Rust runtime, linked statically

*Implemented 2026-09-25, with two refinements over the first draft:* the app
crates keep `cdylib` + `rlib`, and `code/scripts/build-mosaic-xcframework.sh`
builds the static library with `cargo rustc --crate-type staticlib`, so no
crate changes; and the simulator slice is one fat library (Apple silicon and
Intel, joined with `lipo`), because a generic simulator build links both. The
runtime is selected with `--runtime-library <name>.xcframework`, and the loader
calls it directly under `MOSAIC_RUNTIME_STATIC` (a runtime reached only by
`dlsym` would be dropped from a static archive by the linker).

- ~~App crates gain `staticlib` in `crate-type`.~~ (see above)
- The artifact builder builds `aarch64-apple-ios`, `aarch64-apple-ios-sim`
  and `x86_64-apple-ios` and bundles them as an **`.xcframework`**
  (`xcodebuild -create-xcframework`). `runtime_file_name` accepts `.a` and
  `.xcframework`.
- `CMosaicRuntime.c` resolves symbols from the running process
  (`dlopen(NULL)`), which works when the library is linked in. The explicit
  path and `MOSAIC_APP_LIBRARY` stay for macOS and tests.

### 2.2 An app target, generated

An iOS app needs an Xcode project; SwiftPM cannot produce an `.app`. The
emitter writes `project.pbxproj` itself (an ASCII property list) with one app
target, and `@main` using the SwiftUI `App` lifecycle. No XcodeGen or Tuist
dependency. This is shared Mosaic infrastructure: every package that is built
with a static runtime gets the same project, so Trestle, Journal, Engram and
Venture scale from one generator rather than four hand-made projects.

- **Where:** the generator is its own crate, `mosaic-ios-project` (a pure
  function from a small description to project text, unit-tested without
  Xcode). The artifact builder calls it for the SwiftUI backend whenever
  `--runtime-library` is an `.xcframework`, and writes
  `swiftui/iOS/App.xcodeproj/project.pbxproj` beside the Swift package, which
  still builds as before. The project sits in `iOS/`, not beside `Package.swift`: `xcodebuild` in a
  directory holding an `.xcodeproj` builds that project, so the Swift
  package's own `xcodebuild -scheme App` builds would silently start building
  the app instead. Its `projectDirPath` is `..`, so its paths are the
  package's.
- **What the target compiles:** the same generated files the Swift package
  compiles: `Sources/App/*.swift`, and the C loader
  `Sources/CMosaicRuntime/CMosaicRuntime.c` with `MOSAIC_RUNTIME_STATIC`. It
  links `Runtime/MosaicAppRuntime.xcframework`; Xcode picks the device or
  simulator slice. Swift imports the loader through a module map,
  `Sources/CMosaicRuntime/include/module.modulemap`, which SwiftPM also
  honours, so both builds see one module.
- **Info.plist from build settings** (`GENERATE_INFOPLIST_FILE`), the way
  current Xcode templates do: the display name, a generated launch screen,
  the scene manifest (multiple scenes, so iPadOS can open several windows),
  and every orientation. Nothing to keep in sync by hand.
- **One target for iPhone and iPad:** `TARGETED_DEVICE_FAMILY = "1,2"`.
  iPadOS multitasking and `NavigationSplitView` come from SwiftUI.
- **Identity:** the bundle identifier defaults to
  `dev.codingadventures.<package name>` (letters and digits only), and the
  version comes from the package manifest. Signing is left to Xcode's
  automatic style (§6: distribution is out of scope); CI builds with
  `CODE_SIGNING_ALLOWED=NO` and the simulator runs unsigned builds.
- **Deterministic:** object identifiers are hashes of each object's role and
  path, so regenerating produces an identical file.
- **Not yet:** an app icon. The asset catalog arrives with package-level icon
  metadata; until then the system shows its default icon.

### 2.3 Platform behaviour

- State in the app sandbox (`Application Support` from `FileManager`).
- UI48 size classes: compact width collapses `HostNavigationSplit` to a stack,
  as `NavigationSplitView` does natively.
- Host effects from the shared per-OS library (UI87 A):
  `UIDocumentPickerViewController` for `files.open` / `files.save`,
  `PHPickerViewController` for images.

## 3. Android

### 3.1 Project

A second Compose project shape, `--target android`: Android Gradle Plugin
(`com.android.application`), one `ComponentActivity` calling
`setContent { MosaicApp(...) }`, a manifest, and launcher icons. The Compose
UI source is shared with desktop; anything desktop-only (`java.awt`, Swing
file choosers, `Window`) moves behind a per-platform seam so the Android
build does not see it.

### 3.2 The Rust runtime

- The artifact builder builds `aarch64-linux-android`, `armv7-linux-androideabi`,
  `x86_64-linux-android` (and `i686` for old emulators) with the NDK, using
  `cargo-ndk`, into `src/main/jniLibs/<abi>/libmosaic_app.so`.
- Loading stays JNA (`net.java.dev.jna:jna` has an Android `aar`), so the
  binding code is shared with desktop. JNI via `System.loadLibrary` is the
  fallback if JNA's size or startup cost is a problem.

### 3.3 Platform behaviour

- State in `Context.filesDir`.
- Back handling via `OnBackPressedDispatcher` (UI48).
- Host effects: `ActivityResultContracts.OpenDocument` / `CreateDocument`, the
  photo picker.

## 4. CI

| lane | builds | drives |
|---|---|---|
| iOS | Trestle, Journal: xcframework + app for the simulator, unsigned | an XCUITest launch-and-restore test on an iOS simulator (the JournalUiTest shape) |
| Android | Trestle, Journal: debug APK, all ABIs | an instrumented Compose test on an x86_64 emulator |

Signing, store packaging (`.ipa`, `.aab`) and release workflows come after the
lanes are green, as their own PRs.

## 5. Order

1. **iOS runtime:** `staticlib` + xcframework in the artifact builder;
   `CMosaicRuntime` resolves from the process. Gate: TaskApp's iOS compile
   now links a real runtime.
2. **iOS app target:** generated `project.pbxproj` for every package with a
   static runtime; TaskApp builds and launches on the simulator in CI.
3. **iOS behaviour:** sandbox state path, restore test, iPad size classes.
4. **Android project:** `--target android` Gradle project and the desktop-only
   seam; Trestle builds an APK.
5. **Android runtime:** `cargo-ndk` ABIs, JNA on Android, `filesDir`; the
   emulator test.
6. **Mobile host effects** through UI87's shared libraries.
7. **Every app:** Journal, Engram, Venture (after BR02's host work).
8. **Flutter:** `flutter create --platforms=android,ios`, per-ABI native
   assets, `path_provider` for state.

iOS goes first because the emitted source already compiles for it; the gap is
packaging only.

## 6. What this does not decide

- App Store / Play Store distribution, signing identities, and store assets.
- Background execution, push notifications, widgets.
- Whether Venture's paint surface uses Metal on iOS (likely) or a shared
  software path; BR02 P10 decides.
