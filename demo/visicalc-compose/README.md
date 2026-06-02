# VisiCalc — Compose for Desktop demo

Sixth cross-backend visual demo (Phase 2 / VC2-compose), running on
[Compose Multiplatform for Desktop](https://www.jetbrains.com/lp/compose-multiplatform/).

## What it shows

A `Window` (from `androidx.compose.ui.window`) containing:

- A hand-written **`FormulaBar`** composable
  (`src/main/kotlin/FormulaBar.kt`) — placeholder for the eventual
  `mosaic-compile --backend compose` output.
- A hand-written **`Grid`** composable (`src/main/kotlin/Grid.kt`),
  visually matching what the eventual Compose Grid emitter should
  produce.

Tap a cell — the formula bar updates with its value, the selected
cell gets the excel-blue highlight. Type in the formula bar — it
updates the local `mutableStateOf`.

5×5 sample spreadsheet hard-coded in `src/main/kotlin/Main.kt`'s
`sampleRows`, matching the data in every other VisiCalc demo so all
seven look visually identical.

## What this demo does NOT yet do

- **No `mosaic-compile --backend compose`** exists in the repo.
  Everything in `src/main/kotlin/` is hand-written.  When the
  Compose emitter lands, `FormulaBar.kt` will be replaced by
  `src/main/kotlin/generated/FormulaBar.kt` and this demo will
  shift to a half-generated, half-hand-written shape like
  `demo/visicalc-swiftui` does today.
- **No strict-Flux dispatch yet.**  Local state via
  `remember { mutableStateOf(...) }` for v0.1.0.  When the Compose
  emitter is generating a real `dispatch: (Event) -> Unit`
  parameter, the host will switch to a `MosaicStore<AppState>` from
  the `mosaic-flux-compose` runtime (which is already pulled in as
  an `includeBuild` composite-build dep, just not exercised yet).

## How to run the app

```bash
./gradlew run                  # launches the desktop window
./gradlew packageDistributionForCurrentOS    # native installer
```

Compose Multiplatform handles the JVM + native bundling for macOS
(`.dmg`), Linux (`.deb`), and Windows (`.msi`).

## Prerequisites

- JDK 17+ (Compose 1.6 requires Java 17 minimum).
- Gradle is fetched automatically by `./gradlew`; no system install
  needed.
- macOS / Linux / Windows.  No Android SDK required — Compose
  Multiplatform for Desktop targets the JVM directly.

## Why "Compose for Desktop" rather than Jetpack Compose for Android?

Same `androidx.compose.*` packages, same composable functions, same
`MaterialTheme`.  The runtime API is identical.  We target Desktop
here so the demo runs locally with no emulator and screenshots come
out of a real OS window.  An Android variant is a straight port:
swap `WindowGroup`/`Window` for an `Activity` + `setContent`, the
composables themselves are unchanged.

## File tree

```
demo/visicalc-compose/
├── README.md                     ← this file
├── BUILD                          ← `./gradlew run` from the build-tool
├── .gitignore                     ← .gradle/, .gradle-out/, build/, etc.
├── settings.gradle.kts            ← includes ../../code/packages/kotlin/mosaic-flux-compose
├── build.gradle.kts               ← kotlin("jvm") + org.jetbrains.compose plugin
├── scripts/
│   └── build.sh                   ← stub for future mosaic-compile --backend compose
└── src/main/kotlin/
    ├── Main.kt                    ← `application { Window { ... } }`
    ├── FormulaBar.kt              ← hand-written FormulaBar composable
    └── Grid.kt                    ← hand-written Grid composable
```
