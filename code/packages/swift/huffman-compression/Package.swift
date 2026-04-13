// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "huffman-compression",
    products: [
        .library(name: "HuffmanCompression", targets: ["HuffmanCompression"]),
    ],
    dependencies: [
        .package(path: "../huffman-tree"),
    ],
    targets: [
        .target(
            name: "HuffmanCompression",
            dependencies: [
                .product(name: "HuffmanTree", package: "huffman-tree"),
            ],
            path: "Sources/HuffmanCompression"
        ),
        .testTarget(
            name: "HuffmanCompressionTests",
            dependencies: ["HuffmanCompression"],
            path: "Tests/HuffmanCompressionTests"
        ),
    ]
)
