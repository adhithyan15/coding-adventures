// swift-tools-version:6.0
import PackageDescription

let package = Package(
    name: "NeuralNetwork",
    products: [.library(name: "NeuralNetwork", targets: ["NeuralNetwork"])],
    targets: [
        .target(name: "NeuralNetwork"),
        .testTarget(name: "NeuralNetworkTests", dependencies: ["NeuralNetwork"]),
    ]
)
