# 01 — Hello World (Android / Kotlin)

The Android counterpart to the Swift Hello World. Same idea, different
platform — a single screen with a water drop icon and "Hello, World!".

## What it does

Displays a water drop icon, "Hello, World!" heading, and a subtitle
using Jetpack Compose and Material Design 3.

## What it teaches

- Android project structure (manifest, modules, Gradle)
- Jetpack Compose basics: `@Composable`, `Column`, `Text`, `Icon`
- Material Design 3 theming with dynamic colour
- How Compose compares to SwiftUI (spoiler: very similar)
- How to build with Gradle on the command line

## Swift ↔ Kotlin / SwiftUI ↔ Compose comparison

| SwiftUI (iOS)             | Jetpack Compose (Android)         |
|---------------------------|-----------------------------------|
| `struct MyView: View`     | `@Composable fun MyScreen()`      |
| `var body: some View`     | function body is the UI           |
| `VStack { }`              | `Column { }`                      |
| `HStack { }`              | `Row { }`                         |
| `Text("hello")`           | `Text("hello")`                   |
| `Image(systemName:)`      | `Icon(imageVector:)`              |
| `.foregroundStyle(.blue)` | `tint = MaterialTheme.colorScheme.primary` |
| `#Preview`                | `@Preview`                        |

## Running locally

```bash
cd code/programs/kotlin/01-hello-world

# Build debug APK
export JAVA_HOME="/Applications/Android Studio.app/Contents/jbr/Contents/Home"
./gradlew assembleDebug

# Install and run on a connected device or running emulator
./gradlew installDebug
```

Or open the folder in Android Studio and press Run (⇧F10 / ▶).

## Project structure

```
01-hello-world/
├── gradle/
│   ├── libs.versions.toml     version catalogue (all deps in one place)
│   └── wrapper/               Gradle wrapper (pins the Gradle version)
├── app/
│   ├── build.gradle.kts       app module build config
│   └── src/main/
│       ├── AndroidManifest.xml
│       ├── java/com/codingadventures/helloworld/
│       │   ├── MainActivity.kt    entry point + Compose UI
│       │   └── Theme.kt           Material 3 theme
│       └── res/values/
│           └── themes.xml
├── build.gradle.kts           root build config
└── settings.gradle.kts        module declarations
```

## Part of the Foveo series

This app is the Android foundation of [Foveo](../../foveo/), a self-care
companion app. Each app in this series introduces one layer of complexity.
