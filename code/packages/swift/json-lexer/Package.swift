// swift-tools-version: 5.9
// ============================================================================
// Package.swift — Tokenizes JSON text using the grammar-driven lexer — a thin wrapper that loads json.tokens
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
    name: "json-lexer",
    products: [
        .library(name: "JsonLexer", targets: ["JsonLexer"]),
    ],
    dependencies: [
        .package(path: "../directed-graph"),
        .package(path: "../grammar-tools"),
        .package(path: "../lexer"),
        .package(path: "../state-machine"),
    ],
    targets: [
        .target(
            name: "JsonLexer",
            dependencies: [
                .product(name: "DirectedGraph", package: "directed-graph"),
                .product(name: "GrammarTools", package: "grammar-tools"),
                .product(name: "Lexer", package: "lexer"),
                .product(name: "StateMachine", package: "state-machine"),
            ]
        ),
        .testTarget(
            name: "JsonLexerTests",
            dependencies: ["JsonLexer"]
        ),
    ]
)
