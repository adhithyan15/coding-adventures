// swift-tools-version: 6.0
// ============================================================================
// Package.swift — Polynomial
// ============================================================================
//
// Swift Package Manager manifest for the Polynomial library.
//
// This package implements polynomial arithmetic over real numbers (Double).
// Polynomials are represented as [Double] where index i is the coefficient
// of x^i (little-endian / ascending-degree order).
//
// Part of the coding-adventures educational computing stack.
//
// ============================================================================

import PackageDescription

let package = Package(
    name: "Polynomial",
    products: [
        .library(name: "Polynomial", targets: ["Polynomial"]),
    ],
    targets: [
        .target(name: "Polynomial"),
        .testTarget(name: "PolynomialTests", dependencies: ["Polynomial"]),
    ]
)
