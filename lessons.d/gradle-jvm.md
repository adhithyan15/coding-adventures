# Gradle / JVM

### Gradle optimizer packages need `includeBuild` for the build-tool's dep graph

When a Gradle package (Java or Kotlin) depends on a sibling package via a file-based JAR
(`implementation(files("../sql-planner/gradle-build/libs/..."))`), the build tool's
`parseGradleDeps` function reads `includeBuild(...)` lines from `settings.gradle.kts` to
construct the dependency graph. Without this, the validator raises:

  `<lang>/sql-optimizer (BUILD): undeclared local package refs: <lang>/sql-planner`

Fix: Add `includeBuild("../sql-planner")` to the optimizer's `settings.gradle.kts` BEFORE
the `rootProject.name` line. This applies to Java and Kotlin optimizer packages (and
any future Gradle packages that depend on siblings via file deps).

Discovered: 2026-06-30 during Java sql-optimizer CI (PR #7073 fix commit d31296a48).
