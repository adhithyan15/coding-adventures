// swift-tools-version: 6.0
// ============================================================================
// Package.swift — PolynomialNative
// ============================================================================
//
// Swift Package Manager manifest for the PolynomialNative library.
//
// This package provides polynomial arithmetic (add, subtract, multiply,
// divide, evaluate, GCD) by calling into the Rust `polynomial-c` static
// library via compile-time C linkage. There is no runtime FFI bridge — Swift
// calls the C symbols directly, just like calling any system C library.
//
// ARCHITECTURE
// ─────────────
//
//   rust/polynomial-c/         ← Rust crate compiled to libpolynomial_c.a
//       include/polynomial_c.h ← C header declaring exported functions
//
//   swift/polynomial-native/
//       Sources/
//           CPolynomial/           ← SPM "C target" (header + module map only)
//               include/
//                   polynomial_c.h    ← copy of the Rust crate's C header
//                   module.modulemap  ← tells SPM "CPolynomial" = this header
//           PolynomialNative/      ← Swift target that imports CPolynomial
//               PolynomialNative.swift
//       Tests/
//           PolynomialNativeTests/ ← Swift tests (require .a to be compiled)
//
// BUILD INSTRUCTIONS (before running `swift test`)
// ─────────────────────────────────────────────────
//
// Step 1: Compile the Rust static library:
//
//     cd code/packages/rust/polynomial-c
//     cargo build --release
//
// Step 2: Copy the compiled library into the CPolynomial sources:
//
//     cp target/release/libpolynomial_c.a \
//        ../../swift/polynomial-native/Sources/CPolynomial/
//
// Step 3: Build or test:
//
//     cd ../../swift/polynomial-native
//     swift build
//     swift test
//
// WHY NOT BINARYTARGET?
// ─────────────────────
// SPM's `.binaryTarget` expects a pre-built `.xcframework` bundle. Using a
// plain `.target` with a manually copied `.a` is simpler for a monorepo where
// the Rust source and Swift consumer live side-by-side. If this package were
// published to the Swift Package Index, a `.binaryTarget` pointing to a
// GitHub release asset would be the right approach.
//
// ============================================================================

import PackageDescription

let package = Package(
    name: "PolynomialNative",
    products: [
        .library(name: "PolynomialNative", targets: ["PolynomialNative"]),
    ],
    targets: [
        // ── CPolynomial ───────────────────────────────────────────────────
        //
        // A "system library" target that wraps the C header. It provides no
        // Swift source — it only gives SPM a module name ("CPolynomial") that
        // Swift code can `import`. The module map in the include directory
        // tells SPM which header file belongs to this module.
        //
        // The actual linkage of libpolynomial_c.a is declared in the Swift
        // target below via `.linkedLibrary`. SPM passes -L and -l flags to
        // the linker when building PolynomialNative.
        .systemLibrary(
            name: "CPolynomial",
            path: "Sources/CPolynomial"
        ),

        // ── PolynomialNative ──────────────────────────────────────────────
        //
        // The Swift library. It depends on CPolynomial for the C header and
        // links against libpolynomial_c.a via linker settings.
        //
        // -L sets the library search path to the CPolynomial sources dir,
        // where the .a was copied in Step 2 above.
        // -l tells the linker to link against libpolynomial_c (the .a file).
        .target(
            name: "PolynomialNative",
            dependencies: ["CPolynomial"],
            linkerSettings: [
                .unsafeFlags([
                    "-L", "Sources/CPolynomial",
                    "-l", "polynomial_c",
                ])
            ]
        ),

        // ── Tests ─────────────────────────────────────────────────────────
        //
        // NOTE: These tests require libpolynomial_c.a to be present in
        // Sources/CPolynomial/ before running. See BUILD instructions above.
        .testTarget(
            name: "PolynomialNativeTests",
            dependencies: ["PolynomialNative"]
        ),
    ]
)
