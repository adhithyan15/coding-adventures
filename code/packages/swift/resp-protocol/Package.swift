// swift-tools-version: 6.0

import PackageDescription

let package = Package(
  name: "RespProtocol",
  products: [
    .library(name: "RespProtocol", targets: ["RespProtocol"])
  ],
  targets: [
    .target(name: "RespProtocol"),
    .testTarget(
      name: "RespProtocolTests",
      dependencies: ["RespProtocol"],
      resources: [.process("Fixtures")]
    ),
  ]
)
