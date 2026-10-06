# Changelog

## Unreleased

- **Gradle wrapper pins its download.** `gradle-wrapper.properties` gains
  `distributionSha256Sum` (gradle-8.11.1-bin.zip), Gradle's published checksum for the zip it
  names, so the wrapper refuses a download that does not match.

## [1.0.0] - 2026-03-29

### Added
- `MainActivity.kt` — entry point with Jetpack Compose UI
- `Theme.kt` — Material Design 3 theme with dynamic colour support
- `AndroidManifest.xml` — app identity and component declarations
- Gradle build files with version catalogue (`libs.versions.toml`)
- README with Swift ↔ Kotlin comparison table
