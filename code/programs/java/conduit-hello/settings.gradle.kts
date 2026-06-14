rootProject.name = "conduit-hello"

// Pull in the Conduit framework package as a composite build so the demo
// compiles against its sources without a published artifact. Gradle
// substitutes the `com.codingadventures:conduit` dependency with this build.
includeBuild("../../../packages/java/conduit")
