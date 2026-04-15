// swift-tools-version: 6.0
// ============================================================================
// Package.swift — HKDF
// ============================================================================
//
// Swift Package Manager manifest for the HKDF library.
//
// HKDF (HMAC-based Extract-and-Expand Key Derivation Function, RFC 5869)
// transforms raw input keying material into cryptographically strong keys.
// It uses HMAC as the underlying pseudorandom function, supporting both
// SHA-256 and SHA-512.
//
// Part of the coding-adventures educational computing stack.
//
// ============================================================================

import PackageDescription

let package = Package(
    name: "HKDF",
    products: [
        .library(name: "HKDF", targets: ["HKDF"]),
    ],
    dependencies: [
        .package(path: "../hmac"),
    ],
    targets: [
        .target(
            name: "HKDF",
            dependencies: [
                .product(name: "HMAC", package: "hmac"),
            ]
        ),
        .testTarget(
            name: "HKDFTests",
            dependencies: ["HKDF"]
        ),
    ]
)
