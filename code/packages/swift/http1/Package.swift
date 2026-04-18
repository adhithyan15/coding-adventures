// swift-tools-version: 5.9
// ============================================================================
// Package.swift — HTTP/1 request and response head parser with body framing detection
// ============================================================================
//
// This is the Swift Package Manager manifest for this package.
// It is part of the coding-adventures project, an educational computing stack
// built from logic gates up through interpreters and compilers.
//
// Local monorepo dependencies are declared via relative path references so
// that SPM resolves them from the local filesystem.
//
import PackageDescription

let package = Package(
    name: "http1",
    products: [
        .library(name: "Http1", targets: ["Http1"]),
    ],
    dependencies: [
        .package(path: "../http-core"),
    ],
    targets: [
        .target(
            name: "Http1",
            dependencies: [
                .product(name: "HttpCore", package: "http-core"),
            ]
        ),
        .testTarget(
            name: "Http1Tests",
            dependencies: ["Http1"]
        ),
    ]
)
