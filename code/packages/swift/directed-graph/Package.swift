// swift-tools-version: 6.0
// ============================================================================
// Package.swift — DirectedGraph
// ============================================================================
//
// Swift Package Manager manifest for the DirectedGraph library.
//
// This package implements a directed graph data structure with algorithms for
// topological sorting, cycle detection, transitive closure, and parallel
// execution level computation. It is a foundational building block used by
// the grammar-tools and build system packages.
//
// ============================================================================

import PackageDescription

let package = Package(
    name: "DirectedGraph",
    products: [
        .library(name: "DirectedGraph", targets: ["DirectedGraph"]),
    ],
    targets: [
        .target(name: "DirectedGraph"),
        .testTarget(name: "DirectedGraphTests", dependencies: ["DirectedGraph"]),
    ]
)
