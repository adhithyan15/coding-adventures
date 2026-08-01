// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "wave",
    products: [
        .library(name: "Wave", targets: ["Wave"]),
    ],
    dependencies: [
        .package(path: "../trig"),
    ],
    targets: [
        .target(
            name: "Wave",
            dependencies: [.product(name: "Trig", package: "trig")]),
        .testTarget(name: "WaveTests", dependencies: ["Wave"]),
    ]
)
