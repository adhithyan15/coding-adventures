rootProject.name = "reed-solomon"

// Include both local builds for composite dependency resolution.
// gf256 must be listed even though polynomial already includes it —
// Gradle composite builds do not transitively expose sub-includes.
includeBuild("../gf256")
includeBuild("../polynomial")
