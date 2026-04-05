// swift-tools-version: 5.9
// ============================================================================
// Package.swift — Hand-written recursive-descent parser for the Mosaic language
// ============================================================================
//
// This is the Swift Package Manager manifest for this package.
// It is part of the coding-adventures project, an educational computing stack
// built from logic gates up through interpreters and compilers.
//
import PackageDescription

let package = Package(
    name: "mosaic-parser",
    products: [
        .library(name: "MosaicParser", targets: ["MosaicParser"]),
    ],
    dependencies: [
        .package(path: "../mosaic-lexer"),
    ],
    targets: [
        .target(
            name: "MosaicParser",
            dependencies: [
                .product(name: "MosaicLexer", package: "mosaic-lexer"),
            ]
        ),
        .testTarget(
            name: "MosaicParserTests",
            dependencies: ["MosaicParser"]
        ),
    ]
)
