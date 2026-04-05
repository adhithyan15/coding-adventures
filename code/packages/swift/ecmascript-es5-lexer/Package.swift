// swift-tools-version: 6.0
// ============================================================================
// Package.swift -- EcmascriptES5Lexer
// ============================================================================
//
// ECMAScript 5 (2009) lexer for Swift. Adds the `debugger` keyword over ES3.
// Syntactic changes are modest; the real ES5 innovations were strict mode
// semantics, native JSON support, and property descriptors.
// ============================================================================

import PackageDescription

let package = Package(
    name: "EcmascriptES5Lexer",
    products: [
        .library(
            name: "EcmascriptES5Lexer",
            targets: ["EcmascriptES5Lexer"]
        ),
    ],
    dependencies: [
        .package(path: "../grammar-tools"),
        .package(path: "../lexer"),
    ],
    targets: [
        .target(
            name: "EcmascriptES5Lexer",
            dependencies: [
                .product(name: "GrammarTools", package: "grammar-tools"),
                .product(name: "Lexer", package: "lexer"),
            ]
        ),
        .testTarget(
            name: "EcmascriptES5LexerTests",
            dependencies: ["EcmascriptES5Lexer"]
        ),
    ]
)
