// swift-tools-version: 6.0
// ============================================================================
// Package.swift -- JsonLexer
// ============================================================================
//
// Swift Package Manager manifest for the JsonLexer library.
//
// A lexer (also called a tokenizer or scanner) converts raw text into a
// sequence of typed tokens. JsonLexer is the first stage of the JSON parsing
// pipeline: it breaks JSON text into tokens that the parser can reason about.
//
// Dependency chain:
//   json-value  ← json-lexer  ← json-parser ← json-serializer
//
// Part of the coding-adventures educational computing stack.
//
// ============================================================================
import PackageDescription

let package = Package(
    name: "JsonLexer",
    products: [
        .library(name: "JsonLexer", targets: ["JsonLexer"]),
    ],
    dependencies: [
        .package(path: "../json-value"),
    ],
    targets: [
        .target(
            name: "JsonLexer",
            dependencies: [
                .product(name: "JsonValue", package: "json-value"),
            ]
        ),
        .testTarget(
            name: "JsonLexerTests",
            dependencies: ["JsonLexer"]
        ),
    ]
)
