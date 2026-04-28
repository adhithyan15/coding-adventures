// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "two-layer-network",
    products: [
        .library(name: "TwoLayerNetwork", targets: ["TwoLayerNetwork"]),
    ],
    targets: [
        .target(name: "TwoLayerNetwork"),
        .testTarget(
            name: "TwoLayerNetworkTests",
            dependencies: ["TwoLayerNetwork"]
        ),
    ]
)
