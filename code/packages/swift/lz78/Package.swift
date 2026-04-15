// swift-tools-version: 5.9
// Package.swift — LZ78 lossless compression algorithm (1978) — CMP01

import PackageDescription

let package = Package(
    name: "LZ78",
    products: [
        .library(name: "LZ78", targets: ["LZ78"]),
    ],
    targets: [
        .target(
            name: "LZ78",
            path: "Sources/LZ78"
        ),
        .testTarget(
            name: "LZ78Tests",
            dependencies: ["LZ78"],
            path: "Tests/LZ78Tests"
        ),
    ]
)
