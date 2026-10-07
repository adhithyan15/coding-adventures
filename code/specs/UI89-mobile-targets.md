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
  itself waits for §4.4. Android follows with the Kotlin library.

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
- **Then (§4.2):** an instrumented Compose test that edits, relaunches and
  reads the screen (§4.2). Journal runs the same gate (§3.9).

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
    type-checked by the APK build. Driving it on a device needs §4.4's
    instrumented test; until then it was driven once against the real
    router and a fake picker, outside the repository.

## 4. CI

| lane | builds | drives |
|---|---|---|
| iOS | Trestle, Journal, Engram: xcframework + app for the simulator, unsigned | an XCUITest launch-and-restore test on an iOS simulator (the JournalUiTest shape) |
| Android | Trestle, Journal, Engram: debug APK, all ABIs | an instrumented Compose test on an x86_64 emulator |

Signing, store packaging (`.ipa`, `.aab`) and release workflows come after the
lanes are green, as their own PRs.

### 4.1 Driving the screen on a device, designed

Written before implementation. The emulator and simulator gates (§2.3, §3.7)
prove that an app starts, persists and restores without touching it. They
cannot show that a person can use it: that a field takes text, a button
reaches the engine, and what was typed is on screen again after a cold
relaunch. The desktop Compose lane already proves this for Journal with
`JournalUiTest` (`journal-mosaic-app/conformance/compose/JournalUiTest.kt`).
§4 brings the same test to the phones.

**What the test does (both platforms).** The JournalUiTest shape, in two
cold launches against one state file:

| launch | starts with | does | must see |
|---|---|---|---|
| 1 | no state | waits for `mosaic-startup-loading` to go; types a title and body into `draft-editor-title` / `draft-editor-body`, taps `draft-editor-save`; does it again for a second entry; deletes one with `draft-editor-delete` | "No entries yet" first; then exactly the surviving entry's row |
| 2 | launch 1's state, new process | nothing until the screen is read | the surviving entry's row (restored, not re-typed); then deletes it and sees "No entries yet" |

- **Nodes are found by part name.** Compose tags host controls with
  `Modifier.testTag(<part>)` and SwiftUI gives them
  `.accessibilityIdentifier(<part>)`, so one set of names serves both. A
  second mount of a component suffixes every part with `-m<N>`, so row
  matchers use a prefix (`record-list-title`), as JournalUiTest already does.
- **Launch 2 is a new process.** Recomposing in the same process would read
  the engine's memory, not the state file; only a new process shows that the
  snapshot was written where the platform keeps app data and read back.
- **Journal first.** It has the most input. Trestle follows with the same
  harness (its desktop `TaskAppUiTest` steps). Engram's picker round trip is
  §4.4.
- **The tests live with the app,** beside the desktop one:
  `<app>/conformance/compose-android/<App>AndroidUiTest.kt` and
  `<app>/conformance/swiftui-ios/<App>UiTests.swift`. The generated projects
  stay free of app-specific tests; CI hands the test to the build, as it
  copies `JournalUiTest.kt` into the desktop project today.

### 4.2 Android: an instrumented Compose test

