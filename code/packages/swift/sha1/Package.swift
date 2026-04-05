// swift-tools-version: 6.0
// ============================================================================
// Package.swift — SHA1
// ============================================================================
//
// Swift Package Manager manifest for the SHA1 library.
//
// This package implements the SHA-1 cryptographic hash function (FIPS 180-4)
// from scratch with no external dependencies. It provides both one-shot and
// streaming APIs for computing 160-bit (20-byte) digests.
//
// SHA-1 is weakened (SHAttered attack, 2017) but remains safe for UUID v5
// and Git object identifiers. For new security applications, use SHA-256.
//
// Part of the coding-adventures educational computing stack.
//
// ============================================================================

import PackageDescription

let package = Package(
    name: "SHA1",
    products: [
        .library(name: "SHA1", targets: ["SHA1"]),
    ],
    targets: [
        .target(name: "SHA1"),
        .testTarget(name: "SHA1Tests", dependencies: ["SHA1"]),
    ]
)
