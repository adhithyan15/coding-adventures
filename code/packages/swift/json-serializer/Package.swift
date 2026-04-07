// swift-tools-version: 6.0
// ============================================================================
// Package.swift -- JsonSerializer
// ============================================================================
//
// Swift Package Manager manifest for the JsonSerializer library.
//
// A serializer converts an in-memory value into a text representation.
// JsonSerializer is the final stage of the JSON pipeline: it converts a
// JsonValue back into a JSON string (compact or pretty-printed).
//
// It also provides `deserialize(_:)` as a convenience wrapper around
// JsonParser, making this package a complete bidirectional codec.
//
// Dependency chain:
//   json-value ← json-lexer ← json-parser ← json-serializer ← you are here
//
// Part of the coding-adventures educational computing stack.
//
// ============================================================================
import PackageDescription

let package = Package(
    name: "JsonSerializer",
    products: [
        .library(name: "JsonSerializer", targets: ["JsonSerializer"]),
    ],
    dependencies: [
        .package(path: "../json-value"),
        .package(path: "../json-lexer"),
        .package(path: "../json-parser"),
    ],
    targets: [
        .target(
            name: "JsonSerializer",
            dependencies: [
                .product(name: "JsonValue", package: "json-value"),
                .product(name: "JsonLexer", package: "json-lexer"),
                .product(name: "JsonParser", package: "json-parser"),
            ]
        ),
        .testTarget(
            name: "JsonSerializerTests",
            dependencies: [
                "JsonSerializer",
                .product(name: "JsonValue", package: "json-value"),
            ]
        ),
    ]
)
