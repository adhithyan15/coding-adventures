// swift-tools-version: 6.0

import PackageDescription

let package = Package(
  name: "HashSet",
  products: [
    .library(name: "HashSet", targets: ["HashSet"])
  ],
  dependencies: [
    .package(path: "../hash-map")
  ],
  targets: [
    .target(
      name: "HashSet",
      dependencies: [
        .product(name: "HashMap", package: "hash-map")
      ]
    ),
    .testTarget(
      name: "HashSetTests",
      dependencies: ["HashSet"]
    ),
  ]
)
