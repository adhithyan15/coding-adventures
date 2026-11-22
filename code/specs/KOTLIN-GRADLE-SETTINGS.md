# Kotlin build-tool Gradle settings

The Kotlin build-tool project pins its compiler execution policy in its
project-local `gradle.properties` file:

```properties
kotlin.compiler.execution.strategy=in-process
kotlin.incremental=false
```

These settings apply when Gradle starts from the package directory and when a
caller selects the project with `-p`. The Unix and Windows package recipes and
the CI validation command all use one of those forms, so they share the same
compiler policy without relying on caller-specific environment state.

The Kotlin Gradle Plugin 2.1.20 reads these values from Gradle project
properties. JVM system properties supplied with `-D` do not configure these
settings. A diagnostic with the previous `-D` arguments showed daemon-backed,
incremental compilation; a fresh compile with this project configuration
reported `IN_PROCESS` for both production and test Kotlin compile tasks.

This setting addresses the selected compilation strategy and incremental
mode. It does not establish which child process retained the CI output pipe,
and it does not change the Go executor's five-second post-exit pipe wait.
