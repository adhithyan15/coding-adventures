rootProject.name = "polynomial"

// Pull in the local gf256 package as a composite build.
// This avoids publishing to a local Maven repository during development.
includeBuild("../gf256")
