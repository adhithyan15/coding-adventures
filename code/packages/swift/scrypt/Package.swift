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
// This package depends on the PBKDF2 package (which itself depends on HMAC)
// for the two PBKDF2-HMAC-SHA256 calls that wrap and unwrap the ROMix step.
// The allowEmptyPassword flag on pbkdf2HmacSHA256 is used here to support
// RFC 7914 test vector 1 (empty password) without weakening the PBKDF2 API.
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
        .package(path: "../pbkdf2"),
    ],
    targets: [
        .target(
            name: "Scrypt",
            dependencies: [
                .product(name: "PBKDF2", package: "pbkdf2"),
            ]
        ),
        .testTarget(
            name: "ScryptTests",
            dependencies: ["Scrypt"]
        ),
    ]
)
