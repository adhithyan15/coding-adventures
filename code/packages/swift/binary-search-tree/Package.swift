// swift-tools-version: 6.0
// ============================================================================
// Package.swift — BinarySearchTree
// ============================================================================
//
// Swift Package Manager manifest for the BinarySearchTree library: an immutable
// binary search tree with order-statistic queries (kthSmallest, rank) backed by
// cached subtree sizes. Part of the coding-adventures stack.
// ============================================================================

import PackageDescription

let package = Package(
    name: "BinarySearchTree",
    products: [
        .library(name: "BinarySearchTree", targets: ["BinarySearchTree"]),
    ],
    targets: [
        .target(name: "BinarySearchTree"),
        .testTarget(name: "BinarySearchTreeTests", dependencies: ["BinarySearchTree"]),
    ]
)
