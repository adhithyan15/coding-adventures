// swift-tools-version: 5.9
// ============================================================================
// Package.swift — Shared HTTP message types and helpers for request/response heads and body framing
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
    name: "http-core",
    products: [
        .library(name: "HttpCore", targets: ["HttpCore"]),
    ],
    targets: [
        .target(
            name: "HttpCore"
        ),
        .testTarget(
            name: "HttpCoreTests",
            dependencies: ["HttpCore"]
        ),
    ]
)
