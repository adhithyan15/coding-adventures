// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "Heap",
    products: [
        .library(name: "Heap", targets: ["Heap"]),
    ],
    targets: [
        .target(
            name: "Heap",
            path: "Sources/Heap"
        ),
        .testTarget(
            name: "HeapTests",
            dependencies: ["Heap"],
            path: "Tests/HeapTests"
        ),
    ]
)
