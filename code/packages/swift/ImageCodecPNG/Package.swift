// swift-tools-version: 5.9

import PackageDescription

let package = Package(
  name: "ImageCodecPNG",
  products: [
    .library(name: "ImageCodecPNG", targets: ["ImageCodecPNG"]),
  ],
  dependencies: [
    .package(path: "../PixelContainer"),
    .package(path: "../zip"),
  ],
  targets: [
    .target(
      name: "ImageCodecPNG",
      dependencies: [
        "PixelContainer",
        .product(name: "Zip", package: "zip"),
      ],
      path: "Sources/ImageCodecPNG"
    ),
    .testTarget(
      name: "ImageCodecPNGTests",
      dependencies: ["ImageCodecPNG", "PixelContainer", .product(name: "Zip", package: "zip")],
      path: "Tests/ImageCodecPNGTests"
    ),
  ]
)
