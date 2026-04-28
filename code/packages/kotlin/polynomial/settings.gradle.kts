rootProject.name = "polynomial"

// Include the local gf256 build so Gradle can resolve com.codingadventures:gf256
// without publishing to a Maven repository.  This is the same pattern used by
// kotlin/directed-graph → kotlin/graph.
includeBuild("../gf256")
