// settings.gradle.kts — composite build declarations for sql-vm.
//
// We include sql-planner, sql-optimizer, and sql-codegen as composite builds
// so Gradle's dependency resolution can substitute project references for
// the JAR files produced by those sibling packages.
//
// The build-tool's dep-graph validator requires every `(cd ../X && gradle jar)`
// line in BUILD to appear as an includeBuild here.
includeBuild("../sql-planner")
includeBuild("../sql-optimizer")
includeBuild("../sql-codegen")
rootProject.name = "coding-adventures-sql-vm"
