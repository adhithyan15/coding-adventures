// settings.gradle.kts — composite build declarations for mini-sqlite Level 1.
//
// We include all five pipeline packages as composite builds so Gradle's
// dependency resolution can substitute project references for the JAR files
// produced by those sibling packages.
//
// The build-tool's dep-graph validator requires every `(cd ../X && gradle jar)`
// line in BUILD to appear as an includeBuild here.
includeBuild("../sql-backend")
includeBuild("../sql-planner")
includeBuild("../sql-optimizer")
includeBuild("../sql-codegen")
includeBuild("../sql-vm")
rootProject.name = "mini-sqlite"
