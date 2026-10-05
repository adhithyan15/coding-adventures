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
  *As built:* nothing iOS-specific was needed. The Swift host already asks
  `FileManager` for `.applicationSupportDirectory` in the user domain, which
  on iOS is the app container's `Library/Application Support`, and keeps
  `<package name>/mosaic-state.v1.json` there, exactly as on macOS. The host
  writes state only after the first event, so the CI gate seeds it instead of
  driving the UI (step 3):
  1. state the runtime rejects (`{}`) is quarantined to `….corrupt` **inside
     the app's container** — proof the host found and read that path — and the
     app keeps running on a fresh start;
  2. the snapshot the macOS TaskApp run just persisted (the same engine) is
     restored with nothing quarantined, and the app keeps running;
  3. the same app is installed and launched on an iPad simulator too.
  An XCUITest that edits, relaunches and reads the screen stays the goal of
  the §4 lane; this gate needs no UI automation.
- UI48 size classes: compact width collapses `HostNavigationSplit` to a stack,
  as `NavigationSplitView` does natively.
- Host effects from the shared per-OS library (UI87 A):
  `UIDocumentPickerViewController` for `files.open` / `files.save`,
  `PHPickerViewController` for images.

### 2.4 Journal on iOS and iPadOS (step 7)

Written before implementation. Journal joins Trestle on the iPhone and iPad
simulators. Nothing in Journal changes.

- **The runtime.** `build-mosaic-xcframework.sh journal-mosaic-app` builds
  the engine as an `.xcframework` of static libraries, as for Trestle (§2.1).
  Journal's SwiftUI project is built native-complete with it, and the
  generated `iOS/App.xcodeproj` (§2.2) builds the simulator `.app`.
- **Identity.** The defaults (§3.9 gives the reason): bundle identifier
  `dev.codingadventures.journalapp` and display name `JournalApp`.
- **The gate: three launches, as on Android (§3.7), not a seeded first
  launch.** Since UI48 ENV4 the SwiftUI shell reports the window's
  environment once it lays out. That report is an event, and the host
  persists after it, so a fresh launch writes state with no finger and no
  seed. `code/scripts/mosaic-ios-simulator-gate.sh <app> <bundle id>
  <mosaic application id>` installs the app on a booted simulator and
  launches it three times. Each launch must leave the app running ten
  seconds later:
  1. fresh: `Library/Application Support/journal-app/mosaic-state.v1.json`
     appears in the app's data container;
  2. again: that state is restored, with nothing quarantined;
  3. seeded with `{}`, which the runtime refuses: the seed is moved to
     `….corrupt` holding `{}`, and fresh state is written.
  The app restores its own state, not a macOS snapshot. The macOS lane only
  builds Journal; it never runs it, so there is no snapshot to take.
  Trestle's gate stays as it is. The same `.app` then installs and runs on
  an iPad simulator.
- **Lane selection.** The Swift runtime lane, like the Compose one, now
  reruns when a script only it calls changes (`CI_SCRIPT_PATHS`):
  `build-mosaic-xcframework.sh` and the new gate.

### 2.5 Engram on iOS and iPadOS (step 7)

Written before implementation. Engram follows Journal (§2.4) through the
same recipe and the same gate. Two points are Engram's own:

- **Its effect handler is compiled for iOS.** `[host_effects]` installs
  `Sources/App/EngramEffects.swift`, and the iOS app target compiles every
  file under `Sources/App` (§2.2), so the handler is in the app. Its AppKit
  panels are under `#if os(macOS)`. On iOS the `#else` branch answers
  `importAnki` and `exportAnki` with `failed { message: "file dialogs are not
  available on this platform" }`, so the app reports it instead of waiting.
  This PR is the first to compile that branch. *Since §2.6, that branch
  answers both kinds through the platform library's document picker instead;
  Android's follows.*
- **Its touch layout.** `EngramApp.touch.swift` is compiled too, and the
  layout rules choose it on a phone-sized window, as on Android (§3.10).

The rest is Journal's:

- `build-mosaic-xcframework.sh engram-mosaic-app`, whose dependencies are
  pure Rust;
- the native-complete project with no degradations, and the generated iOS
  app target;
- `_mosaic_app_create` linked into the app;
- the default identity, `dev.codingadventures.engramapp` and `EngramApp`;
- `mosaic-ios-simulator-gate.sh` with Mosaic application id `engram-app`,
  then a launch on the iPad simulator.

The macOS lane's step timeout goes from 45 to 60 minutes. It took about 25
minutes with Journal, and Engram adds a second engine and app build.

### 2.6 App effects through the platform picker (iOS)

Written before implementation. On iOS a package's `[host_effects]` handler
cannot show a file dialog of its own: AppKit's panels do not exist, and
presenting a `UIDocumentPickerViewController` needs a view controller and the
one-request-at-a-time rule that only the platform library has (§3.8). Engram's
handler has therefore failed `importAnki` and `exportAnki` on iOS (§2.5). The
platform library now lends its picker to the app's handler.

- **Finding the router.** `installMosaicPlatformEffects` already keeps a
  table of the hosts it has routed. That table now holds each host's router,
  weakly: the host's handler closure keeps the router alive for exactly the
  host's lifetime. `mosaicPlatformRouter(for: host)` returns it, or nil when
  the library was never installed on that host. By the time an app kind
  reaches the app's handler, the router that passed it on exists.
