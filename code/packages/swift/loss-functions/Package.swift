// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "loss-functions",
    products: [
        .library(name: "LossFunctions", targets: ["LossFunctions"]),
    ],
    targets: [
        .target(name: "LossFunctions"),
        .testTarget(
            name: "LossFunctionsTests",
            dependencies: ["LossFunctions"]
        ),
    ]
)
