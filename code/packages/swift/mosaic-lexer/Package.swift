// swift-tools-version: 5.9
// ============================================================================
// Package.swift — Tokenizes .mosaic source using the grammar-driven lexer
// ============================================================================
//
// This is the Swift Package Manager manifest for this package.
// It is part of the coding-adventures project, an educational computing stack
// built from logic gates up through interpreters and compilers.
//
// Local monorepo dependencies are declared via relative path references so
// that SPM resolves them from the local filesystem.
//
import PackageDescription

let package = Package(
    name: "mosaic-lexer",
    products: [
        .library(name: "MosaicLexer", targets: ["MosaicLexer"]),
    ],
    dependencies: [
        .package(path: "../grammar-tools"),
        .package(path: "../lexer"),
        .package(path: "../directed-graph"),
    ],
    targets: [
        .target(
            name: "MosaicLexer",
            dependencies: [
                .product(name: "GrammarTools", package: "grammar-tools"),
                .product(name: "Lexer", package: "lexer"),
                .product(name: "DirectedGraph", package: "directed-graph"),
            ]
        ),
        .testTarget(
            name: "MosaicLexerTests",
            dependencies: ["MosaicLexer"]
        ),
    ]
)
