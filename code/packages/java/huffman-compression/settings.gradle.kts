rootProject.name = "huffman-compression"

// Composite build: pull in the huffman-tree package as a sibling project so
// that `com.codingadventures:huffman-tree` resolves without Maven publishing.
includeBuild("../huffman-tree")
