// swift-tools-version: 6.0

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
