// swift-tools-version: 6.0
// ============================================================================
// Package.swift — DES
// ============================================================================
//
// Swift Package Manager manifest for the DES library.
//
// This package implements the Data Encryption Standard (DES) block cipher
// (FIPS 46-3) from scratch with no external dependencies. It provides
// single-block DES encryption/decryption, ECB mode with PKCS#7 padding,
// and triple-DES (3DES/TDEA) EDE operation.
//
// Part of the coding-adventures educational computing stack.
//
// ============================================================================

import PackageDescription

let package = Package(
    name: "DES",
    products: [
        .library(name: "DES", targets: ["DES"]),
    ],
    targets: [
        .target(name: "DES"),
        .testTarget(name: "DESTests", dependencies: ["DES"]),
    ]
)
