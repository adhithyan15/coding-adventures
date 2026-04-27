// swift-tools-version: 5.9
// ============================================================================
// Package.swift — RFC 1738 URL parser with relative resolution and percent-encoding
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
    name: "url-parser",
    products: [
        .library(name: "UrlParser", targets: ["UrlParser"]),
    ],
    targets: [
        .target(
            name: "UrlParser"
        ),
        .testTarget(
            name: "UrlParserTests",
            dependencies: ["UrlParser"]
        ),
    ]
)
