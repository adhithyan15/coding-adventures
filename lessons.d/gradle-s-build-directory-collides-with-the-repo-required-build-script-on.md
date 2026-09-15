---
category: Repo policy / workflow reminders
---

# Gradle's `build/` directory collides with the repo-required `BUILD` script on case-insensitive macOS HFS+

Two failure modes: (1) `gradle test` fails to create `build/reports/problems/` because the filesystem already has `BUILD` at the same name; (2) `rm -rf build` (cleaning gradle's output) ALSO deletes the `BUILD` script — silent data loss. **Fix in `build.gradle.kts`**: `layout.buildDirectory.set(file(".gradle-out"))`. Add `.gradle-out/` to the package's `.gitignore` alongside `.gradle/`. Document the redirect in the README so other contributors don't undo it. Applies to every new Kotlin/Gradle package added under `code/packages/kotlin/`. Same logic would apply to any other Gradle-using language we add (Java, Scala, etc.) if their convention is also lowercase `build/`. **NEVER `rm -rf build` in a repo with a `BUILD` script** — use `rm -rf .gradle-out` (or whatever you redirected to).
