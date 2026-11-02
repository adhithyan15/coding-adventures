// swift-tools-version: 6.0

import PackageDescription

let package = Package(
  name: "InMemoryDataStoreProtocol",
  products: [
    .library(
      name: "InMemoryDataStoreProtocol",
      targets: ["InMemoryDataStoreProtocol"]
    )
  ],
  targets: [
    .target(name: "InMemoryDataStoreProtocol"),
    .testTarget(
      name: "InMemoryDataStoreProtocolTests",
      dependencies: ["InMemoryDataStoreProtocol"]
    ),
  ]
)