- **Open.** `router.openForApp(id, accept:, limit:, ok:)` answers the app's
  Await `id` through the picker. It follows the rules `files.open` follows:
  - one file operation at a time (a second request fails at once);
  - the effect is deferred first, and nothing is shown for an id the runtime
    is not awaiting;
  - the document is read on the background queue, bounded while it is read;
  - `cancelled {}` on a cancel, and `failed` with a fixed message otherwise.

  Three things are the app's own:
  - the accepted extensions (`MosaicAccept`, not a MIME list, since an app
    type such as `.apkg` is in no MIME table);
  - the size limit, which the app sets for its own data;
  - the `ok` answer, built by `ok(name, bytes)` from the bytes read. `ok`
    runs on the background queue on iOS, so it only builds the answer.
- **Save.** `router.saveForApp(id, suggestedName:, bytes:, accept:, ok:)`
  checks the name exactly as `files.save` does, by the same function, so a
  refused name fails before anything is shown. Extensions are compared without
  case. The app chooses its own extensions here rather than taking them from
  the MIME table, so an executable extension is refused whatever the app
  accepts. Then the router asks the picker and answers with `ok(name)`. The bytes come from the app's own payload, so there
  is no `files.save` 16 MiB limit: the app's runtime already holds them.
- **Shared, not copied.** The standard `files.open` and `files.save` paths are
  rebuilt on the same two operations with the library's own accept list,
  limits and `ok` shapes, so there is one implementation of each.
