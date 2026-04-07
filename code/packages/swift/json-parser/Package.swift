// swift-tools-version: 6.0
// ============================================================================
// Package.swift -- JsonParser
// ============================================================================
//
// Swift Package Manager manifest for the JsonParser library.
//
// A parser converts a token stream into a structured value. JsonParser is the
// second stage of the JSON pipeline: it takes the flat `[Token]` array from
// JsonLexer and builds a `JsonValue` tree.
//
// Dependency chain:
//   json-value  ← json-lexer  ← json-parser ← json-serializer
//
// Part of the coding-adventures educational computing stack.
//
// ============================================================================
import PackageDescription

let package = Package(
    name: "JsonParser",
    products: [
        .library(name: "JsonParser", targets: ["JsonParser"]),
    ],
    dependencies: [
        .package(path: "../json-value"),
        .package(path: "../json-lexer"),
    ],
    targets: [
        .target(
            name: "JsonParser",
            dependencies: [
                .product(name: "JsonValue", package: "json-value"),
                .product(name: "JsonLexer", package: "json-lexer"),
            ]
        ),
        .testTarget(
            name: "JsonParserTests",
            dependencies: [
                "JsonParser",
                .product(name: "JsonValue", package: "json-value"),
                .product(name: "JsonLexer", package: "json-lexer"),
            ]
        ),
    ]
)
