// swift-tools-version: 5.9
// ============================================================================
// Package.swift — DEFLATE lossless compression — CMP05
// ============================================================================
//
// This is the Swift Package Manager manifest for this package.
// It is part of the coding-adventures project, an educational computing stack
// built from logic gates up through interpreters and compilers.
//
import PackageDescription

let package = Package(
    name: "deflate",
    products: [
        .library(name: "Deflate", targets: ["Deflate"]),
    ],
    dependencies: [
        .package(path: "../huffman-tree"),
        .package(path: "../lzss"),
    ],
    targets: [
        .target(
            name: "Deflate",
            dependencies: [
                .product(name: "HuffmanTree", package: "huffman-tree"),
                .product(name: "LZSS", package: "lzss"),
            ],
            path: "Sources/Deflate"
        ),
        .testTarget(
            name: "DeflateTests",
            dependencies: ["Deflate"],
            path: "Tests/DeflateTests"
        ),
    ]
)
