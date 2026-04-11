// swift-tools-version: 6.0
// ============================================================================
// Package.swift -- Scrypt
// ============================================================================
//
// Swift Package Manager manifest for the Scrypt library.
//
// Scrypt (RFC 7914) is a memory-hard password hashing function designed by
// Colin Percival. Unlike PBKDF2, scrypt forces an adversary to use large
// amounts of memory when performing brute-force attacks, making GPU/ASIC
// cracking substantially harder.
//
// This package depends on HMAC for PBKDF2 key stretching and SHA256 for the
// raw hash function passed to the internal HMAC implementation (required to
// support empty passwords as used in RFC 7914 test vectors).
//
// Part of the coding-adventures educational computing stack.
//
// ============================================================================

import PackageDescription

let package = Package(
    name: "Scrypt",
    products: [
        .library(name: "Scrypt", targets: ["Scrypt"]),
    ],
    dependencies: [
        .package(path: "../hmac"),
        .package(path: "../sha256"),
    ],
    targets: [
        .target(
            name: "Scrypt",
            dependencies: [
                .product(name: "HMAC",   package: "hmac"),
                .product(name: "SHA256", package: "sha256"),
            ]
        ),
        .testTarget(
            name: "ScryptTests",
            dependencies: ["Scrypt"]
        ),
    ]
)
