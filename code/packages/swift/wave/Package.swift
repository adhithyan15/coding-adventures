// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "wave",
    products: [
        .library(name: "Wave", targets: ["Wave"]),
    ],
    targets: [
        .target(name: "Wave"),
        .testTarget(name: "WaveTests", dependencies: ["Wave"]),
    ]
)
