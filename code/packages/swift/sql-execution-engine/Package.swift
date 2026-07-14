// swift-tools-version: 6.0
import PackageDescription

let package = Package(
  name: "sql-execution-engine",
  products: [
    .library(name: "SqlExecutionEngine", targets: ["SqlExecutionEngine"])
  ],
  targets: [
    .target(name: "SqlExecutionEngine"),
    .testTarget(
      name: "SqlExecutionEngineTests",
      dependencies: ["SqlExecutionEngine"]
    ),
  ]
)
