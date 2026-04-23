// swift-tools-version:5.7
import PackageDescription

let package = Package(
    name: "LZW",
    products: [
        .library(name: "LZW", targets: ["LZW"]),
    ],
    targets: [
        .target(name: "LZW", path: "Sources/LZW"),
        .testTarget(name: "LZWTests", dependencies: ["LZW"], path: "Tests/LZWTests"),
    ]
)
