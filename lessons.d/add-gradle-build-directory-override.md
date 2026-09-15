---
category: Workspace & package metadata
---

# Add `gradle-build` directory override

in every Java/Kotlin `build.gradle.kts`: `layout.buildDirectory = file("gradle-build")` BEFORE the plugins block. Gradle's default `build/` collides with the `BUILD` file on case-insensitive filesystems (macOS/Windows) and explodes with `Could not create problems-report directory`. Also: don't pin `java { toolchain { languageVersion } }` — let Gradle use the running JDK so CI's `actions/setup-java` is honored.
