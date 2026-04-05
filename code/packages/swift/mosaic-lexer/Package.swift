// swift-tools-version: 5.9
// ============================================================================
// Package.swift — Hand-written lexer for the Mosaic component description language
// ============================================================================
//
// This is the Swift Package Manager manifest for this package.
// It is part of the coding-adventures project, an educational computing stack
// built from logic gates up through interpreters and compilers.
//
import PackageDescription

let package = Package(
    name: "mosaic-lexer",
    products: [
        .library(name: "MosaicLexer", targets: ["MosaicLexer"]),
    ],
    targets: [
        .target(name: "MosaicLexer"),
        .testTarget(
            name: "MosaicLexerTests",
            dependencies: ["MosaicLexer"]
        ),
    ]
)
