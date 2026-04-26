// swift-tools-version: 5.9
// ============================================================================
// Package.swift — ZStd compression (RFC 8878) — CMP07
// ============================================================================
//
// This is the Swift Package Manager manifest for the Zstd package.
// It depends on LZSS for LZ77 match-finding (the back-reference engine that
// ZStd's FSE codec encodes so efficiently).
//
import PackageDescription

let package = Package(
    name: "zstd",
    products: [
        .library(name: "Zstd", targets: ["Zstd"]),
    ],
    dependencies: [
        .package(path: "../lzss"),
    ],
    targets: [
        .target(
            name: "Zstd",
            dependencies: [
                .product(name: "LZSS", package: "lzss"),
            ],
            path: "Sources/Zstd"
        ),
        .testTarget(
            name: "ZstdTests",
            dependencies: ["Zstd"],
            path: "Tests/ZstdTests"
        ),
    ]
)
