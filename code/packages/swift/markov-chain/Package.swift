// swift-tools-version:5.9
// ============================================================================
// Package.swift — CodingAdventuresMarkovChain
// ============================================================================
//
// Swift Package Manager manifest for the MarkovChain library.
//
// This package implements a general-purpose Markov Chain data structure with
// support for arbitrary-order chains, Laplace/Lidstone smoothing, text
// generation, and stationary distribution computation via power iteration.
//
// It depends on CodingAdventuresDirectedGraph to represent the topology of
// the state-transition graph (which states exist and which transitions are
// possible), keeping probability data separate.
//
// ============================================================================

import PackageDescription

let package = Package(
    name: "CodingAdventuresMarkovChain",
    products: [
        .library(
            name: "CodingAdventuresMarkovChain",
            targets: ["CodingAdventuresMarkovChain"]
        )
    ],
    dependencies: [
        .package(path: "../../swift/directed-graph")
    ],
    targets: [
        .target(
            name: "CodingAdventuresMarkovChain",
            dependencies: [
                .product(name: "DirectedGraph", package: "directed-graph")
            ]
        ),
        .testTarget(
            name: "CodingAdventuresMarkovChainTests",
            dependencies: ["CodingAdventuresMarkovChain"]
        )
    ]
)
