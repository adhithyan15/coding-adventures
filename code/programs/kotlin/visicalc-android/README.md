# VisiCalc — Android (Jetpack Compose) demo

Ninth cross-backend VisiCalc visualisation.  Native Android app
running on Jetpack Compose, sibling to:

- `code/programs/kotlin/visicalc-compose/` — Compose for Desktop (JVM)
- `code/programs/swift/visicalc-swiftui/` — SwiftUI (macOS + iOS)
- `code/programs/dart/visicalc-flutter/` — Flutter (mobile + desktop + web)
- `code/programs/cpp/visicalc-qt/`, `code/programs/typescript/visicalc-electron/`, ...

## What it shows

A single `MainActivity` hosting the Compose surface.  `MainActivity`
uses `androidx.activity.compose.setContent { ... }` — the standard
Compose-on-Android entry point.  Once that's bootstrapped, the
composables themselves (`FormulaBar.kt`, `Grid.kt`) are **byte-for-
byte identical** to the visicalc-compose Desktop demo's
composables.  Jetpack Compose for Android and Compose for Desktop
share the `androidx.compose.*` package and runtime, so the same
composable functions render unmodified on both platforms — that's
the whole point of Compose Multiplatform's design and the reason
this demo can hand-write almost nothing new compared to its
desktop sibling.

The 5×5 sample dataset and dark theme are shared with every other
visicalc-* demo so the screenshot you take on an Android phone or
emulator looks visually identical to its React, HTML, Qt, Flutter,
SwiftUI, and Compose Desktop siblings.

### Desktop/touch FormulaBar toggle

A **Touch bar** button flips the formula bar between two composables
generated from the *same* `FormulaBar.mil` interface: the desktop `Row`
(address label left of the input) and the UI30 touch `Column` (address
label stacked above a full-width input — the phone arrangement, so this
demo defaults to touch). `scripts/build.sh` emits both; because the
Compose emitter names the composable after the `.mil` component
(`FormulaBar`) and also emits the shared `sealed class FormulaBarEvent`,
the touch output has its duplicate event class stripped (it reuses the
one in `FormulaBar.kt`) and its composable renamed to `FormulaBarTouch`.
`MainActivity` holds a `touch` flag and calls one or the other with the
identical dispatch — editing behaves the same in both; only the shape
changes. This is the Android sibling of the Qt / Compose / Flutter toggles
and the web demo's switcher (the UI30 "one component, many layouts" invariant).

## What this demo does NOT yet do

- No `mosaic-compile --backend compose` exists yet (tracked as
  `emit-compose` in the autonomous loop's state file).  When it
  lands, `FormulaBar.kt` here becomes a `lib/generated/...kt`
  symlink to a generated file, and this app shifts to the half-
  generated / half-hand-written shape used by SwiftUI / Qt / Flutter.
- No strict-Flux dispatch yet.  Local state via
  `remember { mutableStateOf(...) }` for v0.1.0.  When the Compose
  emitter ships a real `dispatch: (Event) -> Unit` parameter,
  the host will switch to a `MosaicStore<AppState>` from the
  `mosaic-flux-compose` runtime.

## Prerequisites

- **Android SDK** with API 35 platform installed
- **Java 17** JDK
- **Gradle** is fetched from the system PATH (`mise exec --` is
  not used inside BUILD scripts per repo lessons.md)

Point Gradle at your SDK:

```bash
cp local.properties.example local.properties
# edit local.properties to set sdk.dir
```

Or set the `ANDROID_HOME` environment variable.

## How to build the APK

```bash
gradle --no-daemon :app:assembleDebug
```

The debug APK lands at
`app/.gradle-out/outputs/apk/debug/app-debug.apk`.  Install on a
device or emulator with `adb install app/.gradle-out/outputs/apk/debug/app-debug.apk`.

## How to launch on a connected device / emulator

```bash
gradle --no-daemon :app:installDebug
adb shell am start -n com.example.visicalc/.MainActivity
```

## File tree

```
code/programs/kotlin/visicalc-android/
├── README.md                        ← this file
├── BUILD                             ← `gradle :app:assembleDebug` (build-tool entry)
├── .gitignore                        ← .gradle/, .gradle-out/, build/, .cxx/, etc.
├── settings.gradle.kts               ← single :app module
├── build.gradle.kts                  ← project-level plugin versions
├── gradle.properties                 ← android.useAndroidX=true + JVM args
├── local.properties.example          ← SDK path template
└── app/
    ├── build.gradle.kts              ← com.android.application + kotlin-android + compose
    ├── src/main/AndroidManifest.xml  ← single launcher Activity
    ├── src/main/res/values/themes.xml← Theme.VisiCalc (dark, no action bar)
    └── src/main/java/com/example/visicalc/
        ├── MainActivity.kt            ← `setContent { VisiCalcApp() }`
        ├── FormulaBar.kt              ← hand-written composable (shared with desktop)
        └── Grid.kt                    ← hand-written composable (shared with desktop)
```
