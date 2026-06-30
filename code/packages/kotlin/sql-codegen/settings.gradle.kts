// settings.gradle.kts — Gradle composite build settings for sql-codegen.
//
// We declare both sql-planner and sql-optimizer as "included builds". This
// tells Gradle's composite-build mechanism that these projects live locally
// rather than being fetched from a remote repository.  Composite builds let
// sibling packages share code without publishing to Maven Central — ideal for
// a monorepo.
//
// CRITICAL: both includeBuild entries must be present.  The codegen depends on
// types from BOTH packages (SqlExpr from sql-planner, OptimizedPlan from
// sql-optimizer).  A missing includeBuild causes "unresolved reference" at
// compile time on a fresh CI runner where neither JAR has been pre-built.

includeBuild("../sql-planner")
includeBuild("../sql-optimizer")

rootProject.name = "coding-adventures-sql-codegen"
