rootProject.name = "qr-code"

// Include all local sibling packages as composite builds so Gradle can resolve
// `com.codingadventures:*` dependencies locally without publishing to Maven.
//
// Composite builds work by substituting a dependency declaration with a local
// project build.  When Gradle sees `implementation("com.codingadventures:barcode-2d")`
// in build.gradle.kts, the matching includeBuild() here tells Gradle to find it
// at the given relative path.
//
// Rule: every package in this `dependencies` block must be listed here,
// PLUS any transitive local deps that Gradle cannot follow automatically.
// Gradle composite builds do NOT transitively expose sub-includes.
includeBuild("../gf256")
includeBuild("../polynomial")
includeBuild("../reed-solomon")
includeBuild("../paint-instructions")
includeBuild("../barcode-2d")
