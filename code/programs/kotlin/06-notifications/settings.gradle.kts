// settings.gradle.kts — declares the project name and which submodules exist.
//
// Gradle evaluates this file first, before any build.gradle.kts. It sets up
// where to find plugins (pluginManagement) and libraries (dependencyResolutionManagement).
// FAIL_ON_PROJECT_REPOS enforces that all repositories are declared here, not
// scattered across individual module build files — this prevents accidental
// dependency resolution against unexpected mirrors.

pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "06-notifications"
include(":app")
