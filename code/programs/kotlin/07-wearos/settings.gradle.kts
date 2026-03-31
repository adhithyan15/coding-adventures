// settings.gradle.kts — project-level configuration.
//
// Gradle reads this file first, before any build.gradle.kts.
// It defines:
//   1. Where to find plugins (pluginManagement).
//   2. Where to find library dependencies (dependencyResolutionManagement).
//   3. The root project name — should match the directory name.
//   4. Which modules (sub-projects) to include.
//
// For a WearOS standalone app there is only one module: :app.
// A companion phone module is NOT included — this app is fully self-contained.

pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    // FAIL_ON_PROJECT_REPOS: prevents individual modules from adding their own
    // repos, keeping dependency resolution centralised and predictable.
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "07-wearos"
include(":app")
