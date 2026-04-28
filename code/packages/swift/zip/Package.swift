// swift-tools-version: 5.9
// ============================================================================
// Package.swift — ZIP archive format (PKZIP 1989) — CMP09
// ============================================================================
//
// This is the Swift Package Manager manifest for this package.
// It is part of the coding-adventures project, an educational computing stack
// built from logic gates up through interpreters and compilers.
//
import PackageDescription

let package = Package(
    name: "zip",
    products: [
        .library(name: "Zip", targets: ["Zip"]),
    ],
    dependencies: [
        .package(path: "../lzss"),
    ],
    targets: [
        .target(
            name: "Zip",
            dependencies: [
                .product(name: "LZSS", package: "lzss"),
            ],
            path: "Sources/Zip"
        ),
        .testTarget(
            name: "ZipTests",
            dependencies: ["Zip"],
            path: "Tests/ZipTests"
        ),
    ]
)
