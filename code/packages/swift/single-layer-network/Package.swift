// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "single-layer-network",
    products: [
        .library(name: "SingleLayerNetwork", targets: ["SingleLayerNetwork"]),
    ],
    targets: [
        .target(name: "SingleLayerNetwork"),
        .testTarget(
            name: "SingleLayerNetworkTests",
            dependencies: ["SingleLayerNetwork"]
        ),
    ]
)
