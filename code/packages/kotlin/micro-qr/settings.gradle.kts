rootProject.name = "micro-qr"

// Composite builds do not inherit sibling includeBuild declarations. Include
// the two direct dependencies and barcode-2d's local paint-instructions
// prerequisite so this package resolves entirely from repository sources.
includeBuild("../gf256")
includeBuild("../paint-instructions")
includeBuild("../barcode-2d")
