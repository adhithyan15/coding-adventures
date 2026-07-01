// build.gradle.kts (project-level) — version catalog of plugin
// versions for the :app module to reference.  No tasks declared at
// this level; the :app module owns the build pipeline.
plugins {
    id("com.android.application") version "8.5.2" apply false
    kotlin("android") version "2.0.0" apply false
    id("org.jetbrains.kotlin.plugin.compose") version "2.0.0" apply false
}

// Redirect Gradle's output directory away from `build/` because the
// repo's case-insensitive filesystem (macOS HFS+) treats `build` and
// the required `BUILD` script as the same name.  Same trap that bit
// mosaic-flux-compose and visicalc-compose (see lessons.md).
layout.buildDirectory.set(file(".gradle-out"))
