// swift-tools-version: 6.0
// ============================================================================
// Package.swift — FenwickTree
// ============================================================================
//
// Swift Package Manager manifest for the FenwickTree library: a Binary Indexed
// Tree over Double supporting O(log n) point updates, prefix/range sums, point
// queries, and an order-statistic search. Part of the coding-adventures stack.
// ============================================================================

import PackageDescription

let package = Package(
    name: "FenwickTree",
    products: [
        .library(name: "FenwickTree", targets: ["FenwickTree"]),
    ],
    targets: [
        .target(name: "FenwickTree"),
        .testTarget(name: "FenwickTreeTests", dependencies: ["FenwickTree"]),
    ]
)
