// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "HashFunctions",
    products: [
        .library(name: "HashFunctions", targets: ["HashFunctions"]),
    ],
    targets: [
        .target(name: "HashFunctions"),
        .testTarget(name: "HashFunctionsTests", dependencies: ["HashFunctions"]),
    ]
)
