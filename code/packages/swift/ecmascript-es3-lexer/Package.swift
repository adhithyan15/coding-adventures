// swift-tools-version: 6.0
// ============================================================================
// Package.swift -- EcmascriptES3Lexer
// ============================================================================
//
// ECMAScript 3 (1999) lexer for Swift. Adds strict equality (===, !==),
// try/catch/finally/throw, instanceof, and regex literals over ES1.
// ============================================================================

import PackageDescription

let package = Package(
    name: "EcmascriptES3Lexer",
    products: [
        .library(
            name: "EcmascriptES3Lexer",
            targets: ["EcmascriptES3Lexer"]
        ),
    ],
    dependencies: [
        .package(path: "../grammar-tools"),
        .package(path: "../lexer"),
    ],
    targets: [
        .target(
            name: "EcmascriptES3Lexer",
            dependencies: [
                .product(name: "GrammarTools", package: "grammar-tools"),
                .product(name: "Lexer", package: "lexer"),
            ]
        ),
        .testTarget(
            name: "EcmascriptES3LexerTests",
            dependencies: ["EcmascriptES3Lexer"]
        ),
    ]
)
