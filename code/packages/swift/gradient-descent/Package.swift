// swift-tools-version: 5.9
import PackageDescription

let package = Package(
  name: "gradient-descent",
  products: [
    .library(name: "GradientDescent", targets: ["GradientDescent"])
  ],
  targets: [
    .target(name: "GradientDescent"),
    .testTarget(
      name: "GradientDescentTests",
      dependencies: ["GradientDescent"]
    ),
  ]
)
