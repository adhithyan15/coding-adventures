// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "BPlusTree",
    products: [
        .library(name: "BPlusTree", targets: ["BPlusTree"]),
    ],
    targets: [
        .target(name: "BPlusTree"),
        .testTarget(
            name: "BPlusTreeTests",
            dependencies: ["BPlusTree"]
        ),
    ]
)
