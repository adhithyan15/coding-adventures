// Declare the local build dependency on sql-planner and sql-optimizer so the
// monorepo build tool can construct the correct dependency graph edges:
//   java/sql-codegen → java/sql-optimizer → java/sql-planner
// The build-tool's dep-graph validator checks that all relative path references
// in BUILD commands are declared predecessors (lesson learned from PR #7073).
includeBuild("../sql-planner")
includeBuild("../sql-optimizer")
rootProject.name = "coding-adventures-sql-codegen"
