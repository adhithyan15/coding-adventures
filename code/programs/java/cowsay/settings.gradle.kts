rootProject.name = "cowsay"

// Pull in the local packages this program depends on as composite builds.
// See code/packages/java/barcode-2d/settings.gradle.kts for the full
// explanation of how this works. Every transitive local dependency is
// listed explicitly here rather than relying on nested composite-build
// resolution.
includeBuild("../../../packages/java/cli-builder")
includeBuild("../../../packages/java/paint-instructions")
includeBuild("../../../packages/java/paint-vm-ascii")
