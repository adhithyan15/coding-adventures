// swift-tools-version: 6.0
// ============================================================================
// Package.swift — X25519
// ============================================================================
//
// Swift Package Manager manifest for the X25519 library.
//
// This package implements X25519 Elliptic Curve Diffie-Hellman (RFC 7748)
// from scratch, including all field arithmetic over GF(2^255-19) using
// custom multi-precision integer operations.
//
// No external dependencies — everything is built from UInt64 limbs up.
//
// Part of the coding-adventures educational computing stack.
// ============================================================================

import PackageDescription

let package = Package(
    name: "X25519",
    products: [
        .library(name: "X25519", targets: ["X25519"]),
    ],
    targets: [
        .target(
            name: "X25519"
        ),
        .testTarget(
            name: "X25519Tests",
            dependencies: ["X25519"]
        ),
    ]
)
