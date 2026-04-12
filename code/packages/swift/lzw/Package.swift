// swift-tools-version: 5.9
// ============================================================================
// Package.swift — LZW lossless compression algorithm (1984) — CMP03
// ============================================================================
//
// This is the Swift Package Manager manifest for this package.
// It is part of the coding-adventures project, an educational computing stack
// built from logic gates up through interpreters and compilers.
//
import PackageDescription

let package = Package(
    name: "lzw",
    products: [
        .library(name: "LZW", targets: ["LZW"]),
    ],
    targets: [
        .target(
            name: "LZW",
            path: "Sources/LZW"
        ),
        .testTarget(
            name: "LZWTests",
            dependencies: ["LZW"],
            path: "Tests/LZWTests"
        ),
    ]
)
