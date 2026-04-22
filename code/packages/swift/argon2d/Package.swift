// swift-tools-version: 6.0
// ============================================================================
// Package.swift -- Argon2d (RFC 9106)
// ============================================================================
//
// Swift Package Manager manifest for Argon2d, the data-dependent Argon2
// variant. Depends on our sibling Blake2b package for the underlying hash.
//
// Part of the coding-adventures educational computing stack.
// ============================================================================

import PackageDescription

let package = Package(
    name: "Argon2d",
    products: [
        .library(name: "Argon2d", targets: ["Argon2d"]),
    ],
    dependencies: [
        .package(path: "../blake2b"),
    ],
    targets: [
        .target(
            name: "Argon2d",
            dependencies: [
                .product(name: "Blake2b", package: "Blake2b"),
            ]
        ),
        .testTarget(
            name: "Argon2dTests",
            dependencies: ["Argon2d"]
        ),
    ]
)
