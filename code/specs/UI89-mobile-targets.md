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
target: the generated Swift sources, the xcframework, an `Info.plist`, an
asset catalog with the app icon, and `@main` using the SwiftUI `App`
lifecycle. No XcodeGen or Tuist dependency. iPad is the same target
(`TARGETED_DEVICE_FAMILY = "1,2"`); iPadOS multitasking and
`NavigationSplitView` come from SwiftUI.

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

### 3.4 Android, designed

Written before implementation, from what the Compose backend emits today
(checked on `main` with Trestle). Four things in that output are desktop-only
and are the seams to cut:

| today | desktop-only because | seam |
|---|---|---|
| `Main.kt` holds both `fun main() = application { Window(…) }` and the shared `MosaicApp(host)` composable | `application`/`Window` exist only on desktop | split: `MosaicAppShell.kt` (shared: `MosaicApp`, `MosaicComposeHost`, the prop helpers) and `Main.kt` (desktop `main`) |
| components with drag and drop read `event.awtTransferable` | AWT | the component calls two platform functions, `mosaicDragText(event)` and `mosaicDragTransfer(text)`, defined in `MosaicPlatform.kt`, one per platform (desktop: AWT; Android: `ClipData`) |
| `MosaicPlatformEffects.kt` opens `java.awt.FileDialog` | AWT | desktop-only file; Android gets its own library later (§3.3, step 6) and installs nothing until then |
| `MosaicRuntimeHost` finds its state under `user.home`, its library through `compose.application.resources.dir` | JVM desktop properties | `MosaicRuntimeHost.load(stateDirectory = …)`: the Android activity passes `filesDir`; JNA loads `libmosaic_app.so` from `jniLibs` by name |

Each seam lands first with desktop output unchanged, byte-for-byte except for
the moved code (PR A). Then:

- **The project.** A Compose build with `--emit-project` also writes
  `compose/android/`, a second Gradle project beside the desktop one (the same
  arrangement as `swiftui/iOS/`): `settings.gradle.kts`, `build.gradle.kts`
  with `com.android.application`, Kotlin Android and the Compose compiler
  plugin, `src/main/AndroidManifest.xml`, and one activity,
  `MosaicActivity`, whose `onCreate` calls `setContent { MosaicApp(host) }`.
  Its source sets name the shared files in `../src/main/kotlin` explicitly and
  never `Main.kt` or `MosaicPlatformEffects.kt`; its own
  `MosaicPlatform.kt` supplies the Android side of each seam. The
  application id and label come from `[app] bundle-identifier` /
  `display-name` (UI32), as on iOS.
- **The runtime.** `code/scripts/build-mosaic-android-libs.sh <cargo package>
  <jniLibs dir>` builds the app crate as a `cdylib` for `arm64-v8a`,
  `armeabi-v7a`, `x86_64` and `x86` with `cargo ndk` into
  `jniLibs/<abi>/libmosaic_app.so`; `--runtime-library <dir>` pointing at such
  a directory installs it into the Android project. JNA comes from its Android
  `aar`.
- **Pinned toolchain:** Android Gradle Plugin and Gradle wrapper versions in
  the generated files, `compileSdk`/`targetSdk` 36, `minSdk` 26, NDK r28
  (28.2.13676358). CI's Ubuntu images carry the SDK and NDK.
- **The gate.** CI builds the debug APK for Trestle with the real runtime,
  checks every ABI's `libmosaic_app.so` exports `mosaic_app_create`, boots an
  x86_64 emulator, installs, launches `MosaicActivity`, and requires the
  process to be alive ten seconds later — the iOS gate's shape.

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
2. **iOS app target:** generated `project.pbxproj`; Trestle builds and
   launches on the simulator in CI.
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
