rootProject.name = "cowsay"

// Pull in the local packages this program depends on as composite builds.
// See code/packages/kotlin/barcode-2d/settings.gradle.kts for the full
// explanation of how this works. Every transitive local dependency is
// listed explicitly here rather than relying on nested composite-build
// resolution.
includeBuild("../../../packages/kotlin/cli-builder")
includeBuild("../../../packages/kotlin/paint-instructions")
includeBuild("../../../packages/kotlin/paint-vm-ascii")
