// swift-tools-version: 5.9
// ============================================================================
// Package.swift — Validates the Mosaic AST and produces a typed MosaicIR
// ============================================================================
//
// This is the Swift Package Manager manifest for this package.
// It is part of the coding-adventures project, an educational computing stack
// built from logic gates up through interpreters and compilers.
//
import PackageDescription

let package = Package(
    name: "mosaic-analyzer",
    products: [
        .library(name: "MosaicAnalyzer", targets: ["MosaicAnalyzer"]),
    ],
    dependencies: [
        .package(path: "../mosaic-parser"),
        .package(path: "../mosaic-lexer"),
    ],
    targets: [
        .target(
            name: "MosaicAnalyzer",
            dependencies: [
                .product(name: "MosaicParser", package: "mosaic-parser"),
                .product(name: "MosaicLexer", package: "mosaic-lexer"),
            ]
        ),
        .testTarget(
            name: "MosaicAnalyzerTests",
            dependencies: ["MosaicAnalyzer"]
        ),
    ]
)
