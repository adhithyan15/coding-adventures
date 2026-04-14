// swift-tools-version: 5.7
import PackageDescription

let package = Package(
    name: "GradientDescent",
    products: [
        .library(
            name: "GradientDescent",
            targets: ["GradientDescent"]),
    ],
    targets: [
        .target(
            name: "GradientDescent",
            dependencies: []),
        .testTarget(
            name: "GradientDescentTests",
            dependencies: ["GradientDescent"]),
    ]
)
