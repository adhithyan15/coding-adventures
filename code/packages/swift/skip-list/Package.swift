// swift-tools-version: 6.0

import PackageDescription

let package = Package(
  name: "SkipList",
  products: [
    .library(name: "SkipList", targets: ["SkipList"])
  ],
  targets: [
    .target(name: "SkipList"),
    .testTarget(
      name: "SkipListTests",
      dependencies: ["SkipList"]
    ),
  ]
)
