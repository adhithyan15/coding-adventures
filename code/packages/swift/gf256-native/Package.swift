// swift-tools-version: 6.0
// ============================================================================
// Package.swift — GF256Native
// ============================================================================
//
// Swift Package Manager manifest for the GF256Native library.
//
// This package provides GF(2^8) arithmetic (add, subtract, multiply, divide,
// power, inverse) by calling into the Rust `gf256-c` static library via
// compile-time C linkage. Swift calls the C symbols directly — no runtime
// bridge, no boxing.
//
// ARCHITECTURE
// ─────────────
//
//   rust/gf256-c/         ← Rust crate compiled to libgf256_c.a
//       include/gf256_c.h ← C header declaring exported functions
//
//   swift/gf256-native/
//       Sources/
//           CGF256/            ← SPM "C target" (header + module map only)
//               include/
//                   gf256_c.h     ← copy of the Rust crate's C header
//                   module.modulemap
//           GF256Native/       ← Swift target that imports CGF256
//               GF256Native.swift
//       Tests/
//           GF256NativeTests/  ← Swift tests (require .a to be compiled)
//
// BUILD INSTRUCTIONS (before running `swift test`)
// ─────────────────────────────────────────────────
//
// Step 1: Compile the Rust static library:
//
//     cd code/packages/rust/gf256-c
//     cargo build --release
//
// Step 2: Copy the compiled library into the CGF256 sources:
//
//     cp target/release/libgf256_c.a \
//        ../../swift/gf256-native/Sources/CGF256/
//
// Step 3: Build or test:
//
//     cd ../../swift/gf256-native
//     swift build
//     swift test
//
// ============================================================================

import PackageDescription

let package = Package(
    name: "GF256Native",
    products: [
        .library(name: "GF256Native", targets: ["GF256Native"]),
    ],
    targets: [
        // ── CGF256 ────────────────────────────────────────────────────────
        //
        // A "system library" target that wraps the C header. It provides no
        // Swift source — it only gives SPM a module name ("CGF256") that
        // Swift code can `import`. The module map in the include directory
        // tells SPM which header file belongs to this module.
        .systemLibrary(
            name: "CGF256",
            path: "Sources/CGF256"
        ),

        // ── GF256Native ───────────────────────────────────────────────────
        //
        // The Swift library. It depends on CGF256 for the C header and links
        // against libgf256_c.a via linker settings.
        .target(
            name: "GF256Native",
            dependencies: ["CGF256"],
            linkerSettings: [
                .unsafeFlags([
                    "-L", "Sources/CGF256",
                    "-l", "gf256_c",
                ])
            ]
        ),

        // ── Tests ─────────────────────────────────────────────────────────
        //
        // NOTE: These tests require libgf256_c.a to be present in
        // Sources/CGF256/ before running. See BUILD instructions above.
        .testTarget(
            name: "GF256NativeTests",
            dependencies: ["GF256Native"]
        ),
    ]
)