- **Engram on iOS.**
  - `importAnki` opens with `.apkg` and `.colpkg` and Engram's 256 MiB
    limit, and answers `ok { apkg: <base64> }`.
  - `exportAnki` validates its package as on macOS (strict base64, a zip
    local header). It saves as `suggestedName` with `.apkg` added when
    missing, accepting `.apkg` only. A name the library would refuse falls
    back to `engram.apkg`, since it is only a suggestion and the macOS panel
    would simply show it for editing. It answers `ok {}`.
  - *Known limit:* a 256 MiB import is held several times in flight (the
    bytes, their base64, the JSON envelope, the runtime's copy), which is
    close to what iOS allows a foreground app on 2–3 GB devices. The limit
    stays the engine's own; a lower iOS limit, or a streamed import, would be
    its own change.
  - The macOS panels are unchanged. A host without the library (none today)
    keeps the old "not available" failure.
- **Gates.** The Linux and macOS Swift harness drives both operations with
  fake pickers:
  - an open answered later, mapped through `ok`;
  - a read over the app's limit;
  - an extension filter passed to the picker;
  - a refused save name, with no picker shown;
  - a second request refused while one is open;
  - lookup of an uninstalled host.
  The iOS simulator build compiles Engram's new branch; driving the picker
  itself waits for §4's XCUITest. Android follows with the Kotlin library.

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
| components with drag and drop read `event.awtTransferable` | AWT | the component calls platform functions -- `mosaicDragText(event)`, `mosaicDragPosition(event)`, `mosaicDragTransfer(text, onCompleted)` and `mosaicDragEnded(event)` -- defined in `MosaicPlatform.kt`, one per platform (desktop: AWT; Android: `ClipData`, see §3.5) |
| `MosaicPlatformEffects.kt` opens `java.awt.FileDialog` | AWT | desktop-only file; Android has its own `MosaicPlatformEffects.kt`, the document picker, beside the shared `MosaicFileEffects.kt` (§3.8) |
| `MosaicRuntimeHost` finds its state under `user.home`, its library through `compose.application.resources.dir` | JVM desktop properties | `MosaicRuntimeHost.load(stateDirectory = …)`: the Android activity passes `filesDir`; JNA loads `libmosaic_app.so` from `jniLibs` by name |

Each seam lands first with desktop output unchanged, byte-for-byte except for
the moved code (PR A). Then:

- **The project.** A Compose build with `--emit-project` also writes
  `compose/android/`, a second Gradle project beside the desktop one (the same
  arrangement as `swiftui/iOS/`): `settings.gradle.kts`, `build.gradle.kts`
  with `com.android.application`, Kotlin Android and the Compose compiler
  plugin, `src/main/AndroidManifest.xml`, and one activity,
  `MosaicActivity`, whose `onCreate` calls `setContent { MosaicApp(host) }`.
  Its source set holds copies of the shared files (§3.5), never `Main.kt` or
  `MosaicPlatformEffects.kt`; its own `MosaicPlatform.kt` supplies the
  Android side of each seam. The
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

### 3.5 Android, as built (step 4)

What step 4 changed from §3.4, and why:

- **Copies, not references.** An Android source set takes directories and
  cannot leave out the desktop-only files that sit beside the shared ones in
  `src/main/kotlin`, so the builder copies the shared ones into
  `android/src/main/kotlin`: `MosaicAppShell.kt`, `MosaicRuntimeHost.kt`, and
  every exported component with its layout variants. It writes the Android
  project last, after `[host_assets]` and `[host_effects]` are installed (the
  iOS project's rule), so a replaced shared file reaches Android too. The
  desktop `compose` effect handlers stay on the desktop; a package's Android
  handler is its own `compose-android` entry (§3.12), and the standard file
  effects reach Android through its own platform library (§3.8).
- **The activity lives in a package.** A manifest cannot name a class in the
  root package, and the shared sources live there. `MosaicActivity` is
  `mosaic.android.MosaicActivity`, and imports the root-package shell, which
  Kotlin (unlike Java) allows. It sets `MosaicRuntimeHost.stateDirectory` to
  `filesDir` before loading the host, and handles its own configuration
  changes, so a rotation re-lays out the tree instead of destroying the host.
- **Identity.** The application id is `[app] bundle-identifier` (or the iOS
  default) made legal for Android: `-` becomes `_`, a part that does not start
  with a letter gets an `x`, and a Java keyword part gets a trailing `_`. The
  label is `display-name` (or the root component) in `res/values/strings.xml`,
  escaped for Android's string syntax (a leading `@` or `?`, after any
  blanks, included) and XML, with `formatted="false"`; a literal in the manifest would
  read a leading `@` or `?` as a reference. Cloud backup is off, and the app asks for
  no permissions.
- **Drag and drop on Android.** Android hands a drop target the `clipData`
  only with the drop, and never tells the source its drag ended. The Android
  half therefore also puts the text in `localState` (every event of an in-app
  drag carries it), so a target accepts, enters and hovers as on desktop; and
  a fourth seam, `mosaicDragEnded(event)`, which every target's `onEnded`
  calls, runs the source's completion once (desktop: nothing to do -- AWT
  reports the end itself). Android tells a drag's end only to the targets
  that were interested in it, so a drag no component wanted -- let go over
  empty space, or over targets of another kind -- would never reach its
  source, which would stay mid-drag. `MosaicActivity` therefore wraps the
  app in `MosaicDragEndWatcher` (Android's `MosaicPlatform.kt`): one target
  over the whole window, interested in every Mosaic drag (its `localState`
  is Mosaic's), that accepts no drop and whose `onEnded` calls
  `mosaicDragEnded`. The components' own targets still decide every drop;
  the completion's once-only guard absorbs the second report when one of
  them was interested too. The wrapper passes the window's constraints
  through unchanged (`propagateMinConstraints`), so layout is as before.
- **Dependencies.** Compose through the same JetBrains coordinates the desktop
  build resolves (each is an `androidx.compose` artifact on Android), so both
  compile against one API; JNA from its `aar`. Pinned: Android Gradle Plugin
  8.13.0, Gradle 8.14.3 (named in `gradle-wrapper.properties`; the wrapper's
  jar and scripts are not generated), Kotlin 2.3.21, compile/target SDK 36,
  min SDK 26 (where `java.nio.file` and `java.util.Base64`, which the runtime
  host uses, arrive). The NDK pin belongs to step 5's runtime script.
- **The gate.** CI builds TaskApp's debug APK and checks its package name,
  label, launchable activity and SDK levels (`aapt2 dump badging`), that the
  shared sources, the activity and the Android seams are in the dex, and that
  JNA's `libjnidispatch.so` is there for x86_64. Without the runtime (step 5)
  a native-complete app shows its startup failure screen; the emulator run
  belongs to step 5.

### 3.6 The Android runtime, as built (step 5, first half)

- **Built per ABI.** `code/scripts/build-mosaic-android-libs.sh <cargo package>
  <jniLibs dir> [--release]` runs `cargo ndk` for `arm64-v8a`, `armeabi-v7a`,
  `x86_64` and `x86` at API level 26 (the project's minimum SDK) and writes
  `<dir>/<abi>/libmosaic_app.so`: renamed, because the shared runtime host
  loads `mosaic_app` by name through JNA on Android as on desktop. Debug by
  default for CI; `--release` for a build to ship.
- **Selected by the path's shape.** For the Compose backend a
  `--runtime-library` that is a directory (and not an `.xcframework`) is that
  set of libraries, as an `.xcframework` directory selects iOS for SwiftUI. It
  is checked strictly, because it is copied into an APK that runs it: only the
  four ABI directories, each a real directory holding exactly one regular
  `libmosaic_app.so`, nothing followed through a link, at least one ABI. It is
  installed into `android/src/main/jniLibs/<abi>/`, where Gradle packages it
  and JNA finds it. The desktop project bundles nothing from it, and the
  Android activity starts strictly (`MosaicStartup`), because a runtime was
  selected.
- **The gate.** CI builds TaskApp's engine for the four ABIs, packages it,
  and requires the APK to hold `lib/<abi>/libmosaic_app.so` for each, each
  exporting `mosaic_app_create` (`llvm-nm` from the NDK). The emulator launch
  and restore test is the second half of step 5 (§3.7).

### 3.7 The Android emulator gate, as built (step 5, second half)

- **The emulator.** `code/scripts/start-mosaic-android-emulator.sh` boots
  `system-images;android-34;default;x86_64` (plain AOSP, no Google services
  to sign in to, the ABI KVM runs natively; API level, variant and ABI are
  pinned, the revision is the SDK repository's and is logged) headless, cold
  and software-rendered, and waits for `sys.boot_completed`. It refuses to start
  without `/dev/kvm`; CI opens it to the runner's user first.
- **No UI automation, as on iOS (§2.3).** The host persists after every event,
  and the first event needs no finger: the activity reports the window's
  environment (UI48 ENV4) once it lays out. So a state file appearing at
  `filesDir/<application id>/mosaic-state.v1.json` proves at once that the
  activity started, JNA loaded `libmosaic_app.so`, the engine answered, and
  the host persisted where `MosaicActivity` pointed it. This is stronger than
  "still running": a native-complete app that fails to start shows its
  failure screen and keeps running.
- **Three launches.** `code/scripts/mosaic-android-emulator-gate.sh <apk>
  <android package> <application id>` installs the debug APK (`run-as`, which
  reads and seeds the app's private files, needs a debuggable app) and
  launches `MosaicActivity` three times. Each must still be running ten
  seconds later with no `FATAL EXCEPTION` in its logcat:
  1. fresh: state is written;
  2. again: that state is restored -- nothing quarantined, and no "rejected
     persisted state" on `System.err`;
  3. seeded with `{}`, which the runtime refuses: it is moved to
     `mosaic-state.v1.json.corrupt` inside the app's storage, and fresh state
     is written.
  Unlike the iOS gate, which restores the macOS run's snapshot, the app
  restores its own: the Linux job has no desktop TaskApp snapshot to hand,
  and one written by the same APK is the case a user meets.
- **Still to come:** an instrumented Compose test that edits, relaunches and
  reads the screen (§4). Journal runs the same gate (§3.9).

### 3.8 Mobile file effects, designed (step 6)

Written before implementation. `files.open` and `files.save` (UI59, UI87) on
iOS/iPadOS and Android, through the same platform library each backend
already has: SwiftUI's `MosaicPlatformEffects.swift` on iOS, and an Android
library for Compose. Flutter on mobile waits for step 8, which builds Flutter
for phones at all; until then it keeps failing each request with a message
(UI87 §7.7). The contracts, limits and name rules (UI87 §3.1) do not change:
a mobile save refuses exactly the names a desktop save refuses, and an open
reads at most 50 MiB.

**Why the desktop seam does not carry over.** On desktop a dialog is modal
and synchronous: `MosaicFileDialogs.chooseFileToOpen` returns the chosen file
and the library reads or writes it in place. A phone's pickers are neither:

| | iOS / iPadOS | Android |
|---|---|---|
| picker | `UIDocumentPickerViewController`, presented; answers through a delegate | the Storage Access Framework through `ActivityResultContracts.OpenDocument` / `CreateDocument`; answers through the activity's result registry |
| what comes back | a file URL (with `asCopy`, a copy in the app's own temporary directory) | a `content://` `Uri`, read and written through `ContentResolver` streams, never a path |
| who confirms a replace | the picker | the provider (most append ` (1)` to a taken name rather than replace) |

So each library gains an asynchronous path beside the synchronous one, and
the part that is the same everywhere -- the name rules, the MIME table, the
base64 and size limits, the `ok` / `cancelled` / `failed` answers -- moves
where both paths share it.

**The shared core, testable without a phone.**

- *Kotlin.* The platform-independent half of today's `MosaicPlatformEffects.kt`
  (routing by kind, the MIME table, `mosaicIsPlainFileName`,
  `mosaicHasExecutableExtension`, the device names, the payload checks and
  limits) moves to `MosaicFileEffects.kt`, with the router and the
  asynchronous path below; it imports nothing from AWT or Android. The desktop project gets both files; desktop output is otherwise
  unchanged. The Android project gets `MosaicFileEffects.kt` and its own
  `MosaicPlatformEffects.kt` (below), and still never the desktop one.
- *The asynchronous router.* `MosaicPlatformRouter` takes a picker that
  answers later: `MosaicDocumentPicker` with `open(accept, done)` and
  `create(request, done)`, each `done` called exactly once
  with a document or nothing (a cancel). `accept` carries the request's
  known MIME types and their extensions; `create` gets the whole checked
  save request, bytes included, because iOS's picker exports a file it is
  handed and so needs them before it is shown. A document is a name plus a way to
  read or write its bytes as streams. The router defers the effect, as on
  desktop, keeps the one-request-at-a-time rule, and answers from `done`:
  reading (bounded, as on desktop) or writing off the main thread, then
  completing the effect. The synchronous desktop dialogs are adapted to this
  shape, so there is one router per language, and the desktop conformance
  harness exercises it with fakes on the JVM and on Linux Swift, as it does
  today.
- *Swift.* The same split inside `MosaicPlatformEffects.swift`: the router and
  the rules compile on every OS, the AppKit panels under `#if os(macOS)`, the
  UIKit picker under `#if os(iOS)`. The Linux harness keeps compiling the
  file, so the router's new path is checked there.

**iOS / iPadOS.**

- *Open.* `UIDocumentPickerViewController(forOpeningContentTypes:asCopy: true)`,
  the content types from the request's accepted types (`UTType` of each of
  their extensions; `.item`, any document, when none map).
  With `asCopy` the system copies the chosen file into the app's temporary
  directory, so no security-scoped access is held and nothing outside the
  app's container is touched after the picker closes. The copy is read with
  the desktop limits and removed whether or not the read succeeded. The name
  is the copy's last path component (the original's name).
- *Save.* The request is checked first, exactly as on desktop; a refused
  name, size or extension fails before anything is shown. The bytes are then
  written as `<suggestedName>` into a fresh private directory under the app's
  temporary directory, and `UIDocumentPickerViewController(forExporting:
  asCopy: true)` hands that file to the person, who chooses where it goes;
  the picker confirms any replace. The answer is `ok { name }` with the name
  the picker reports, `cancelled {}` on a cancel; the private directory is
  removed either way.
- *Where it is shown.* From the foreground window scene's key window (an
  active scene first, else one that is only momentarily inactive), on its
  topmost presented view controller. No such window (the app is in the
  background) fails the request with a message rather than waiting; a host
  that closed while the request was queued shows nothing.
- *Lifetime.* The picker is its own delegate and is held by the view
  controller presenting it. A picker that goes away without either delegate
  callback (a scene destroyed under it) answers `cancelled {}` from its
  `deinit` -- after any delegate call, since UIKit passes the picker to the
  delegate -- so a deferred effect is never left pending. A presentation
  UIKit refuses fails the request at once.
- *Off the main queue.* The copy is read, and an export's answer handled,
  on a background queue; the outcome comes back to the main queue before the
  effect is completed, so the host is touched only where it always was.
- `mosaicPlatformHasFileDialogs` becomes true on iOS; the "not available on
  this platform yet" failure remains for any other OS without a picker.

**Android.**

- *The library.* `android/src/main/kotlin/MosaicPlatformEffects.kt`, from a
  new `mosaic-app-bindings` template, implements `MosaicDocumentPicker` on
  the activity's `ActivityResultRegistry`. It registers its two launchers
  when the activity is created (before it starts, as the registry requires)
  and unregisters them when it is destroyed.
- *Open.* `OpenDocument` with the request's MIME types (`*/*` when none
  map). The name is the provider's `OpenableColumns.DISPLAY_NAME`, the type
  `ContentResolver.getType` (else the MIME table's guess, as on desktop); the
  bytes are read from `openInputStream` on a background thread, with the
  desktop limit enforced while reading, not trusted from the provider's size.
- *Save.* Checked first, as on desktop. Then `ACTION_CREATE_DOCUMENT` with
  the suggested name and its type (the name's own type when the request
  accepts it or accepts anything, else the first accepted type); the bytes
  are written with `openOutputStream(uri, "wt")` on a background thread. The
  answer's name is the provider's display name for the document, which may
  differ from the suggestion (` (1)`). A write that fails fails the request
  and touches nothing else: the picker may have handed back a document the
  person already had (DocumentsUI asks "replace?"), and deleting it would
  lose its history along with the bytes. There is no temporary-and-rename
  here: the provider owns the file, and SAF offers no atomic replace.
- *Install.* `MosaicActivity` installs the library when the host loads, as
  the desktop `Main.kt` does: `installMosaicPlatformEffects(host, picker)`,
  with the picker it built in `onCreate`, returning the router. Without a
  `compose-android` handler (§3.12), the standard kinds always go to the
  library, and any other kind is failed by the host as unanswered.
- *The activity goes away.* `MosaicActivity` already handles rotation and
  size changes itself. If it is destroyed anyway while a picker is open (the
  system reclaims it), the router answers the waiting effect `failed` with a
  message from `onDestroy`, so the runtime is never left awaiting a result
  that can no longer arrive; a result delivered later is dropped.

**Gates.**

- The desktop harnesses (Compose on the JVM, Swift on Linux and macOS) drive
  the asynchronous router with fake pickers: a document answered later, a
  cancel, `done` never called twice, a second request refused while one is
  open, a read over the limit, a failed write reported with a fixed
  message, and a save refused before any picker is shown.
- CI's existing mobile lanes compile the new code: the iOS simulator and
  device builds (§2.3) and the Android APK (§3.5), whose dex must hold the
  Android library. Driving the system pickers themselves needs the UI
  automation of §4 (an XCUITest, an instrumented Compose test), which lands
  with those lanes.

**As built: the Kotlin shared core (first PR).** `MosaicFileEffects.kt`
(`mosaic-app-bindings`' `compose_file_effects()`) holds the rules, the MIME
table, routing, `mosaicCheckSaveRequest`, the picker seam
(`MosaicDocumentPicker`, `MosaicOpenedDocument`, `MosaicSaveTarget`,
`MosaicAccept`, `MosaicFileFailure`), `mosaicAnswerFilesOpen` /
`mosaicAnswerFilesSave` (each answers exactly once, never throws, and hears a
picker's first answer only) and the router, which now takes a picker and an
optional `runInBackground` -- null (the desktop) reads and writes inline, as
before; otherwise the outcome is handed back through `runOnUi` before the
effect is completed -- and gains `failPending(message)` for a picker whose
answer can no longer arrive. The router is typed on
`MosaicPlatformEffectHost`, an interface `MosaicRuntimeHost.kt` now declares
and `MosaicRuntimeHost` implements, so the harness drives it with a fake
host as the Swift harness does. The desktop
`MosaicPlatformEffects.kt` keeps the AWT dialogs and writing in place,
adapted by `MosaicDialogPicker`; `mosaicRunFilesOpen` / `mosaicRunFilesSave`
and `installMosaicPlatformEffects` keep their signatures and outcomes, so
`Main.kt` is unchanged. The builder writes both files into every desktop
project and reserves the new public type names (UI48 §7.5); Android gets the
shared file with its own picker in the third PR. Nothing thrown on the
background thread escapes it (an `OutOfMemoryError` encoding a large file is
still an answer); an answer the host cannot take is answered again with a
small failure ("couldn't deliver the file"), so a deferred effect is never
left awaited; and a stream that only ever returns nothing fails the read
after 1000 such reads instead of holding the one file operation open. The
Compose harness gained thirteen checks of the asynchronous path and the
router (26 in all).

**As built: Swift and iOS (second PR).** `MosaicPlatformEffects.swift`
gained the same seam as Kotlin (`MosaicDocumentPicker`,
`MosaicOpenedDocument`, `MosaicSaveTarget`, `MosaicSaveRequest`,
`MosaicAccept`, `MosaicFileFailure`, `mosaicCheckSaveRequest`,
`mosaicAnswerFilesOpen` / `mosaicAnswerFilesSave`), the macOS panels
adapted by `MosaicDialogPicker`, and, under `#if os(iOS)`,
`MosaicUIKitDocumentPicker` with its `MosaicPickerController`. The router
takes a picker and an optional `runInBackground`: nil on macOS (inline, as
before), a global queue on iOS, whose outcome is handed back through
`runOnUI`. `installMosaicPlatformEffects` keeps its parameters and adds
`picker:` and `runInBackground:`; the generated `App.swift` is unchanged. The
table of hosts already routed is a lock-guarded list of weak references
compared by identity, not `NSHashTable`, which Linux's Foundation lacks, so
the Linux harness compiles the library exactly as generated. A Rust test
fences AppKit to `#if os(macOS)` and UIKit to `#if os(iOS)`. The
Linux and macOS harness drives the asynchronous path and the background
hand-off through the router with fakes; the iOS picker itself is compiled by
CI's iOS simulator and device builds and driven by nothing until §4's
XCUITest.

**As built: Android (third PR).** `mosaic-app-bindings`'
`compose_android_platform_effects()` is written as
`android/src/main/kotlin/MosaicPlatformEffects.kt`, and `MosaicFileEffects.kt`
joins the shared sources copied from the desktop project. The library is
`MosaicAndroidDocumentPicker` (two launchers: `OpenDocument`, and
`MosaicCreateDocument`, a contract that takes the type and title per request,
since `CreateDocument` fixes its type when registered) and an
`installMosaicPlatformEffects(host, picker)` that returns the router. A save
asks for the suggested name's own type when the request accepts it (or
accepts anything), else the first type it accepts. A provider's display
name reaches the app only when it is an ordinary name -- after any `/` or
`\`, not `.` or `..`, no control or separator characters, lone surrogates or
bidirectional controls, at most 255 UTF-16 units (the zero-width joiners of
ordinary writing and emoji pass) -- and is "document" otherwise; its MIME type is used only
when shaped like one, each part at most 127 characters. Both are asked of
the provider on the background thread. A failed write is reported, never
cleaned up by deleting the document, which may be one the person already
had. A launch that throws anything leaves the picker not waiting.

*A stalled provider fails the request instead of holding it.* A provider
serves its document through a pipe it fills as it goes -- a cloud
provider, downloading -- and one that stops filling it would block the read
for ever. So would a provider that never returns from opening the document,
or a reader at the far end of a save that stops taking bytes. Each would hold
the one background thread, and with it the one file operation, until the
process ended. A watch now bounds every Android read and write:

- `MosaicStallWatch` (shared, in `MosaicFileEffects.kt`) notes each time
  bytes move, and when none has moved for `MOSAIC_STALL_MILLIS` (60 seconds)
  stops the transfer and remembers that it did. Progress, not total time,
  is what it measures: a slow file that keeps arriving is never cut off.
- Opening asks the provider through `openAssetFileDescriptor(uri, mode,
  signal)`; until the stream exists, stopping cancels that
  `CancellationSignal`, which a `DocumentsProvider` receives in
  `openDocument`.
- Once the descriptor exists, stopping closes it from the watchdog's thread,
  with `closeWithError`: a provider reading a save through a reliable pipe
  learns the save failed, rather than taking a truncated one for finished.
  Android's file streams wake a thread blocked on a descriptor another thread
  closed, so the blocked `read` or `write` throws. The descriptor is closed
  exactly once, by the watch or by the transfer, whichever comes first, since
  two threads closing one descriptor could close a reused fd number. A
  transfer that returns after the watch fired is failed all the same.
- The display-name query takes a `CancellationSignal` too and is watched the
  same way; a provider that never answers it leaves the name "document".
  `getType` and the binder call that sets up a cancellation take no signal
  and are not watched.
- The bytes of a save are written in 64 KiB pieces, so a slow save that is
  moving keeps the watch fed.
- A transfer the watch stopped fails with a fixed message ("the selected file
  stopped arriving", "the file stopped saving"), and the router answers the
  request, freeing the file operation for the next one. The answer waits for
  the blocked call to return, which the close forces on a stream; an
  `openDocument` that ignores its cancellation still holds the thread until
  it returns.

The desktop reads and writes local files, which do not stall this way, and
does not use the watch.
`MosaicActivity` constructs the picker in `onCreate` before `setContent`,
installs the library as the host loads (inside `MosaicStartup`'s loader, or
the sample's `remember`), and in `onDestroy` fails the request whose picker
is still open. The CI Android build checks the shared and Android files are
in the project and that `MosaicPlatformRouter`, `MosaicFileEffectsKt` and
`MosaicAndroidDocumentPicker` are in the dex; the library and both
activities were type-checked locally against `android.jar` (API 36). The
picker itself is driven by nothing until §4's instrumented test.

**Order.** Three PRs: the Kotlin shared core and asynchronous router, with
desktop behaviour unchanged; the Swift router's asynchronous path with the
iOS picker; the Android library.

**Decided after step 6.**

- *iOS open keeps `asCopy: true`.* The system copies the whole chosen file
  into the app's temporary directory before the picker answers, so a file
  over 50 MiB is copied and then refused. The other choice, `asCopy: false`,
  could refuse an oversized file from its metadata before anything moves,
  but it reads in place: it holds security-scoped access and needs a
  coordinated read (`NSFileCoordinator`) for anything a file provider
  serves. That read downloads a cloud document with no progress shown, no
  cancel, and no way to tell a slow download from one that has stopped, so
  it would need the stall watch Android has (above) with no progress to
  feed it. The system's copy shows progress, offers cancel, and leaves
  nothing behind: the copy is removed once read, or unread. The temporary
  disk and time spent on an oversized copy are the accepted cost.
- *No quarantine mark on saves.* Files saved through the SwiftUI library get
  no `com.apple.quarantine`, by UI87 §3.1 ("No download marks of our own").
  A save is the app writing its own document, and the extension rules keep it
  from being a launcher. A mark the system adds itself, as for a sandboxed
  app, is left in place.

### 3.9 Journal on Android (step 7, first app)

Written before implementation. Journal is the second app on Android, after
Trestle. It runs the same way and passes the same gate. Nothing in Journal
changes:

- **The runtime.** `journal-mosaic-app` is a workspace crate that already
  declares `cdylib`. `build-mosaic-android-libs.sh journal-mosaic-app` builds
  it for the four ABIs, and the Compose build installs it as for Trestle
  (§3.6). The engine keeps no files of its own: the whole journal is the
  runtime's snapshot, which the host persists under `filesDir`. It reads the
  clock through `SystemTime` and the UTC offset from the host's start
  context, and both work on Android unchanged.
- **Identity.** Journal's manifest names no `bundle-identifier` or
  `display-name`, so Android takes the defaults the Apple builds already use.
  The application id is `dev.codingadventures.journalapp` and the label is
  `JournalApp`. A product name and identifier would change every host's
  window title and app identity at once, so they are not chosen here.
- **Effects.** Journal raises none, so the Android file library is present
  but never asked.
- **The gate.** The CI step that builds Trestle's runtime APK builds
  Journal's the same way: native-complete with no degradations; badging
  naming its package, label and `MosaicActivity`; and `libmosaic_app.so`
  exporting `mosaic_app_create` for each ABI. The emulator step then runs
  `mosaic-android-emulator-gate.sh` for Journal after Trestle, on the same
  booted emulator, with Mosaic application id `journal-app`. Its restore
  launch is the one that matters most for Journal, whose entries live only
  in that snapshot.
- **What is already proven.** The desktop Compose lane already compiles
  Journal's output and drives it with `JournalUiTest` (edit, relaunch,
  restore). The APK build compiles the same component and shell sources
  against Android's Compose. The emulator gate adds what only a device
  shows: the per-ABI engine loads through JNA, and the snapshot is kept in
  the app's own storage. Driving Journal's screen on Android is the
  instrumented test of §4.

Engram and Venture follow as their own PRs.

### 3.10 Engram on Android (step 7, second app)

Written before implementation. Engram runs on Android the same way Journal
does (§3.9), with two differences that are Engram's own:

- **Its touch layout.** Engram has a second layout, `EngramApp.touch.mll`,
  chosen by the layout rules (UI48 §7.9). The Android project already gets
  every exported component with its layout variants (§3.5), so a phone-sized
  window selects the touch root exactly as a narrow desktop window does. No
  Android-specific rule is added.
- **Its effects stay desktop-only.** Engram's package `[host_effects]`
  handlers (`importAnki`, `exportAnki`) are installed by each desktop host's
  `Main`. The Android project gets neither the handler file nor the install
  (§3.5). On Android the platform library answers the standard `files.*`
  kinds. An Anki import or export is failed by the host as unanswered, so
  the app reports it and is never left waiting. An Android handler, through
  the document picker of §3.8, is its own later step. (Since done: §3.12.)

Everything else is Journal's:

- **The runtime.** `engram-mosaic-app` and its dependencies are pure Rust;
  the SQLite reader and zstd are this repository's own crates. So
  `build-mosaic-android-libs.sh engram-mosaic-app` needs no C toolchain
  beyond the NDK's.
- **Identity.** The manifest defaults: `dev.codingadventures.engramapp` and
  `EngramApp`.
- **The gate.** The Android CI step builds Engram's APK beside Journal's. It
  checks native-complete with no degradations, the badging, the dex, and
  `mosaic_app_create` for each ABI. The emulator step runs
  `mosaic-android-emulator-gate.sh` for Engram after Journal, with Mosaic
  application id `engram-app`. The seeded `{}` has no snapshot schema, so the
  runtime refuses it before Engram's own `restore` sees it, as for the other
  two apps.

Venture follows after BR02's host work.

### 3.11 App effects through the platform picker (Android)

Written before implementation: the Android counterpart of §2.6, in two PRs.

**The Kotlin seam (first PR).** It is the same contract as Swift's, in the
shared `MosaicFileEffects.kt`, so the desktop Compose library has it too.

- **Finding the router.** The router *is* the host's `effectHandler`
  (`installMosaicPlatformEffects` replaces the app's handler with a
  `MosaicPlatformRouter` that wraps it). So `mosaicPlatformRouter(host)` is
  `host.effectHandler as? MosaicPlatformRouter`, with no table to keep.
- **`openForApp(id, accept, limit, ok)` and `saveForApp(id, suggestedName,
  bytes, accept, ok)`** keep `files.*`'s rules, as in §2.6:
  - one file operation at a time;
  - deferred before anything is shown;
  - slow work on the background thread, handed back to the UI thread before
    the effect is completed;
  - exactly one answer, and `failPending` still reaches a request in flight;
  - the same name checks (`mosaicCheckSaveName`, extensions compared without
    case), and an executable extension refused for an app save whatever the
    app accepts.

  The app supplies the accepted extensions, the read limit and the `ok`
  answer. On Android, reads and writes go through the stall watch as before.
  `ok` runs on the background thread, so it only builds the answer.
- **Robustness (both libraries, after review).**
  - A refusal never throws into the app's handler: the host's own refusal of
    an id it is not awaiting is swallowed, as in Swift.
  - The request in flight, asked for again with the same id, is left to its
    own picker rather than answered "busy". A busy answer would make the
    picker's later answer undeliverable.
  - A throw inside the posted operation still answers the effect and frees
    the router.
  - Kotlin compares extensions the way Swift does: the last extension
    whole, folded.
- **Shared, not copied.** `files.open` and `files.save` are rebuilt on the
  same `mosaicAnswerOpen` and `mosaicAnswerSave`. Their behaviour is
  unchanged.
- **Gate.** The JVM harness (`MosaicPlatformEffectsTest`) drives both
  operations with the fake pickers, mirroring the Swift checks of §2.6.

**Android `[host_effects]` (second PR).** Before it, Android installed no
package handler: the Compose handler is the desktop's, AWT and all (§3.5). A
package may declare handler files and a handler for its own Android target
(`compose-android`, §3.12). The
builder copies those files into `android/src/main/kotlin`, and
`MosaicActivity` installs the handler on the host before the platform
library, passing its `kinds`. Engram gains an Android handler that answers
`importAnki` and `exportAnki` through the seam above, with the rules of
§2.6. That PR specifies the manifest form in its own subsection.

### 3.12 Android `[host_effects]` (§3.11, second PR)

Written before implementation.

- **Manifest form.** `compose-android` is a handler target of its own, so
  nothing in the manifest format changes:

  ```toml
  [host_effects]
  files = [
    { backend = "compose-android", source = "host/android/EngramAndroidEffects.kt",
      target = "src/main/kotlin/EngramAndroidEffects.kt" },
  ]
  handlers = [
    { backend = "compose-android", install = "installEngramAndroidEffects",
      kinds = ["importAnki", "exportAnki"] },
  ]
  ```

  The desktop `compose` handler is untouched, so a package can declare both.
  The manifest already allows one handler per backend name, and requires
  every file to belong to a declared handler.
- **Files.** The Android project writer copies `compose-android` files into
  `compose/android/<target>` after the shared sources, under the same rules
  as `install_host_effects`:
  - paths relative to the package, with no `..` and nothing absolute;
  - the source resolved inside the package and a regular file;
  - the target a Kotlin source (`.kt`) under `src/main/kotlin/`. A handler
    is Kotlin, and elsewhere in the project a file has more reach than a
    handler needs: `buildSrc/` runs at build time,
    `src/debug/AndroidManifest.xml` is merged into the app, and
    `src/main/jniLibs/` is wiped by the runtime copy. (Added after review.)

  A target that would replace any file the Android project already holds
  (`MosaicActivity.kt`, a shared source, a component) is refused, compared
  without case, because the handler would silently replace generated code.
- **Install.** `MosaicActivity` calls the handler's `install(host)` on the
  loaded `MosaicRuntimeHost`, then installs the platform library with the
  handler's `kinds`. Android's `installMosaicPlatformEffects(host, picker,
  appKinds)` gains the `appKinds` parameter. With no handler, nothing
  changes. The activity lives in `mosaic.android`, so a plain install
  name (a root-package function) is imported there, and a dotted one is
  called by its full name. The manifest already restricts the name to an
  identifier path; the builder also refuses:
  - `:` or `::`, which the manifest allows for C++ and C#, and Kotlin
    cannot compile;
  - `include`, as the desktop Compose handler does: Kotlin has no include
    directive, so the field would be silently ignored.
- **Only with an Android project.** `compose-android` entries are used only
  when the Compose build writes `android/` (`--emit-project`). Every other
  backend ignores them, as it ignores another backend's entries.
- **Engram.** `host/android/EngramAndroidEffects.kt` answers `importAnki`
  and `exportAnki` through `mosaicPlatformRouter(host)` and the §3.11 seam,
  with the rules of §2.6:
  - import: `.apkg` / `.colpkg`, 256 MiB, answers `ok { apkg }`;
  - export: strict base64 and a zip local header, `.apkg` only, the
    suggested name with `.apkg` added, falling back to `engram.apkg`. The
    decode runs on the main thread, so base64 longer than a 256 MiB package
    is refused before it, and running out of memory while decoding is
    answered as a failure (added after review).

  Without a router it answers "file dialogs are not available on this
  platform". A refusal never throws.
- **Gates.**
  - Builder tests: the files are copied, the activity installs the handler
    with its kinds before the platform library (strict and sample), and a
    colliding target, a target outside `src/main/kotlin/*.kt`, a source
    outside the package, a directory, `include` and a colon are refused.
  - The Engram Compose CI step greps the generated activity for the install
    line before building the APK.
  - The Engram APK's dex must now hold `EngramAndroidEffectsKt` and still
    not `EngramEffectsKt`.
  - The JVM harness keeps covering the seam. The handler itself is
    type-checked by the APK build. Driving it on a device needs §4's
    instrumented test; until then it was driven once against the real
    router and a fake picker, outside the repository.

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
   seam; Trestle builds an APK. *Done (§3.5): every Compose `--emit-project`
   build writes `android/`; there is no separate target flag.*
5. **Android runtime:** `cargo-ndk` ABIs, JNA on Android, `filesDir`; the
   emulator test. *Done (§3.6, §3.7): the engine is built per ABI and
   packaged, and Trestle launches, restores its state and quarantines refused
   state on an x86_64 emulator.*
6. **Mobile host effects** through UI87's shared libraries. *Done (§3.8):
   Compose on Android and SwiftUI on iOS and iPadOS; Flutter's arrive with
   step 8, which builds Flutter for phones.*
7. **Every app:** Journal, Engram, Venture (after BR02's host work).
   *Journal on Android: §3.9; on iOS and iPadOS: §2.4. Engram on Android:
   §3.10; on iOS and iPadOS: §2.5.*
8. **Flutter:** `flutter create --platforms=android,ios`, per-ABI native
   assets, `path_provider` for state.

iOS goes first because the emitted source already compiles for it; the gap is
packaging only.

## 6. What this does not decide

- App Store / Play Store distribution, signing identities, and store assets.
- Background execution, push notifications, widgets.
- Whether Venture's paint surface uses Metal on iOS (likely) or a shared
  software path; BR02 P10 decides.
