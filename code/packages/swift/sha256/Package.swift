// swift-tools-version: 6.0
// ============================================================================
// Package.swift -- SHA256
// ============================================================================
//
// Swift Package Manager manifest for the SHA256 library.
//
// This package implements the SHA-256 cryptographic hash function (FIPS 180-4)
// from scratch with no external dependencies. It provides both one-shot and
// streaming APIs for computing 256-bit (32-byte) digests.
//
// Part of the coding-adventures educational computing stack.
//
// ============================================================================

import PackageDescription

let package = Package(
    name: "SHA256",
    products: [
        .library(name: "SHA256", targets: ["SHA256"]),
    ],
    targets: [
        .target(name: "SHA256"),
        .testTarget(name: "SHA256Tests", dependencies: ["SHA256"]),
    ]
)
