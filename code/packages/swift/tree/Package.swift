// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "Tree",
    products: [
        .library(name: "Tree", targets: ["Tree"]),
    ],
    dependencies: [
        .package(path: "../directed-graph"),
    ],
    targets: [
        .target(
            name: "Tree",
            dependencies: [
                .product(name: "DirectedGraph", package: "directed-graph"),
            ]
        ),
        .testTarget(
            name: "TreeTests",
            dependencies: ["Tree", .product(name: "DirectedGraph", package: "directed-graph")]
        ),
    ]
)
