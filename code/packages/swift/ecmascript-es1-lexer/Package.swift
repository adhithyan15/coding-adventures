// swift-tools-version: 6.0
// ============================================================================
// Package.swift -- EcmascriptES1Lexer
// ============================================================================
//
// ECMAScript 1 (1997) lexer for Swift. A thin wrapper around the grammar-driven
// GrammarLexer from the Lexer package, configured by ecmascript/es1.tokens.
//
// ES1 is the first standardized JavaScript. It has 23 keywords, basic operators
// (no === or !==), string/number literals, and no regex literals.
// ============================================================================

import PackageDescription

let package = Package(
    name: "EcmascriptES1Lexer",
    products: [
        .library(
            name: "EcmascriptES1Lexer",
            targets: ["EcmascriptES1Lexer"]
        ),
    ],
    dependencies: [
        .package(path: "../grammar-tools"),
        .package(path: "../lexer"),
    ],
    targets: [
        .target(
            name: "EcmascriptES1Lexer",
            dependencies: [
                .product(name: "GrammarTools", package: "grammar-tools"),
                .product(name: "Lexer", package: "lexer"),
            ]
        ),
        .testTarget(
            name: "EcmascriptES1LexerTests",
            dependencies: ["EcmascriptES1Lexer"]
        ),
    ]
)
