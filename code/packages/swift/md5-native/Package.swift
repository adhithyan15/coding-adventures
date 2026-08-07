// swift-tools-version: 6.0
// Package.swift — Md5Native. Native-through-Rust MD5 via the md5-c static library
// (compile-time C linkage), mirroring swift/sha256-native.
//
// BEFORE `swift test` (also done by the BUILD file):
//   cargo build --manifest-path ../../rust/Cargo.toml -p md5-c --release
//   cp ../../rust/target/release/libmd5_c.a Sources/CMd5/
import PackageDescription

let package = Package(
    name: "Md5Native",
    products: [.library(name: "Md5Native", targets: ["Md5Native"])],
    targets: [
        .systemLibrary(name: "CMd5", path: "Sources/CMd5"),
        .target(
            name: "Md5Native",
            dependencies: ["CMd5"],
            linkerSettings: [.unsafeFlags(["-L", "Sources/CMd5", "-l", "md5_c"])]
        ),
        .testTarget(name: "Md5NativeTests", dependencies: ["Md5Native"]),
    ]
)
