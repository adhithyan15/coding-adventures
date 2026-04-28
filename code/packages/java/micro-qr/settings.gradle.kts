rootProject.name = "micro-qr"

// Pull in local dependencies via composite builds.
// Gradle sees `api("com.codingadventures:gf256")` in build.gradle.kts and
// looks for an includeBuild that provides that artifact.  The includeBuild
// directive tells Gradle to build the sibling package locally and substitute
// it for the Maven artifact rather than downloading from a remote repository.
//
// This avoids publishing to a local Maven repository during development — the
// same mechanism used by all other Java packages in this monorepo.
includeBuild("../gf256")
includeBuild("../barcode-2d")
includeBuild("../paint-instructions")
