// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "BTree",
    products: [
        .library(name: "BTree", targets: ["BTree"]),
    ],
    targets: [
        .target(name: "BTree"),
        .testTarget(
            name: "BTreeTests",
            dependencies: ["BTree"]
        ),
    ]
)
