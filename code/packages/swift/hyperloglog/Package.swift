// swift-tools-version: 6.0

import PackageDescription

let package = Package(
  name: "HyperLogLog",
  products: [
    .library(name: "HyperLogLog", targets: ["HyperLogLog"])
  ],
  dependencies: [
    .package(path: "../hash-functions")
  ],
  targets: [
    .target(
      name: "HyperLogLog",
      dependencies: [
        .product(name: "HashFunctions", package: "hash-functions")
      ]
    ),
    .testTarget(name: "HyperLogLogTests", dependencies: ["HyperLogLog"]),
  ]
)
