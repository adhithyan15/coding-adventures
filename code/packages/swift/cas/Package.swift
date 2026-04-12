// swift-tools-version: 6.0
// ============================================================================
// Package.swift — Cas
// ============================================================================
//
// Swift Package Manager manifest for the content-addressable storage library.
//
// This package implements a generic CAS layer: content is stored by its SHA-1
// hash, and the hash doubles as an integrity check on every read.
//
// Dependencies:
//   sha1 — our own SHA-1 implementation (coding-adventures/sha1)
//
// Part of the coding-adventures educational computing stack.
//
// ============================================================================

import PackageDescription

let package = Package(
    name: "cas",
    products: [
        .library(name: "Cas", targets: ["Cas"]),
    ],
    dependencies: [
        .package(path: "../sha1"),
    ],
    targets: [
        .target(
            name: "Cas",
            dependencies: [
                .product(name: "SHA1", package: "sha1"),
            ]
        ),
        .testTarget(
            name: "CasTests",
            dependencies: ["Cas"]
        ),
    ]
)
