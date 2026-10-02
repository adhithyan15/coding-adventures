# 02 — Water Counter (Android / Kotlin)

The Android counterpart to the Swift water counter. It is the second app in
the series, after `01-hello-world`, and the first with state that changes.

## What it does

The app tracks how much water you have drunk today against a 2,000 ml daily
goal:
- Each tap of the button adds a 250 ml serving.
- A progress bar fills toward the goal.
- When the total reaches the goal, the drop icon and the colors change and a
  "goal reached" message appears.
- **Reset** sets the total back to zero.

## What it teaches

- Compose state: `remember { mutableIntStateOf(0) }` keeps a value across
  recompositions, like SwiftUI's `@State`.
- Property delegation with `by`, which lets you write `total` instead of
  `total.value`.
- Recomposition: any composable that reads the state re-runs when it changes.
- Animated transitions with `AnimatedContent`, `animateColorAsState` and
  `animateFloatAsState`.

## Building

```sh
./gradlew assembleDebug
```

This is the same command the package's `BUILD` runs. The Gradle wrapper
downloads the Gradle 8.11.1 distribution and verifies it against
`distributionSha256Sum` in `gradle/wrapper/gradle-wrapper.properties`.
