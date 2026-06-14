rootProject.name = "conduit-kotlin"

// Reuse the Java Conduit package (and its native cdylib) as a composite build.
// Gradle substitutes the `com.codingadventures:conduit` dependency with it.
includeBuild("../../java/conduit")
