rootProject.name = "reed-solomon"

// Pull in the local gf256 and polynomial packages as composite builds.
// This avoids publishing to a local Maven repository during development.
includeBuild("../gf256")
includeBuild("../polynomial")
