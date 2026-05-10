rootProject.name = "data-matrix"

// No local composite-build dependencies are needed here because the Data Matrix
// encoder carries its own self-contained GF(256)/0x12D tables and RS computation.
// If you later want to wire in the shared barcode-2d / paint-instructions types,
// add the following includeBuild() lines and update build.gradle.kts accordingly:
//
// includeBuild("../paint-instructions")
// includeBuild("../barcode-2d")
