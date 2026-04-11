// swift-tools-version: 6.0
// ============================================================================
// Package.swift -- PBKDF2
// ============================================================================
//
// Swift Package Manager manifest for the PBKDF2 library.
//
// PBKDF2 (RFC 8018) derives a cryptographic key from a password by applying
// HMAC many thousands of times. This package provides PBKDF2-HMAC-SHA1,
// PBKDF2-HMAC-SHA256, and PBKDF2-HMAC-SHA512.
//
// Part of the coding-adventures educational computing stack.
//
// ============================================================================

import PackageDescription

let package = Package(
    name: "PBKDF2",
    products: [
        .library(name: "PBKDF2", targets: ["PBKDF2"]),
    ],
    dependencies: [
        .package(path: "../hmac"),
    ],
    targets: [
        .target(
            name: "PBKDF2",
            dependencies: [
                .product(name: "HMAC", package: "hmac"),
            ]
        ),
        .testTarget(
            name: "PBKDF2Tests",
            dependencies: ["PBKDF2"]
        ),
    ]
)
