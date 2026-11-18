# Use Gradle project properties for Kotlin compiler execution policy

The Kotlin build-tool CI command passed
`-Dkotlin.compiler.execution.strategy=in-process` and
`-Dkotlin.incremental=false`, but those are JVM system properties. Kotlin
Gradle Plugin 2.1.20 reads these options through Gradle's project-property
service, which does not fall back to `System.getProperty`.

On PR #19644 Windows job `135065756220`, Gradle printed `BUILD SUCCESSFUL`
after Kotlin tests and JaCoCo. The Go executor then reported that a process
kept the captured output pipe open beyond its existing five-second wait. This
establishes an executor pipe-wait failure after Gradle's successful exit; it
does not identify the pipe owner or prove a single root cause.

With the previous `-D` flags, a fresh local Gradle 9.4.1 `--info` compile
reported daemon strategy and incremental compilation. With the package-local
`gradle.properties` containing
`kotlin.compiler.execution.strategy=in-process` and `kotlin.incremental=false`,
fresh production and test compiles both reported `IN_PROCESS`, including the
workflow-shaped `-p` invocation with the old `-D` flags still present. A clean
build through the actual Go executor on local JDK 21 captured successful
Kotlin tests and JaCoCo tasks, returned BUILT within the unchanged five-second
post-exit wait, and measured 98.98% line coverage (3 tests, 0 failures). CI
runs Gradle 9.8.2, so its exact-head build remains the platform check. This
local pass does not identify the process that owned the failed CI pipe. Keep
the executor's five-second post-exit wait and verify the package through the
real executor, not just Gradle's shell exit status.

The package-local file covers Gradle started from the package directory and
CI's `-p code/programs/kotlin/build-tool` invocation. Avoid changing global
Gradle state or stopping unrelated daemons to work around a package setting.
