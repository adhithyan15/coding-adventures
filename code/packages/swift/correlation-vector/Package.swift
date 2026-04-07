// swift-tools-version: 6.0
// ============================================================================
// Package.swift -- CorrelationVector
// ============================================================================
//
// Swift Package Manager manifest for the CorrelationVector library.
//
// A Correlation Vector (CV) is a lightweight, append-only provenance record
// that follows a piece of data through every transformation it undergoes.
// Assign a CV to anything when it is born. Every system, stage, or function
// that touches it appends its contribution.
//
// This package depends on:
//   - sha256:          for deterministic 8-char hex base IDs from origin strings
//   - json-value:      JsonValue enum for typed metadata and serialization
//   - json-serializer: JsonSerializer for encode/decode of the CVLog to JSON
//
// Part of the coding-adventures educational computing stack.
//
// ============================================================================

import PackageDescription

let package = Package(
    name: "CorrelationVector",
    products: [
        .library(name: "CorrelationVector", targets: ["CorrelationVector"]),
    ],
    dependencies: [
        .package(path: "../sha256"),
        .package(path: "../json-value"),
        .package(path: "../json-serializer"),
    ],
    targets: [
        .target(
            name: "CorrelationVector",
            dependencies: [
                .product(name: "SHA256", package: "sha256"),
                .product(name: "JsonValue", package: "json-value"),
                .product(name: "JsonSerializer", package: "json-serializer"),
            ]
        ),
        .testTarget(
            name: "CorrelationVectorTests",
            dependencies: ["CorrelationVector"]
        ),
    ]
)
