// swift-tools-version: 6.0
// ============================================================================
// Package.swift -- Ed25519
// ============================================================================
//
// Swift Package Manager manifest for the Ed25519 digital signature library.
//
// This package implements Ed25519 (RFC 8032) from scratch with custom
// multi-precision arithmetic. The only dependency is SHA-512 from the
// same monorepo.
//
// Ed25519 provides:
// - 128-bit security level
// - Deterministic signatures (no random nonce)
// - 32-byte public keys, 64-byte signatures
// - Fast verification
//
// Part of the coding-adventures educational computing stack.
//
// ============================================================================

import PackageDescription

let package = Package(
    name: "Ed25519",
    products: [
        .library(name: "Ed25519", targets: ["Ed25519"]),
    ],
    dependencies: [
        .package(path: "../sha512"),
    ],
    targets: [
        .target(name: "Ed25519", dependencies: [
            .product(name: "SHA512", package: "sha512"),
        ]),
        .testTarget(name: "Ed25519Tests", dependencies: ["Ed25519"]),
    ]
)
