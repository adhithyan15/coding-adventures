// swift-tools-version: 5.9
// ============================================================================
// Package.swift — Brotli lossless compression — CMP06
// ============================================================================
//
// This is the Swift Package Manager manifest for this package.
// It is part of the coding-adventures project, an educational computing stack
// built from logic gates up through interpreters and compilers.
//
import PackageDescription

let package = Package(
    name: "brotli",
    products: [
        .library(name: "CodingAdventuresBrotli", targets: ["CodingAdventuresBrotli"]),
    ],
    dependencies: [
        .package(path: "../huffman-tree"),
    ],
    targets: [
        .target(
            name: "CodingAdventuresBrotli",
            dependencies: [
                .product(name: "HuffmanTree", package: "huffman-tree"),
            ],
            path: "Sources/CodingAdventuresBrotli"
        ),
        .testTarget(
            name: "BrotliTests",
            dependencies: ["CodingAdventuresBrotli"],
            path: "Tests/BrotliTests"
        ),
    ]
)
