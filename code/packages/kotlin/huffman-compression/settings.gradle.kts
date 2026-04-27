rootProject.name = "huffman-compression"

// Composite build: pull in the kotlin/huffman-tree sibling package so that
// `com.codingadventures:huffman-tree` resolves without Maven publishing.
includeBuild("../huffman-tree")
