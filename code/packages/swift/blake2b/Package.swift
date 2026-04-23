// swift-tools-version: 6.0
// ============================================================================
// Package.swift — BLAKE2b (RFC 7693)
// ============================================================================
//
// Swift Package Manager manifest for the BLAKE2b cryptographic hash function,
// implemented from scratch. Entirely self-contained — only native UInt64
// wrapping arithmetic (&+, >>, <<, ^) is used.
//
// Part of the coding-adventures educational computing stack.
// ============================================================================

import PackageDescription

let package = Package(
    name: "Blake2b",
    products: [
        .library(name: "Blake2b", targets: ["Blake2b"]),
    ],
    targets: [
        .target(
            name: "Blake2b"
        ),
        .testTarget(
            name: "Blake2bTests",
            dependencies: ["Blake2b"]
        ),
    ]
)
