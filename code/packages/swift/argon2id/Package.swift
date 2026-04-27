// swift-tools-version: 6.0
// ============================================================================
// Package.swift -- Argon2id (RFC 9106)
// ============================================================================
//
// Swift Package Manager manifest for Argon2id, the hybrid Argon2 variant and
// RFC 9106's recommended password-hashing default. Depends on our sibling
// Blake2b package for the underlying hash.
//
// Part of the coding-adventures educational computing stack.
// ============================================================================

import PackageDescription

let package = Package(
    name: "Argon2id",
    products: [
        .library(name: "Argon2id", targets: ["Argon2id"]),
    ],
    dependencies: [
        .package(path: "../blake2b"),
    ],
    targets: [
        .target(
            name: "Argon2id",
            dependencies: [
                .product(name: "Blake2b", package: "Blake2b"),
            ]
        ),
        .testTarget(
            name: "Argon2idTests",
            dependencies: ["Argon2id"]
        ),
    ]
)
