// build.gradle.kts (root) — project-level build script.
//
// The root build script is intentionally minimal. It just declares plugins
// that apply to ALL modules but does not configure any of them.
// Module-specific configuration lives in app/build.gradle.kts.
//
// The `alias(libs.plugins.*)` syntax references entries in
// gradle/libs.versions.toml — the single source of truth for versions.

plugins {
    alias(libs.plugins.android.application) apply false
    alias(libs.plugins.kotlin.android) apply false
    alias(libs.plugins.kotlin.compose) apply false
}
