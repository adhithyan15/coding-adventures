// swift-tools-version: 6.0
// ============================================================================
// Package.swift — HashBreaker
// ============================================================================
//
// Demonstrates three attacks on MD5 proving it is cryptographically broken.
// ============================================================================

import PackageDescription

let package = Package(
    name: "HashBreaker",
    products: [
        .executable(name: "HashBreaker", targets: ["HashBreaker"]),
    ],
    dependencies: [
        .package(path: "../../../packages/swift/md5"),
    ],
    targets: [
        .executableTarget(
            name: "HashBreaker",
            dependencies: [
                .product(name: "MD5", package: "md5"),
            ]
        ),
    ]
)
