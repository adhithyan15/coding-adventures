// swift-tools-version: 5.9
import PackageDescription

let package = Package(
  name: "wasm-simulator",
  products: [
    .library(name: "WasmSimulator", targets: ["WasmSimulator"])
  ],
  targets: [
    .target(name: "WasmSimulator"),
    .testTarget(name: "WasmSimulatorTests", dependencies: ["WasmSimulator"]),
  ]
)
