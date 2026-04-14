// swift-tools-version: 5.9
// ============================================================================
// Package.swift — Huffman Tree optimal prefix-free entropy coding — DT27
// ============================================================================
//
// This is the Swift Package Manager manifest for this package.
// It is part of the coding-adventures project, an educational computing stack
// built from logic gates up through interpreters and compilers.
//
import PackageDescription

let package = Package(
    name: "huffman-tree",
    products: [
        .library(name: "HuffmanTree", targets: ["HuffmanTree"]),
    ],
    dependencies: [
        .package(path: "../heap"),
    ],
    targets: [
        .target(
            name: "HuffmanTree",
            dependencies: [
                .product(name: "Heap", package: "heap"),
            ],
            path: "Sources/HuffmanTree"
        ),
        .testTarget(
            name: "HuffmanTreeTests",
            dependencies: ["HuffmanTree"],
            path: "Tests/HuffmanTreeTests"
        ),
    ]
)
