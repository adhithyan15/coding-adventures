// swift-tools-version: 6.0
// ============================================================================
// Package.swift -- SHA512
// ============================================================================
//
// Swift Package Manager manifest for the SHA512 library.
//
// This package implements the SHA-512 cryptographic hash function (FIPS 180-4)
// from scratch with no external dependencies. It provides both one-shot and
// streaming APIs for computing 512-bit (64-byte) digests.
//
// SHA-512 is the 64-bit sibling of SHA-256, using 128-byte blocks, 80 rounds,
// and 8 x 64-bit state words. On 64-bit platforms it is often faster than
// SHA-256.
//
// Part of the coding-adventures educational computing stack.
//
// ============================================================================

import PackageDescription

let package = Package(
    name: "SHA512",
    products: [
        .library(name: "SHA512", targets: ["SHA512"]),
    ],
    targets: [
        .target(name: "SHA512"),
        .testTarget(name: "SHA512Tests", dependencies: ["SHA512"]),
    ]
)
