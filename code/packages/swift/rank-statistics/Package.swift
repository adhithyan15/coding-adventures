// swift-tools-version: 5.9
// ============================================================================
// Package.swift — Exact rank statistics for finite binary64 samples
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
    name: "rank-statistics",
    products: [
        .library(name: "RankStatistics", targets: ["RankStatistics"]),
    ],
    targets: [
        .target(
            name: "RankStatistics"
        ),
        .testTarget(
            name: "RankStatisticsTests",
            dependencies: ["RankStatistics"]
        ),
    ]
)
