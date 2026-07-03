// settings.gradle.kts — Android Gradle project.
//
// Single :app module hosts the Jetpack Compose for Android demo.

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

rootProject.name = "visicalc-android"
include(":app")
