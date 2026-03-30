// Root build.gradle.kts — the top-level build script.
//
// This file applies plugins to ALL subprojects (here just :app) but defers
// actual configuration to each module's own build.gradle.kts.
// `apply false` means "declare the plugin is available but don't activate it
// at the root level" — each module opts in explicitly.

plugins {
    alias(libs.plugins.android.application) apply false
    alias(libs.plugins.kotlin.android) apply false
    alias(libs.plugins.kotlin.compose) apply false
}
