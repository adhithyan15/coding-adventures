// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "trig",
    products: [
        .library(name: "Trig", targets: ["Trig"]),
    ],
    targets: [
        .target(name: "Trig"),
        .testTarget(
            name: "TrigTests",
            dependencies: ["Trig"]
        ),
    ]
)
