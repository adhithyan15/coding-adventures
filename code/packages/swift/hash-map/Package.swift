// swift-tools-version: 6.0

import PackageDescription

let package = Package(
  name: "HashMap",
  products: [
    .library(name: "HashMap", targets: ["HashMap"])
  ],
  dependencies: [
    .package(path: "../hash-functions")
  ],
  targets: [
    .target(
      name: "HashMap",
      dependencies: [
        .product(name: "HashFunctions", package: "hash-functions")
      ]
    ),
    .testTarget(
      name: "HashMapTests",
      dependencies: ["HashMap"]
    ),
  ]
)
