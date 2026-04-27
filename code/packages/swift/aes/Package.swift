// swift-tools-version: 6.0
// ============================================================================
// Package.swift — AES
// ============================================================================
//
// Swift Package Manager manifest for the AES library.
//
// This package implements the Advanced Encryption Standard (AES) block cipher
// (FIPS 197) from scratch. It depends on the GF256 package for GF(2^8)
// arithmetic with the AES polynomial 0x11B.
//
// Part of the coding-adventures educational computing stack.
//
// ============================================================================

import PackageDescription

let package = Package(
    name: "AES",
    products: [
        .library(name: "AES", targets: ["AES"]),
    ],
    dependencies: [
        .package(path: "../gf256"),
    ],
    targets: [
        .target(
            name: "AES",
            dependencies: [
                .product(name: "GF256", package: "gf256"),
            ]
        ),
        .testTarget(
            name: "AESTests",
            dependencies: ["AES"]
        ),
    ]
)
