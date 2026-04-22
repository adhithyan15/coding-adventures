// swift-tools-version: 6.0
// ============================================================================
// Package.swift -- Argon2i (RFC 9106)
// ============================================================================
//
// Swift Package Manager manifest for Argon2i, the data-independent Argon2
// variant. Depends on our sibling Blake2b package for the underlying hash.
//
// Part of the coding-adventures educational computing stack.
// ============================================================================

import PackageDescription

let package = Package(
    name: "Argon2i",
    products: [
        .library(name: "Argon2i", targets: ["Argon2i"]),
    ],
    dependencies: [
        .package(path: "../blake2b"),
    ],
    targets: [
        .target(
            name: "Argon2i",
            dependencies: [
                .product(name: "Blake2b", package: "Blake2b"),
            ]
        ),
        .testTarget(
            name: "Argon2iTests",
            dependencies: ["Argon2i"]
        ),
    ]
)
