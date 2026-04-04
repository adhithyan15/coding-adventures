// swift-tools-version: 5.9
// ============================================================================
// Package.swift — Generic tree walker that drives Mosaic compiler backends
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
    name: "mosaic-vm",
    products: [
        .library(name: "MosaicVm", targets: ["MosaicVm"]),
    ],
    dependencies: [
        .package(path: "../mosaic-analyzer"),
        .package(path: "../mosaic-parser"),
        .package(path: "../mosaic-lexer"),
        .package(path: "../grammar-tools"),
        .package(path: "../lexer"),
        .package(path: "../directed-graph"),
        .package(path: "../parser"),
        .package(path: "../state-machine"),
    ],
    targets: [
        .target(
            name: "MosaicVm",
            dependencies: [
                .product(name: "MosaicAnalyzer", package: "mosaic-analyzer"),
                .product(name: "MosaicParser", package: "mosaic-parser"),
                .product(name: "MosaicLexer", package: "mosaic-lexer"),
                .product(name: "GrammarTools", package: "grammar-tools"),
                .product(name: "Lexer", package: "lexer"),
                .product(name: "DirectedGraph", package: "directed-graph"),
                .product(name: "Parser", package: "parser"),
                .product(name: "StateMachine", package: "state-machine"),
            ]
        ),
        .testTarget(
            name: "MosaicVmTests",
            dependencies: ["MosaicVm"]
        ),
    ]
)
