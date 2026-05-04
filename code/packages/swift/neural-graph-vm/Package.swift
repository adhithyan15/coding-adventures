// swift-tools-version:6.0
import PackageDescription

let package = Package(
    name: "NeuralGraphVM",
    products: [.library(name: "NeuralGraphVM", targets: ["NeuralGraphVM"])],
    dependencies: [.package(path: "../neural-network")],
    targets: [
        .target(name: "NeuralGraphVM", dependencies: [.product(name: "NeuralNetwork", package: "neural-network")]),
        .testTarget(name: "NeuralGraphVMTests", dependencies: ["NeuralGraphVM", .product(name: "NeuralNetwork", package: "neural-network")]),
    ]
)
