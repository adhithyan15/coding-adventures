// swift-tools-version: 5.9

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
