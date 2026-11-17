// swift-tools-version: 5.9
// ============================================================================
// Package.swift -- CanonicalCbor (RFC 8949 CBR01 Conformance)
// ============================================================================

import PackageDescription

let package = Package(
  name: "CanonicalCbor",
  products: [
    .library(name: "CanonicalCbor", targets: ["CanonicalCbor"])
  ],
  targets: [
    .target(
      name: "CanonicalCbor"
    ),
    .testTarget(
      name: "CanonicalCborTests",
      dependencies: ["CanonicalCbor"]
    ),
  ]
)
