rootProject.name = "test-bootstrap-kt"

// The Kotlin package is a binding over the Java PBT engine (VER05f): it reuses
// the engine as a composite build rather than carrying a second one. Gradle
// substitutes the `com.codingadventures:test-bootstrap` dependency with it, and
// the repo build tool reads this line to learn the kotlin -> java dependency.
includeBuild("../../java/test-bootstrap")
