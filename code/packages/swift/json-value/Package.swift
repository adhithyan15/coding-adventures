// swift-tools-version: 5.9
// ============================================================================
// Package.swift — Converts json-parser ASTs into typed JsonValue objects and native Python types
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
    name: "json-value",
    products: [
        .library(name: "JsonValue", targets: ["JsonValue"]),
    ],
    dependencies: [
        .package(path: "../directed-graph"),
        .package(path: "../grammar-tools"),
        .package(path: "../json-lexer"),
        .package(path: "../json-parser"),
        .package(path: "../lexer"),
        .package(path: "../parser"),
        .package(path: "../state-machine"),
    ],
    targets: [
        .target(
            name: "JsonValue",
            dependencies: [
                .product(name: "DirectedGraph", package: "directed-graph"),
                .product(name: "GrammarTools", package: "grammar-tools"),
                .product(name: "JsonLexer", package: "json-lexer"),
                .product(name: "JsonParser", package: "json-parser"),
                .product(name: "Lexer", package: "lexer"),
                .product(name: "Parser", package: "parser"),
                .product(name: "StateMachine", package: "state-machine"),
            ]
        ),
        .testTarget(
            name: "JsonValueTests",
            dependencies: ["JsonValue"]
        ),
    ]
)
