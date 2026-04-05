// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "activation-functions",
    products: [
        .library(name: "ActivationFunctions", targets: ["ActivationFunctions"]),
    ],
    targets: [
        .target(name: "ActivationFunctions"),
        .testTarget(
            name: "ActivationFunctionsTests",
            dependencies: ["ActivationFunctions"]
        ),
    ]
)
