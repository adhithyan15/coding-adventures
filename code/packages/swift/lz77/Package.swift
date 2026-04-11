// swift-tools-version: 5.9
// ============================================================================
// Package.swift — LZ77 lossless compression algorithm (1977) — CMP00
// ============================================================================
//
// This is the Swift Package Manager manifest for this package.
// It is part of the coding-adventures project, an educational computing stack
// built from logic gates up through interpreters and compilers.
//
import PackageDescription

let package = Package(
    name: "lz77",
    products: [
        .library(name: "LZ77", targets: ["LZ77"]),
    ],
    targets: [
        .target(
            name: "LZ77"
        ),
        .testTarget(
            name: "LZ77Tests",
            dependencies: ["LZ77"]
        ),
    ]
)
