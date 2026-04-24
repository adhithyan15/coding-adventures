rootProject.name = "barcode-2d"

// Include the paint-instructions package as a composite build so Gradle can
// resolve the `com.codingadventures:paint-instructions` artifact locally without
// publishing it to a Maven repository.
//
// Composite builds work by substituting a dependency declaration with a local
// project build.  When Gradle sees:
//
//     implementation(project(":paint-instructions"))
//
// in the barcode-2d build.gradle.kts, this includeBuild() directive tells
// Gradle to find it at ../paint-instructions relative to this file.
includeBuild("../paint-instructions")
