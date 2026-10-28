// swift-tools-version: 6.0

import PackageDescription

let package = Package(
  name: "BloomFilter",
  products: [
    .library(name: "BloomFilter", targets: ["BloomFilter"])
  ],
  dependencies: [
    .package(path: "../hash-functions")
  ],
  targets: [
    .target(
      name: "BloomFilter",
      dependencies: [
        .product(name: "HashFunctions", package: "hash-functions")
      ]
    ),
    .testTarget(
      name: "BloomFilterTests",
      dependencies: ["BloomFilter"]
    ),
  ]
)
