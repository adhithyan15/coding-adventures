rootProject.name = "coding-adventures-sql-optimizer"

// Declare the local build dependency on sql-planner so the monorepo build tool
// can construct the correct dependency graph edge (java/sql-optimizer →
// java/sql-planner) and satisfy the BUILD file validator that checks all
// relative path references in BUILD commands are declared predecessors.
includeBuild("../sql-planner")
