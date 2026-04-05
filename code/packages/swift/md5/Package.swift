// swift-tools-version: 6.0
// ============================================================================
// Package.swift — MD5
// ============================================================================
//
// Swift Package Manager manifest for the MD5 library.
//
// This package implements the MD5 message digest algorithm (RFC 1321) from
// scratch with no external dependencies. It provides both one-shot and
// streaming APIs for computing 128-bit (16-byte) digests.
//
// MD5 is cryptographically broken (collision attacks since 2004) and should
// NOT be used for security. It remains valid for checksums, UUID v3, and
// legacy compatibility.
//
// Part of the coding-adventures educational computing stack.
//
// ============================================================================

import PackageDescription

let package = Package(
    name: "MD5",
    products: [
        .library(name: "MD5", targets: ["MD5"]),
    ],
    targets: [
        .target(name: "MD5"),
        .testTarget(name: "MD5Tests", dependencies: ["MD5"]),
    ]
)