- **The project.** `build.gradle.kts` for `android/` gains
  `testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"`
  and `androidTestImplementation` dependencies only: `androidx.test:runner`
  1.6.2, `androidx.test.ext:junit` 1.2.1 (one androidx.test release, with the
  repository's core 1.6.1) and JetBrains' `ui-test-junit4` at the app's own
  Compose version. JetBrains' module metadata maps that artifact on Android
  to androidx `ui-test-junit4` (1.11.2 for Compose 1.11.1), so the rule
  follows the Compose pin rather than holding a second one that could drift.
  *As built:* the metadata was read from Maven Central; Google's Maven is
  unreachable from the authoring sandbox, so CI's `assembleDebugAndroidTest`
  is what resolves the androidx.test pins. Every `androidTest` dependency, including §4.4's
  `espresso-intents`, is pinned to an exact version (no `+`, range or
  BOM-only resolution) and comes from the app's own repositories. The app
  APK is unchanged: `assembleDebug` never resolves
  the `androidTest` classpath, and no `debugImplementation` test manifest is
  needed because the rule drives `MosaicActivity` itself.
- **The test.** `createAndroidComposeRule<MosaicActivity>()`, so the real
  activity starts: real `filesDir`, real JNA load, real picker. Launch 1 or 2
  is chosen by an instrumentation argument (`-e mosaicLaunch 1|2`), read
  through `InstrumentationRegistry.getArguments()`, the Android form of the
  desktop test's `MOSAIC_EXPECT_RESTORED`. The test cannot set environment
  variables, so state goes where `MosaicActivity` always puts it
  (`filesDir/<application id>/mosaic-state.v1.json`).
- **Running it.** Gradle's `connectedDebugAndroidTest` uninstalls the app
  after each run, which would erase the state launch 2 needs. So:
  1. `assemble-mosaic-android-debug.sh` builds `assembleDebugAndroidTest`
     beside `assembleDebug` when asked for the test APK;
  2. a new `code/scripts/mosaic-android-ui-test.sh <apk> <test apk>
     <android package> <test class>` installs both, runs `pm clear` on the
     app so launch 1 starts empty, then runs
     `am instrument -w -r -e class <test class> -e mosaicLaunch N
     <package>.test/androidx.test.runner.AndroidJUnitRunner` for N = 1, 2.
     Each `am instrument` starts a new app process.
  3. `am instrument` exits 0 even when a test fails, so the script reads the
     raw output and requires `INSTRUMENTATION_CODE: -1` with an
     `OK (<n> tests)` line and no `FAILURES!!!`. An empty or truncated
     output fails, as the gate's logcat read does.
  `adb shell` joins its arguments into one remote command line, so nothing
  the caller passes reaches it unchecked: the package must match the gate's
  package pattern, and the test class
  `^[A-Za-z_][A-Za-z0-9_]*(\.[A-Za-z_][A-Za-z0-9_]*)+$` (no `$`, `#` or
  space, which the device shell would expand or cut). `mosaicLaunch` is the
  literal 1 or 2 in the script, never the caller's. The APK paths go only to
  host-side `adb install`, as separate arguments.
- **Driving the controls (as built).** On the emulator's phone screen the
  app shell's navigation split collapses (`collapse: auto`), so whether the
  timeline and the editor share the screen depends on the window size
  class. The test therefore drives controls through their semantics
  actions (OnClick, text input), which run a control's own handler wherever
  it is drawn, and reads the result through the semantics tree
  (`assertExists`) rather than asserting what is visible. It still proves
  that the controls reach the engine, that the screen is rebuilt from the
  engine's answer, and that a new process restores it. Asserting what a
  compact screen shows is the compact layout's own test. The desktop test's
  malformed-event check is not repeated: the activity keeps its host private.
- **Where it runs.** In the emulator step, after the three-launch gate for
  the same app. The gate leaves state behind, which is why the script clears
  first rather than relying on a fresh install.
- **Animations** are already off (`start-mosaic-android-emulator.sh`), which
  the Compose test rule needs for stable idling.

### 4.3 iOS: an XCUITest

- **The project.** `mosaic-ios-project` can add a second target: a UI test
  bundle (`com.apple.product-type.bundle.ui-testing`, `TEST_TARGET_NAME =
  App`, a dependency on App), and a shared scheme
  `xcshareddata/xcschemes/App.xcscheme` whose test action runs it, because
  `xcodebuild test` needs a scheme. Xcode lists every source file in
  `project.pbxproj`, so CI cannot copy a test in afterwards as it does on
  Android: the builder takes the test sources through a new
  `mosaic-compile --ios-ui-test <file.swift>` (repeatable). Without the flag
  the project is exactly today's: one target, no scheme. The flag takes
  only a `.swift` file whose name matches `^[A-Za-z0-9_]+\.swift$`. The file
  is copied into the project, and only that basename reaches
  `project.pbxproj`, through the same `check_path` as every other source.
  The scheme is XML built from the same names, so its attribute values are
  escaped, and builder tests cover a product name with `&` and `"`.
- **The test.** `XCUIApplication`: launch, act, `terminate()`, `launch()`
  again. A real terminate gives the new process launch 2 needs inside one
  test method. Nodes are found with `descendants(matching: .any)
  .matching(identifier:)`, and rows with a predicate
  `identifier BEGINSWITH 'record-list-title'`. Text goes in with `tap()` and
  `typeText`, which needs only the focus the tap gives.
- **A known start.** The gate leaves its own state in the app's container,
  so the step runs `xcrun simctl uninstall <udid> <bundle id>` first, and
  `xcodebuild test` installs the app fresh. The test does not override
  `MOSAIC_APP_STATE_PATH`: the override must be an absolute path, and the
  test runner cannot know the app's container, so state goes where users'
  state goes (Application Support), which is the path worth proving.
- **Running it.** `xcodebuild test -project App.xcodeproj -scheme App
  -destination 'platform=iOS Simulator,id=<udid>' CODE_SIGNING_ALLOWED=NO`
  on the simulator the gate already picked, after the gate. Test bundles
  need no signing on the simulator.
- **As built.**
  - **The test runs on the iPad simulator, in landscape.** The app shell's
    `NavigationSplitView` (`collapse: auto`) collapses into a stack on an
    iPhone, so the editor is a navigation push away from the timeline, and
    an XCUITest touches only what is on screen. On a landscape iPad both
    columns are showing.
  - **Before each Save it hides the software keyboard** when one is up,
    since that keyboard covers the bottom of a landscape iPad.
  - **It runs after the iPad launch check.** The app is uninstalled first,
    and the step requires `Executed 1 test, with 0 failures` in the log.
  - **The scheme lives in a workspace.** The builder writes
    `iOS/App.xcworkspace` (holding only `App.xcodeproj`) and puts the scheme
    there as `AppUITests`. CI runs `xcodebuild test -workspace
    App.xcworkspace -scheme AppUITests -sdk iphonesimulator`. The first runs
    put the scheme inside the project instead. Every scheme there resolved
    to no buildables, Xcode's auto-created `App` scheme included: the
    project's directory is `..`, and a project's own schemes resolve
    `container:` against it. A workspace resolves it against its own
    folder.
  - **The builder.** It takes the sources through
    `build_package_with_ios_ui_tests`, which
    `build_package_with_profile_runtime_and_tokens` now calls with none, so
    its many callers are unchanged. `mosaic-ios-project` names the bundle
    `<product>UITests`, gives it the app's bundle identifier plus
    `.uitests`, and checks its sources like every other path.

### 4.4 Engram: the picker round trip (after 4.2 and 4.3)

Engram's Anki import and export go through the platform picker (§2.6,
§3.11), which the system draws: neither test framework should drive
DocumentsUI or `UIDocumentPickerViewController`, whose layout changes with
the OS. Both platforms answer the picker for the test instead:

- **Android.** Espresso-Intents stubs `ACTION_OPEN_DOCUMENT` and
  `ACTION_CREATE_DOCUMENT` (the Activity Result API starts them through
  `startActivityForResult`, which Espresso-Intents intercepts) with a URI the
  test owns. Any provider that answers the stub is declared only in the
  `androidTest` manifest, so it ships in the test APK and never in the app's
  main or debug manifest. It is `exported="false"` and grants per-URI
  access. A `file://` fixture lives in the app's own cache and is written by
  the test. The builder test that pins `assembleDebug`'s output also checks
  that the app's manifest gains no provider. The test checks that an import of a fixture `.apkg` adds its
  cards to the screen, and that an export writes a zip whose first bytes are
  a local header. Whether the URI is a `file://` in the app's cache or a
  test-only provider is decided there, by what the picker's
  `OpenableColumns.DISPLAY_NAME` query needs.
- **iOS.** The picker is presented by the platform library, so the test
  build swaps in the fake picker the Swift harness already uses (§2.6) when
  launched with a test-only argument. The seam is compiled only with the UI
  test: it lives in a source file the builder adds only under
  `--ios-ui-test`, and only to the Debug configuration. A project built
  without the flag, and any Release build, has no code that reads the
  argument, and builder tests assert both. The argument only selects the
  fake. It never carries a path or URL, and the fake reads and writes fixed
  fixture names inside the app's own container. The rest of the seam is
  designed in that PR.

*As built (Engram).* The round trip needs no fixture. A fresh collection
lists no decks. Its export is a whole Anki package, and importing that
package back lists its `Default` deck, which a relaunch restores.
`engram-mosaic-app` pins that sequence in a unit test. Both device tests
export, import what they exported, wait for the `Default` row in the deck
list (`deck-option-button`), and on a second launch find it restored.

