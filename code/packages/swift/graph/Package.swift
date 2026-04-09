// swift-tools-version: 6.0
// ============================================================================
// Package.swift — Graph
// ============================================================================
//
// Swift Package Manager manifest for the Graph library.
//
// This package implements an undirected graph data structure with basic
// graph operations and neighbor queries. It is a foundational building block
// in the DT00 data structures series.
//
// ============================================================================

import PackageDescription

let package = Package(
    name: "Graph",
    products: [
        .library(name: "Graph", targets: ["Graph"]),
    ],
    targets: [
        .target(name: "Graph"),
        .testTarget(name: "GraphTests", dependencies: ["Graph"]),
    ]
)
