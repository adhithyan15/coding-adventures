## Unreleased — HOS-003b Dart and Kotlin fixture routing

- Route the canonical historical Unix file-read fixture to the Dart and Kotlin
  reusable library packages.
- Hash only the exact external Dart adapter, its shared replay helper, and the
  external Kotlin Gradle test adapter for those package consumers.
- Extend platform, language-filter, rename, and exact repository-input hash
  tests to cover both new lanes without selecting `build_native.py`.
