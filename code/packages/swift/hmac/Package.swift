// swift-tools-version: 6.0
// ============================================================================
// Package.swift -- HMAC
// ============================================================================
//
// Swift Package Manager manifest for the HMAC library.
//
// HMAC (Hash-based Message Authentication Code, RFC 2104 / FIPS 198-1) wraps
// any hash function to produce an authentication tag that proves both message
// integrity and authenticity. This package provides HMAC-MD5, HMAC-SHA1,
// HMAC-SHA256, and HMAC-SHA512.
//
// Part of the coding-adventures educational computing stack.
//
// ============================================================================

import PackageDescription

let package = Package(
    name: "HMAC",
    products: [
        .library(name: "HMAC", targets: ["HMAC"]),
    ],
    dependencies: [
        .package(path: "../md5"),
        .package(path: "../sha1"),
        .package(path: "../sha256"),
        .package(path: "../sha512"),
    ],
    targets: [
        .target(
            name: "HMAC",
            dependencies: [
                .product(name: "MD5",    package: "md5"),
                .product(name: "SHA1",   package: "sha1"),
                .product(name: "SHA256", package: "sha256"),
                .product(name: "SHA512", package: "sha512"),
            ]
        ),
        .testTarget(
            name: "HMACTests",
            dependencies: ["HMAC"]
        ),
    ]
)
