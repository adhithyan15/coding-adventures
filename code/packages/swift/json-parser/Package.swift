// swift-tools-version: 5.9
// ============================================================================
// Package.swift — Parses JSON text into ASTs using the grammar-driven parser — a thin wrapper that loads json.grammar
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
    name: "json-parser",
    products: [
        .library(name: "JsonParser", targets: ["JsonParser"]),
    ],
    dependencies: [
        .package(path: "../directed-graph"),
        .package(path: "../grammar-tools"),
        .package(path: "../json-lexer"),
        .package(path: "../lexer"),
        .package(path: "../parser"),
        .package(path: "../state-machine"),
    ],
    targets: [
        .target(
            name: "JsonParser",
            dependencies: [
                .product(name: "DirectedGraph", package: "directed-graph"),
                .product(name: "GrammarTools", package: "grammar-tools"),
                .product(name: "JsonLexer", package: "json-lexer"),
                .product(name: "Lexer", package: "lexer"),
                .product(name: "Parser", package: "parser"),
                .product(name: "StateMachine", package: "state-machine"),
            ]
        ),
        .testTarget(
            name: "JsonParserTests",
            dependencies: ["JsonParser"]
        ),
    ]
)
