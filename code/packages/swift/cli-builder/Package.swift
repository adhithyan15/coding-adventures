// swift-tools-version: 6.0
import PackageDescription

let package = Package(
  name: "cli-builder",
  products: [
    .library(name: "CliBuilder", targets: ["CliBuilder"])
  ],
  dependencies: [
    .package(path: "../directed-graph"),
    .package(path: "../state-machine"),
  ],
  targets: [
    .target(
      name: "CliBuilder",
      dependencies: [
        .product(name: "DirectedGraph", package: "directed-graph"),
        .product(name: "StateMachine", package: "state-machine"),
      ]
    ),
    .testTarget(name: "CliBuilderTests", dependencies: ["CliBuilder"]),
  ]
)
