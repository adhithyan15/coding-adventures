// swift-tools-version: 6.0

import PackageDescription

let package = Package(
  name: "BinaryTree",
  products: [
    .library(name: "BinaryTree", targets: ["BinaryTree"])
  ],
  targets: [
    .target(name: "BinaryTree"),
    .testTarget(
      name: "BinaryTreeTests",
      dependencies: ["BinaryTree"]
    ),
  ]
)
