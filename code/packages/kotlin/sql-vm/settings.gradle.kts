// settings.gradle.kts — Gradle composite build settings for sql-vm.
//
// We declare all four upstream sibling packages as "included builds".  This
// lets Gradle resolve their types (SqlValue, Instruction, Program, Backend …)
// at compile time without requiring the JARs to be published to Maven Central.
//
// Composite builds are Gradle's monorepo story: each includeBuild declares
// "this project lives locally at this relative path, please treat it as part
// of the build graph rather than fetching it from a remote repository."
//
// CRITICAL: all four packages must be listed.  Missing one causes an
// "unresolved reference" at compile time on a fresh CI runner.

includeBuild("../sql-backend")
includeBuild("../sql-planner")
includeBuild("../sql-optimizer")
includeBuild("../sql-codegen")

rootProject.name = "coding-adventures-sql-vm"
