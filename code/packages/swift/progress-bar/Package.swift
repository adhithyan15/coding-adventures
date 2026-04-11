// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "ProgressBar",
    products: [
        .library(name: "ProgressBar", targets: ["ProgressBar"]),
    ],
    targets: [
        .target(name: "ProgressBar"),
        .testTarget(name: "ProgressBarTests", dependencies: ["ProgressBar"]),
    ]
)
