// swift-tools-version: 6.0
// ============================================================================
// Package.swift -- JsonValue
// ============================================================================
//
// Swift Package Manager manifest for the JsonValue library.
//
// JsonValue provides the core algebraic data type for representing any JSON
// value as a Swift enum. It is the foundational type used by all other
// packages in the JSON pipeline (json-lexer, json-parser, json-serializer).
//
// This package has no external dependencies — it is a pure value type library.
//
// Part of the coding-adventures educational computing stack.
//
// ============================================================================
import PackageDescription

let package = Package(
    name: "JsonValue",
    products: [
        .library(name: "JsonValue", targets: ["JsonValue"]),
    ],
    targets: [
        .target(name: "JsonValue"),
        .testTarget(name: "JsonValueTests", dependencies: ["JsonValue"]),
    ]
)
