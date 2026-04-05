// swift-tools-version: 5.9
// ============================================================================
// Package.swift — Scytale cipher — ancient Spartan transposition cipher
// ============================================================================
//
// This is the Swift Package Manager manifest for this package.
// It is part of the coding-adventures project, an educational computing stack
// built from logic gates up through interpreters and compilers.
//
import PackageDescription

let package = Package(
    name: "scytale-cipher",
    products: [
        .library(name: "ScytaleCipher", targets: ["ScytaleCipher"]),
    ],
    targets: [
        .target(
            name: "ScytaleCipher"
        ),
        .testTarget(
            name: "ScytaleCipherTests",
            dependencies: ["ScytaleCipher"]
        ),
    ]
)
