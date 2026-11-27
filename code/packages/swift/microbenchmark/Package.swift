// swift-tools-version: 5.9
// ============================================================================
// Package.swift — Paired microbenchmark collection and replay
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
    name: "microbenchmark",
    platforms: [.macOS("10.15.4")],
    products: [
        .library(name: "Microbenchmark", targets: ["Microbenchmark"]),
        .executable(name: "array-microbenchmark", targets: ["ArrayMicrobenchmark"]),
    ],
    dependencies: [
        .package(path: "../rank-statistics"),
    ],
    targets: [
        .target(
            name: "Microbenchmark",
            dependencies: [
                .product(name: "RankStatistics", package: "rank-statistics"),
            ]
        ),
        .executableTarget(
            name: "ArrayMicrobenchmark",
            dependencies: ["Microbenchmark"]
        ),
        .testTarget(
            name: "MicrobenchmarkTests",
            dependencies: [
                "Microbenchmark",
                .product(name: "RankStatistics", package: "rank-statistics"),
            ]
        ),
    ]
)