- *Android: no provider.* The test answers both picker intents with a
  `file://` URI in the app's cache. The router opens it through
  `ContentResolver`, which serves `file://` itself. It never requires a
  chosen document's `DISPLAY_NAME`: open checks no name, and save checks
  only the suggested one. The app is told "document", which Engram does
  not use. Espresso-Intents (3.6.1, the runner's release) is the one new
  `androidTestImplementation`. The builder test asserts that the app
  manifest has no `<provider>` and the project has no `src/androidTest`
  or debug manifest of its own.
- *iOS: the seam.* `--ios-ui-test` writes
  `UITestSupport/MosaicUITestPicker.swift` (from `mosaic-app-bindings`),
  outside `Sources/App`, and lists it in the Xcode app target. All of the
  file is inside `#if MOSAIC_UI_TEST_PICKER`, which only the app target's
  Debug configuration defines (`IosApp.debug_compilation_conditions`).
  The platform library's `mosaicSystemPicker` returns the fake only under
  that condition, and only when the process was launched with
  `-MosaicUITestPicker`. The fake saves to and opens
  `tmp/mosaic-ui-test-picker/document` in the app's container.
- *One file operation at a time.* Nothing on screen says when the
  export's answer has reached the main thread, and an Import tapped
  before then is refused. So Android waits for a whole zip (local header
  first, end-of-central-directory record last), and both tests tap Import
  at most three times until the deck appears.

### 4.5 Order and gates

1. This design (spec only).
2. Android: the `build.gradle.kts` additions (builder tests assert them, and
   that `assembleDebug`'s output is unchanged), `mosaic-android-ui-test.sh`,
   and `JournalAndroidUiTest.kt` run in the emulator step.
3. iOS: the optional UI test target and scheme (builder tests: no flag gives
   today's one target; with the flag, two targets, the dependency, and the
   scheme's test action), `--ios-ui-test`, and `JournalUiTests.swift` run in
   the simulator step.
4. Trestle on both, then Engram's round trip (§4.4).
   *As built for Trestle:* `TrestleAndroidUiTest.kt` and
   `TrestleUiTests.swift` live in `task-mosaic-app/conformance/`. They follow
   the desktop `TaskAppUiTest` and do not carry over its window-size checks,
   which are about the desktop window. In launch 1 they add a task to an
   empty Inbox, complete it, reopen it and delete it. Each step is read back
   from the toggle's engine-provided accessible name ("Complete task: …" /
   "Reopen task: …"). Launch 1 then adds the task that launch 2 must find
   restored and delete. The task name field is matched as `name-input` or
   `name-input-corrected`, because the engine's post-add focus marker swaps
   the field. Trestle's Android build keeps its own verified-wrapper Gradle
   call, which now also runs `assembleDebugAndroidTest`. The macOS step's
   timeout rises from 75 to 90 minutes for the second XCUITest run.

*Done (#16748):* steps 1–4 merged together, and every device test passed on
its first green run. On the x86_64 emulator, Trestle, Journal and Engram each
passed both cold launches. On the iPad simulator, the three XCUITests each
reported `Executed 1 test, with 0 failures`. The macOS step took 49 minutes,
under its 105-minute timeout; the emulator step took 4. One iOS problem took
three CI rounds: the scheme had to move into a workspace (§4.3), recorded in
`lessons.d/a-shared-xcode-scheme-for-a-project-whose-projectdirpath-is-belongs-in.md`.

Neither lane can be run in this repository's Linux sandbox (no `/dev/kvm`,
no Xcode), so each PR is proven in CI. A green run must show it drove the
screen: the scripts require the expected test count (`OK (1 test)`, or
xcodebuild's `Executed 1 test, with 0 failures`), so a test that was never
compiled in or was filtered out fails the step instead of passing it.

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
   §3.10; on iOS and iPadOS: §2.5. CI drives each app's generated controls
   on both platforms (§4, done). Venture is still open.*
8. **Flutter:** `flutter create --platforms=android,ios`, per-ABI native
   assets, `path_provider` for state. *Designed in §7. Trestle passed
   its gates on both phones (§7.8, §7.9), and CI builds and gates Journal
   and Engram the same way (§7.10). Phone file effects are designed in
   §7.11. Flutter device UI tests remain (§7.6).*

iOS goes first because the emitted source already compiles for it; the gap is
packaging only.

## 6. What this does not decide

- App Store / Play Store distribution, signing identities, and store assets.
- Background execution, push notifications, widgets.
- Whether Venture's paint surface uses Metal on iOS (likely) or a shared
  software path; BR02 P10 decides.

## 7. Flutter on phones, designed (step 8)

Written before implementation. Mosaic's Flutter backend runs Trestle and
Engram on Linux, macOS and Windows. This section takes the same generated
project to Android and iOS. Every surface it needs is already in the code,
and each is still desktop-only.

| surface | today | on a phone |
|---|---|---|
| runner directories | the builder writes none; CI runs `flutter create --platforms=linux` | `flutter create --platforms=android,ios` (§7.1) |
| runtime input | one `.so`, `.dylib` or `.dll` file; a directory is the Compose jniLibs and is refused | a runtime **directory** with an `android/` half and an `ios/` half (§7.2) |
| `hook/build.dart` | registers that one file. One `.so` matches one ABI, so `flutter build apk` fails for every other ABI | picks the file for the target's ABI or SDK (§7.3) |
| state path | `HOME` / `XDG_DATA_HOME`, which an Android app does not have, so persistence is silently off there | the platform's app-support directory, from `path_provider` (§7.4) |
| file effects | refused with "… is not available on this platform yet" | unchanged in step 8; designed separately (§7.6) |
| CI | Linux builds and tests only | an APK through the emulator gate; an iOS simulator build through the simulator gate (§7.5) |

### 7.1 Runner directories

The builder still writes no runner. A runner is Flutter's template and
belongs to the Flutter version that creates it. CI and people run
`flutter create` in the generated project, as they do on desktop. What the
builder adds is the command, written into the generated README:

    flutter create --platforms=android,ios --org <org> --project-name <name> .

- **The identity comes from the manifest.** `[app] bundle_identifier`
  `dev.codingadventures.trestle` splits at its last dot into
  `--org dev.codingadventures` and `--project-name trestle`. Android's
  `applicationId` and iOS's bundle identifier are then the manifest's
  identity, as they are for Compose and SwiftUI.
- **Every part must be valid for both platforms.** The project name must
  be a Dart package name: `^[a-z][a-z0-9_]*$`, and not a Dart reserved
  word. Each org part must match the same pattern and not be a Java or
  Kotlin keyword, so the org is a valid `applicationId` and Kotlin
  package. A manifest identifier is `[A-Za-z0-9-]` and dots, which can
  still yield `my-app`, `2d` or `dev.new`.
- **If any part fails,** the README gives no command and says which part,
  and it never makes up a different identity. (Compose sanitizes the same
  identifier into an `applicationId`; Flutter cannot, because `flutter
  create` derives it.)
- **No quoting is needed.** Every token in the command is then free of
  shell metacharacters.
- The desktop command in the README is unchanged.

### 7.2 The runtime directory

`--runtime-library <dir>` for the Flutter backend takes a directory in one
fixed layout. Either half may be absent, but at least one must be present:

    <dir>/android/arm64-v8a/libmosaic_app.so
    <dir>/android/armeabi-v7a/libmosaic_app.so
    <dir>/android/x86_64/libmosaic_app.so
    <dir>/android/x86/libmosaic_app.so
    <dir>/ios/iphoneos/libmosaic_app.dylib          arm64
    <dir>/ios/iphonesimulator/libmosaic_app.dylib   arm64 + x86_64 (lipo)

- **The Android half** is exactly the jniLibs layout that
  `build-mosaic-android-libs.sh` writes for Compose (which builds all four
  ABIs), read by the same strict reader.
- **The iOS half** is dynamic. Flutter's native assets bundle a dynamic
  library into the app as a framework, and it cannot link a static one, so
  SwiftUI's static `.xcframework` does not carry over. A new script,
  `build-mosaic-ios-dylibs.sh`, builds the app crate as a `cdylib` for
  `aarch64-apple-ios`, `aarch64-apple-ios-sim` and `x86_64-apple-ios`. It
  joins the two simulator slices with `lipo`, as the static script does.
- **Reading the directory: nothing is copied recursively.** The rules:
  - The top level may hold only `android/` and `ios/`, each a real
    directory, not a link.
  - `android/` is read by `android_jni_libs`, Compose's strict reader.
  - `ios/` may hold only `iphoneos/` and `iphonesimulator/`, each holding
    exactly one regular `libmosaic_app.dylib`.
  - Any other entry, link or file type is refused, never ignored.
  - Each selected file is read with `read_regular_file_without_links`,
    which compares device and inode after opening, so a link swapped in
    after the check is still refused. It is written to its fixed path
    under `runtime/`.
- **Clearing first.** Before installing, the builder removes the project's
  `runtime/` entirely, as Compose clears jniLibs. Only this build's files
  then exist there. A reused output directory cannot keep last time's
  `x86` library, or a desktop `libmosaic_app.so`, for the hook to bundle.
- **Which ABIs.** `android_jni_libs` accepts any non-empty subset of the
  four ABIs. An APK build that asks for an ABI with no file fails in the
  hook (§7.3), saying which one.
- **What each file must be.**
  - Android: ELF, as the hook checks.
  - `iphoneos`: thin `MH_MAGIC_64` arm64.
  - `iphonesimulator`: fat `FAT_MAGIC` with exactly arm64 and x86_64.
  - Each iOS slice's `LC_BUILD_VERSION` platform must be IOS (2) in
    `iphoneos` and IOSSIMULATOR (7) in `iphonesimulator`. A simulator
    library swapped into the device folder would otherwise pass every
    architecture check and fail only on a real device, which CI never
    runs.
- **What is refused.** A desktop runtime file and a runtime directory
  cannot be mixed. A Flutter build is either for desktop or for phones, as
  a Compose build with jniLibs is for Android. The refusal names the
  expected layout.

### 7.3 `hook/build.dart` per ABI and per SDK

The hook already guards on the target OS and verifies what it hands
Flutter: ELF `e_machine` for a `.so`, and `lipo` slices for a `.dylib`.
With a runtime directory it first chooses the file:

| `targetOS` | chooses | by |
|---|---|---|
| Android | `runtime/android/<abi>/libmosaic_app.so` | `targetArchitecture`: arm64 → `arm64-v8a`, arm → `armeabi-v7a`, x64 → `x86_64`, ia32 → `x86` |
| iOS | `runtime/ios/<sdk>/libmosaic_app.dylib` | the code config's iOS target SDK: device → `iphoneos`, simulator → `iphonesimulator` |

It then runs the existing verifier on that file. The CodeAsset is the same
one (`mosaic_host.dart`, `DynamicLoadingBundled`), so `mosaic_host.dart`'s
`@Native` bindings do not change.

- **Missing file.** An ABI or SDK the directory has no file for fails the
  build, naming the path it looked for. It never falls back to another
  ABI's library.
- **Outside the map.** An architecture outside the four-entry map (riscv64,
  say) or an unknown SDK fails before any path is built, naming what was
  asked for.
- **Stricter than the desktop verifier.** With a runtime directory, the
  hook refuses a file that is not ELF (Android) or Mach-O (iOS). The
  desktop verifier's "too short or not ELF: left alone" does not apply.
  It also repeats the iOS platform check from §7.2. Like the `lipo`
  slicing it already does, the check runs its tool through `Process.run`
  with an argument list, never through a shell.

### 7.4 State through `path_provider`

`_statePath()` keeps its desktop roots exactly. A desktop user's state does
not move. On Android and iOS it uses a root the generated `main.dart` sets
before the host loads:

    WidgetsFlutterBinding.ensureInitialized();
    if (Platform.isAndroid || Platform.isIOS) {
      mosaicStateRoot = (await getApplicationSupportDirectory()).path;
    }

- **Where that is.** It is `Context.getFilesDir()` on Android and the
  sandbox's `Library/Application Support` on iOS. The state file is
  `<root>/<application id>/mosaic-state.v1.json`. That is where Compose's
  `MosaicActivity` and SwiftUI's iOS host already keep it, so the existing
  emulator and simulator gates can check a Flutter app.
- **When the root is unknown.** If the call throws, or a phone build has no
  root, persistence is off and the host says so through its existing
  persistence warning. It never writes beside the executable.
- **No desktop fallback on a phone.** On Android and iOS, `_statePath`
  uses only `MOSAIC_APP_STATE_PATH` or `mosaicStateRoot`. If neither is
  set, it returns null. It never consults `HOME` or `XDG_DATA_HOME`, which
  would put Android in the desktop branch. On desktop `mosaicStateRoot` is
  ignored.
- **`MOSAIC_APP_STATE_PATH` still wins, for tests.** No other app can set
  it on Android. On iOS only a developer can, through the simulator
  (`SIMCTL_CHILD_…`) or an Xcode scheme, which is acceptable.
- **The bundled runtime only.** On Android and iOS the bundled runtime is
  always used, and `MOSAIC_APP_LIBRARY` is ignored. That variable is a
  desktop development override and has no meaning in a signed app.
- **The dependency.** `path_provider` is pinned exactly in the generated
  `pubspec.yaml`, as `file_selector` is. Its endorsed implementations are
  published by flutter.dev:
  - `path_provider_android` and `path_provider_foundation`;
  - on desktop, also `path_provider_linux`, `path_provider_windows` and
    `xdg_directories`.

  They resolve with `pub get` within its ranges, as `file_selector`'s do,
  and generated projects ship no lockfile. CI prints the resolved versions
  from `pubspec.lock`, so a change in them is visible in the log.

### 7.5 CI

Neither lane runs in this sandbox: there is no `/dev/kvm` and no Xcode. Each
PR is proven in CI.

- **Android, on the Linux runner after the Compose emulator gate.**
  1. Build Trestle's jniLibs (the Compose step already does).
  2. `pkg --backend flutter --emit-project --runtime-library <dir>`, with
     only the `android/` half.
  3. `flutter create --platforms=android …`, then set
     `android:allowBackup="false"` in the created
     `android/app/src/main/AndroidManifest.xml`, as Compose's manifest
     does. The README says to make the same edit. Then
     `flutter build apk --debug`.
  4. Check the APK:
     - its manifest says `allowBackup` false (`aapt2 dump xmltree`);
     - it holds `lib/<abi>/libmosaic_app.so` for every ABI it packages,
       each byte-equal to its input;
     - those ABIs include `x86_64`, which the emulator runs, and
       `arm64-v8a`.

     The build may not package 32-bit `x86`, so the check does not
     require it.
  5. `adb uninstall` the Compose Trestle first. It has the same package
     name, so its data and signing key must not carry over.
  6. Run the same `mosaic-android-emulator-gate.sh`, which gains an
     optional fourth argument: an activity class name only, never a full
     component, because Flutter's launcher is `.MainActivity` rather than
     `mosaic.android.MosaicActivity`.
     - It must match `^\.?[A-Za-z][A-Za-z0-9_]*(\.[A-Za-z][A-Za-z0-9_]*)*$`,
       or the gate exits 2.
     - The gate builds the component itself as `$package/$activity`. The
       default stays `mosaic.android.MosaicActivity`, and the gate accepts
       three or four arguments.
     - The rule matters because `adb shell` joins its arguments into one
       device command line, so local quoting does not protect them. Every
       value the gate splices in must be validated first.
- **iOS, on the macOS runner after the SwiftUI iOS gates.**
  1. Run `build-mosaic-ios-dylibs.sh`.
  2. `pkg` with the `ios/` half.
  3. `flutter create --platforms=ios …`, then
     `flutter build ios --simulator --debug --no-codesign`.
  4. Run `mosaic-ios-simulator-gate.sh` on the `Runner.app`.

### 7.6 Not in step 8

- **File effects on phones.** `file_selector` opens a document on Android
  and iOS, but it has no save there. Saving needs a document-create intent
  on Android and an export picker on iOS, which means platform code that the
  generated project does not own. That is its own design, after this step,
  and the refusal message stays until then. *Designed in §7.11.*
- **Device UI tests** for Flutter (`integration_test`), as §4 did for
  Compose and SwiftUI.
- **Release builds, signing and store packaging**, as §6 says for every
  backend.

### 7.7 Order

1. This design (spec only).
2. **Android:** the runtime directory's `android/` half, the per-ABI hook,
   the state root, the README command, the gate's activity argument, and the
   CI APK and emulator gate. Trestle first.
3. **iOS:** `build-mosaic-ios-dylibs.sh`, the `ios/` half, the per-SDK hook,
   and the CI simulator build and gate.
4. Journal and Engram on both, then file effects (§7.6).

### 7.8 Android, as built (step 2)

What differs from the design, or what the design left open:

- **No `ios/` yet.** `ios/` in a runtime directory is refused with a message
  naming step 3, rather than read early. The README's command is
  `flutter create --platforms=android …`; step 3 adds `ios`.
- **Phone builds only.** `path_provider` and the state-root `main()` are
  added to phone builds only. A desktop Flutter build's `pubspec.yaml` and
  `main.dart` are byte-for-byte what they were. `main()` becomes
  `Future<void> main() async`, which finds the root before `runApp`. If the
  generated `main()` is not the plain one, the build fails rather than
  editing something else.
- **The host template does change for every build**, but nothing changes on
  desktop:
  - `mosaicStateRoot` and `_isPhone` are new.
  - `_statePath` gives phones a branch of their own.
  - iOS no longer shares macOS's `HOME` root. No iOS Flutter build existed
    before this step.
  - On a phone, `MOSAIC_APP_LIBRARY` is ignored, and a missing root sets
    the persistence warning: "Mosaic state is not saved: this phone has no
    app-support directory for it".
- **`runtime/` is cleared on every Flutter runtime install**, desktop as
  well as phone, so neither kind of build leaves files for the other's hook.
- **Bare jniLibs are refused.** Compose's bare jniLibs layout passed as a
  Flutter runtime is refused (`x86_64 is not part of a Flutter phone
  runtime`): the `android/` level is what tells the two apart.
- **The pin.** `path_provider` is 2.1.6, whose floor (Dart 3.10, Flutter
  3.38) is the bundled-runtime floor. Locally, `flutter pub get` and
  `flutter analyze` of a generated Trestle phone project (Flutter 3.44)
  report no issues.
- **CI** follows §7.5. The Flutter APK step runs when both the Compose and
  Flutter lanes do. The emulator step gates it last, after the Compose
  apps, with its timeout raised to 55 minutes.

### 7.9 iOS, as built (step 3)

- **The library.** `build-mosaic-ios-dylibs.sh <cargo-package> <phone-runtime-dir>`
  builds the app crate with `cargo rustc --crate-type cdylib` for
  `aarch64-apple-ios`, `aarch64-apple-ios-sim` and `x86_64-apple-ios`. It
  writes `ios/iphoneos/` (thin arm64) and `ios/iphonesimulator/` (the two
  simulator slices, joined with `lipo`). It replaces only `ios/`, so an
  `android/` half beside it is kept.
- **The deployment target.** The script exports
  `IPHONEOS_DEPLOYMENT_TARGET=16.0`, as Mosaic's iOS app target uses. That
  also makes the linker write `LC_BUILD_VERSION`. An older target gets only
  `LC_VERSION_MIN_IPHONEOS`, which cannot tell device from simulator, and
  the builder refuses such a library by name.
- **The builder's checks.** The builder reads `ios/` as it reads
  `android/`: known SDK directories only, each with exactly one regular
  library, nothing followed through a link, at least one present. Each
  library's Mach-O is checked without trusting a single offset:
  - `iphoneos`: a thin `MH_MAGIC_64` arm64 library, platform 2;
  - `iphonesimulator`: `FAT_MAGIC` holding exactly arm64 and x86_64, each
    slice's CPU type agreeing with the fat header, and platform 7.
- **The hook's iOS branch.** It chooses by `IOSSdk` (`iPhoneOS` or
  `iPhoneSimulator`, nothing else), slices with the existing `lipo`
  helper, and reads the thin slice's `LC_BUILD_VERSION` again in Dart. It
  runs no tool through a shell.
- **The README.** It names only the platforms the runtime has:
  `--platforms=android`, `ios` or `android,ios`. The Android manifest edit
  appears only with an Android half, and an iOS half adds the
  `flutter build ios` commands.
- **CI.**
  - The macOS job sets Flutter up when the Swift lane runs, and builds
    `flutter build ios --simulator --debug`.
  - It finds `_mosaic_app_create` in one of the `Runner.app`'s frameworks.
  - It uninstalls the SwiftUI Trestle (same bundle id), then runs
    `mosaic-ios-simulator-gate.sh` on the iPhone simulator.
  - A local `flutter analyze` of a generated Android-and-iOS phone project
    (Flutter 3.44) finds no issues. That confirms the hook's `IOSSdk`
    calls exist in `code_assets`.

### 7.10 Every app, as built (step 4)

*Steps 2 and 3 are done (#16834).* Trestle's Flutter APK and its emulator
gate, and its Flutter iOS simulator build and simulator gate, passed in CI.
That run also showed one thing the design had wrong. The Android Gradle
plugin strips debug symbols from the native libraries it packages, so the
APK's copy of an engine can never be byte-identical to its input. CI now
compares what survives stripping: the ELF machine, and the exported
symbols, including `mosaic_app_create`.

Step 4 brings Journal and Engram along, by the same path:

- **One script for every app.** `build-mosaic-flutter-phone-app.sh
  <android|ios> <program> <phone-runtime> <output> <org> <name>` holds
  what CI did for Trestle alone:
  - `pkg` must be native-complete.
  - The README must give the expected `flutter create` command, which then
    runs.
  - Android gets `allowBackup="false"`.
  - Then `pub get`, `analyze`, and a debug build.
  - Then the checks on the built app:
    - Android: `aapt2` reads `allowBackup` false, and each packaged ABI has
      its input's machine and symbols, with x86_64 and arm64-v8a present.
    - iOS: the bundle id, and `_mosaic_app_create` in a framework.

  It refuses an org or name outside the README's own shapes, and an empty
  or `/` output, before building anything.
- **CI.**
  - **Android:** one step builds all three apps from the per-ABI engines
    their Compose steps built.
  - **Emulator:** the emulator step uninstalls each Compose app, which has
    the same package, and gates its Flutter APK through `.MainActivity`.
  - **iOS:** the macOS step builds each crate's iOS dylibs, builds each
    app, and gates it on the iPhone simulator after uninstalling the
    SwiftUI app (same bundle id).
  - **The app lists.** Each loop reads its list on file descriptor 3, so
    `adb shell`, `flutter` or `cargo` reading stdin cannot swallow the
    rest of it.
- **Locally**, Journal and Engram phone projects (both halves) are
  native-complete, and `flutter analyze` (Flutter 3.44) finds no issues in
  either.


### 7.11 File effects on phones, designed (step 5)

Written before implementation. This is the first item §7.6 left out. A
Flutter app on Android or iOS answers `files.open` and `files.save` with
the system's own pickers. Engram's import and export go through the same
pickers. The contracts, limits and name rules (UI87 §3.1) do not change:
a phone save refuses exactly the names a desktop save refuses, and an open
reads at most 50 MiB. Until this lands, each request still fails with
"… is not available on this platform yet".

**Why `file_selector` is not enough.**

| | Android (`file_selector_android` 0.5.2) | iOS (`file_selector_ios` 0.5.3) |
|---|---|---|
| open | reads the **whole** document into memory, then copies it again into the cache, before Dart sees it. A provider's 2 GiB file is 2 GiB of heap, with no stall watch | a copy, through `UIDocumentPickerViewController` |
| save | `getSaveLocation` is unimplemented | `getSaveLocation` is unimplemented |

So a phone needs platform code: Storage Access Framework intents on Android,
and the export picker on iOS. A runner belongs to `flutter create` (§7.1),
so that code cannot live there.

**A plugin the builder owns.** A phone build writes a local Flutter plugin,
`mosaic_phone_files/`, at the project root, and a path dependency on it in
`pubspec.yaml`:

    mosaic_phone_files:
      path: mosaic_phone_files

- Flutter's tooling registers a path-dependency plugin through
  `GeneratedPluginRegistrant`, as it registers `path_provider`. So nothing in
  the runner changes, and `flutter create` can run before or after the
  builder.
- The plugin has an `android/` half (Kotlin) and an `ios/` half (Swift,
  with a `Package.swift` for Swift Package Manager and a `.podspec` for
  CocoaPods, whichever the Flutter tool uses). It declares only the
  platforms the runtime has: a runtime with only an `android/` half gets a
  plugin with only `android/`, as the README already names only those
  platforms (§7.9).
- A desktop build writes no plugin, and its `pubspec.yaml` and `main.dart`
  stay byte for byte as they are (§7.8).
- The plugin's name is reserved: a package whose own Dart files or
  dependencies use `mosaic_phone_files` is refused, as the other generated
  names are (UI48 §7.5).

**One channel, two methods, paths not bytes.** The plugin talks over one
`MethodChannel`, `dev.codingadventures.mosaic/phone_files`, and never
carries a document's bytes across it:

| method | arguments | answer |
|---|---|---|
| `open` | `directory`, `mimeTypes`, `limit` | `{ "path" }`, or null for a cancel |
| `export` | `path`, `mimeType` | `{ "name" }`, or null for a cancel |

A failure is a `PlatformException` whose code is one of `busy`, `no_window`,
`activity_gone`, `too_large`, `stalled` or `unreadable`. Dart maps each
code to a fixed message and never shows the exception's own text, which can
carry a provider's path. Any other code, a `MissingPluginException`, or an
answer of the wrong shape gives the generic message: "couldn't read the
file" for an open, "couldn't save the file" for an export.

- **Why paths.** Up to 50 MiB in a channel message is copied at least
  twice, and on Android it is encoded on the main thread. A path keeps the
  bytes in files the app owns. Dart then reads them with the bounded reader
  it already has (`_mosaicReadOpened`, in a background isolate) and writes
  them with the exclusive create it already has.
- **Dart owns the directory.** For each request Dart creates a fresh
  directory, `<temporary>/mosaic-files/<random>/`. `<temporary>` comes from
  `path_provider`'s `getTemporaryDirectory()`: `cacheDir` on Android, and
  the sandbox's `tmp/` on iOS. Dart passes the directory to `open` and stages
  a save in it. It deletes the directory in a `finally`, whether the request
  succeeded, was cancelled or failed.
- **The plugin trusts nothing the channel sends.** Any Dart code in the
  process can call the channel by its name, so reserving the package name
  is not a boundary. Every argument is checked as if hostile:
  - **The directory.** A directory is accepted only when its canonical path
    is directly below the canonical `<its own temporary>/mosaic-files/`
    (iOS canonicalizes `/var` to `/private/var` on both sides). The plugin
    opens a descriptor for it, and creates the open copy relative to that
    descriptor with `O_CREAT | O_EXCL | O_NOFOLLOW`. It writes only that one
    new file and deletes nothing.
  - **The staged file.** `export` accepts only a regular file, not a link,
    directly inside a directory that passes the same check. It is opened
    with `O_NOFOLLOW`, checked to be a regular file through that
    descriptor, and is at most 16 MiB. Its name must pass
    `mosaicIsPlainFileName` and the executable-extension rule, ported to
    the plugin's language, whatever Dart already checked. A FIFO, a device,
    or a link to the app's own `shared_prefs` or databases is refused as
    `unreadable` before any picker is shown.
  - **`limit`** is clamped to 50 MiB.
  - **One call at a time.** A second call while one is waiting is `busy`,
    whoever makes it.
- **Dart trusts nothing the plugin answers.** The path from `open` is
  accepted only when it is a regular file, not a link, directly inside the
  request's own directory. It is read without resolving links, unlike a
  desktop dialog's choice, which `_mosaicReadOpened` follows on purpose.
  Anything else is the generic failure.
- **Leftovers.** A process killed mid-request leaves its directory behind.
  Dart makes each request's directory with `Directory.createTemp`, mode
  0700. Before its first request, each router deletes the entries of
  `<temporary>/mosaic-files/` that have not changed for more than an hour,
  aged by each entry's last modification. It does not follow links, and it
  never deletes a younger entry. The rule of one
  operation at a time holds for each router, not for the process: a second
  Flutter engine (an iPad's second scene) may have a request in flight
  there.

**Android.**

- *The plugin.* `MosaicPhoneFilesPlugin` is a `FlutterPlugin`,
  `ActivityAware` and `PluginRegistry.ActivityResultListener`. It starts
  `ACTION_OPEN_DOCUMENT` and `ACTION_CREATE_DOCUMENT` with
  `startActivityForResult` on the attached activity. A plain
  `FlutterActivity`, which is what `flutter create` writes, has no
  `ActivityResultRegistry`, so Compose's `MosaicAndroidDocumentPicker`
  cannot be reused as it is. The plugin's two request codes are its own,
  and any other code is not its result.
- *Open.* The intent asks for the request's MIME types (`*/*` when none
  map), with `CATEGORY_OPENABLE`. On a background thread, the plugin reads
  the document through `openAssetFileDescriptor(uri, "r", signal)` and
  copies **at most `limit + 1` bytes** into `<directory>/<name>`. It stops
  as soon as the copy passes `limit` and answers `too_large`, so a large
  document is never read whole. `<name>` is the provider's display name
  under §3.8's ordinary-name rule, else `document` (below).
- *Only another app's documents.* A result is used only when its `Uri` has
  the `content` scheme and an authority that is none of this app's own
  providers (checked with `PackageManager.resolveContentProvider`). Any
  other result, `file:` included, is `unreadable`, so a crafted result can
  never make the plugin read or overwrite the app's private files. No
  persistable permission is taken.
- *Save.* `ACTION_CREATE_DOCUMENT` with `EXTRA_TITLE` set to the
  suggested name and the type Dart chose (§3.8's rule: the name's own type
  when the request accepts it or anything, else the first accepted type). On
  a background thread, the staged file is copied into the document
  through `openAssetFileDescriptor(uri, "wt", signal)` in 64 KiB pieces. The
  answer's name is the provider's display name, under the same rule. As in
  §3.8, a failed write is reported and the document is never deleted.
- *The stall watch, shared.* Both transfers go through §3.8's
  `MosaicStallWatch` (60 seconds without progress fails them as `stalled`),
  including the cancellation of `openDocument` and the single close. Those
  pieces move out of `MosaicFileEffects.kt` into a new template,
  `MosaicDocumentTransfer.kt`:
  - `MosaicStallWatch`;
  - `mosaicWatchedInput` and `mosaicWriteWatched`;
  - `mosaicReadBounded`;
  - the display-name and MIME-shape checks;
  - new: the plugin's directory and staged-file checks (above), which use
    only `java.io` and `android.system.Os`'s POSIX calls through a small
    interface, so the JVM harness can run them with a fake.

  It has no `package` line, as the Compose files have none. The Compose
  Android project gets it beside `MosaicFileEffects.kt`. The plugin gets
  the same file, and its packaged plugin class imports those declarations
  by their simple names. Kotlin allows that from the root package, though
  Java does not.
  The Android PR confirms it by compiling; if it does not compile, the
  template gains a package line chosen per destination, and nothing else
  changes.
- *One change to the name rule, everywhere.* A name longer than 255 UTF-8
  bytes is no longer ordinary, because 255 UTF-16 units can be more bytes
  than a file system's name limit. The clause is added to §3.8's rule on
  Compose Android, to SwiftUI's iOS rule and to the plugin's, so all three
  keep one rule. A name the clause refuses is `document`. Apart from this
  clause and the moved file, Compose and SwiftUI output does not change.
- *The activity goes away.* `onDetachedFromActivity` (and
  `ForConfigChanges`, since `flutter create`'s manifest handles rotation
  itself) answers the waiting call `activity_gone`, and a result that
  arrives later is dropped. A launch that throws answers `unreadable` and
  leaves the plugin not waiting.

**iOS.**

- *Open.* `UIDocumentPickerViewController(forOpeningContentTypes:asCopy:
  true)`. The content types are the `UTType`s of the request's extensions,
  or `.item` when none map, as in §2.6. Keeping `asCopy: true` was decided
  after step 6 (§3.8). The plugin moves the system's copy into
  `<directory>/<name>` with `FileManager.moveItem`, which refuses an
  existing name, and answers its path. Dart then enforces the limit,
  as for the SwiftUI open, which also reads the whole copy.
- *Save.* `UIDocumentPickerViewController(forExporting: [staged], asCopy:
  true)`. The picker confirms any replace. The answer's name is the last
  path component of the URL the picker reports.
- *Names.* The open copy's name and the export's reported name pass the
  same ordinary-name rule as Android's, else they are `document`.
- *Where and how long.* These follow `MosaicUIKitDocumentPicker` (§3.8):
  - the picker is shown from the foreground scene's key window, on its
    topmost presented view controller;
  - `no_window` when there is none;
  - the picker is its own delegate, and a picker released without a delegate
    call answers a cancel from `deinit`;
  - a presentation UIKit refuses fails at once.

  The Swift is the plugin's own. The SwiftUI library's picker is typed on
  that library's router and host, which a plugin module cannot import.
  The two are kept parallel by a Rust test that checks each has the same
  presentation and `deinit` rules.

**Dart.**

- *The seam.* The core gains `MosaicPhoneDocuments`, the plugin's two
  methods as an interface, so the headless conformance test can answer
  them with fakes:

      abstract interface class MosaicPhoneDocuments {
        Future<String?> copyForOpening(String directory, List<String> mimeTypes, int limit);
        Future<String?> export(String stagedPath, String mimeType);
      }

- *The phone path.* `mosaicRunPhoneFilesOpen` and `mosaicRunPhoneFilesSave`
  sit beside the desktop functions and share their checks:
  - **Open:** copy, read the copy with `_mosaicReadOpened`, and answer with
    the copy's name. A `too_large` from the plugin and an oversized copy
    both give the desktop's "too large" failure.
  - **Save:** check the request exactly as `mosaicRunFilesSave` does, before
    any picker is shown. Stage the bytes as `<directory>/<suggestedName>`
    with an exclusive create, export, and answer `ok { name }` or
    `cancelled {}`.
  - The directory is removed in a `finally` either way.
  - `mosaicConfirmReplacing` is not used: the provider or picker confirms a
    replace.
- *The router.* `installMosaicPlatformRouter` takes `phoneDocuments`.
  `mosaicPlatformHasFileDialogs` is true on Android and iOS when it is
  given. The one-at-a-time rule, deferral and the single answer are the
  router's, unchanged. Without `phoneDocuments` (a phone build made before
  this step), the "not available" failure stays.
- *The app seam.* `mosaicOpenForApp(host, id, extensions, limit, ok)` and
  `mosaicSaveForApp(host, id, suggestedName, bytes, extensions, ok)` lend
  the router's pickers to a package's own handler, as Kotlin's
  `openForApp` and `saveForApp` do (§3.11). The busy rule, deferral and
  exactly-one-answer are shared with `files.*`. They keep §3.11's rules
  unchanged:
  - `mosaicCheckSaveName`, and an executable extension refused for an app
    save whatever the app accepts;
  - a refusal never throws into the app's handler;
  - the request in flight, asked for again with the same id, is left to
    its own picker;
  - a throw inside the operation still answers the effect and frees the
    router.

  On desktop they use the
  dialogs and on a phone the plugin. Engram's Flutter handler moves to the
  seam, so its `importAnki` and `exportAnki` work on phones. Its desktop
  behaviour does not change: the seam runs the same dialogs it calls
  today.

**Gates.**

- **Dart VM.** The conformance harness drives the phone path with fake
  `MosaicPhoneDocuments`:
  - a cancel;
  - a copy that is read and then deleted;
  - an answered path that is a link, outside the directory, or not a
    regular file, each refused without being read;
  - an unknown code, `MissingPluginException` and a malformed answer, each
    given the generic message;
  - leftover cleanup that keeps a younger entry and does not follow a link;
  - an oversized copy, and `too_large`;
  - each plugin code mapped to its fixed message;
  - a save refused before `export` is called;
  - the staged file deleted after success, cancel and failure;
  - a second request refused as busy;
  - the app seam, through the same fakes.
- **Rust.**
  - A phone build writes the plugin and the path dependency, and a desktop
    build is unchanged.
  - Only the runtime's platforms are declared.
  - The name is reserved.
  - `MosaicDocumentTransfer.kt` is byte-identical in the Compose Android
    project and in the plugin.
  - The plugin's Kotlin and Swift each contain the argument checks above:
    the directory, the staged file, the clamp, `busy`, and, on Android,
    the `content` scheme and the authority check.
- **JVM.** The Compose harness checks the name rule's new 255-byte clause,
  and that the plugin's directory and staged-file checks refuse a link, a
  FIFO, a file outside the directory, and an oversized file.
- **CI.**
  - The Flutter phone build script (§7.10) also checks that
    `MosaicPhoneFilesPlugin` is in the APK's dex, and that the plugin's
    Swift class is in the `Runner.app`.
  - `flutter analyze` covers the plugin's Dart.
  - Driving the pickers themselves is the other item of §7.6, the Flutter
    device UI tests (`integration_test`), which land after this.

**Order.**

1. This design (spec only).
2. **Dart:** `MosaicPhoneDocuments`, the phone path, the router's
   `phoneDocuments`, the app seam, and the conformance checks. No phone
   build passes a `phoneDocuments` yet, so nothing changes on a device.
3. **Android:** the plugin's `android/` half, `MosaicDocumentTransfer.kt`
   moved out of `MosaicFileEffects.kt`, the builder writing the plugin, and
   the dex check. Phone builds pass the plugin, so Android answers.
4. **iOS:** the plugin's `ios/` half and the `Runner.app` check.
5. **Engram:** its Flutter handler on the app seam, on every platform.
