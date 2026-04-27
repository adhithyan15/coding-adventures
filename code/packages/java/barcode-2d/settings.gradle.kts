rootProject.name = "barcode-2d"

// Pull in the local paint-instructions package as a composite build.
// This avoids publishing to a local Maven repository during development.
//
// How composite builds work:
//   Gradle sees `api("com.codingadventures:paint-instructions")` in
//   build.gradle.kts and looks for an includeBuild that provides that
//   artifact.  The includeBuild here tells Gradle to build
//   ../paint-instructions locally and substitute it for the Maven artifact.
//
// This is the same mechanism used by Kotlin's barcode-2d package.
includeBuild("../paint-instructions")
