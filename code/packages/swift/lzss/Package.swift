// swift-tools-version: 5.9
// ============================================================================
// Package.swift — LZSS lossless compression algorithm (1982) — CMP02
// ============================================================================
//
// This is the Swift Package Manager manifest for this package.
// It is part of the coding-adventures project, an educational computing stack
// built from logic gates up through interpreters and compilers.
//
import PackageDescription

let package = Package(
    name: "lzss",
    products: [
        .library(name: "LZSS", targets: ["LZSS"]),
    ],
    targets: [
        .target(
            name: "LZSS"
        ),
        .testTarget(
            name: "LZSSTests",
            dependencies: ["LZSS"]
        ),
    ]
)
