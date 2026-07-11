// swift-tools-version: 6.0
// ============================================================================
// Package.swift — Sha256Native
// ============================================================================
//
// Native-through-Rust SHA-256. Calls the Rust `coding_adventures_sha256` crate
// via the `sha256-c` static library (compile-time C linkage) — the same shape
// as `swift/gf256-native`.
//
//   rust/sha256-c/         ← Rust crate compiled to libsha256_c.a
//   swift/sha256-native/
//       Sources/CSha256/       ← SPM C target (header + module map only)
//       Sources/Sha256Native/  ← Swift wrapper importing CSha256
//
// BEFORE `swift test` (also done by the BUILD file):
//   cargo build --manifest-path ../../rust/Cargo.toml -p sha256-c --release
//   cp ../../rust/target/release/libsha256_c.a Sources/CSha256/
// ============================================================================

import PackageDescription

let package = Package(
    name: "Sha256Native",
    products: [
        .library(name: "Sha256Native", targets: ["Sha256Native"]),
    ],
    targets: [
        .systemLibrary(name: "CSha256", path: "Sources/CSha256"),
        .target(
            name: "Sha256Native",
            dependencies: ["CSha256"],
            linkerSettings: [
                .unsafeFlags(["-L", "Sources/CSha256", "-l", "sha256_c"])
            ]
        ),
        .testTarget(name: "Sha256NativeTests", dependencies: ["Sha256Native"]),
    ]
)
