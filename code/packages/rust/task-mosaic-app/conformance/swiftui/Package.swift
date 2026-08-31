// swift-tools-version: 5.10
import PackageDescription

let package = Package(
  name: "TaskAppSwiftRuntimeConformance",
  platforms: [.macOS(.v13)],
  targets: [
    .target(
      name: "CMosaicRuntime",
      path: "Sources/CMosaicRuntime",
      publicHeadersPath: "include",
      linkerSettings: [.linkedLibrary("dl")]
    ),
    .executableTarget(
      name: "Conformance",
      dependencies: ["CMosaicRuntime"],
      path: "Sources/Conformance"
    ),
  ]
)
