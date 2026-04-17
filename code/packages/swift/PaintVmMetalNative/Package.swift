// swift-tools-version: 5.9
import Foundation
import PackageDescription

let packageDirectory = URL(fileURLWithPath: #filePath).deletingLastPathComponent().path

let package = Package(
    name: "PaintVmMetalNative",
    products: [
        .library(name: "PaintVmMetalNative", targets: ["PaintVmMetalNative"]),
    ],
    dependencies: [
        .package(path: "../PaintInstructions"),
        .package(path: "../PixelContainer"),
    ],
    targets: [
        .systemLibrary(
            name: "CPaintVmMetalNative",
            path: "Sources/CPaintVmMetalNative"
        ),
        .target(
            name: "PaintVmMetalNative",
            dependencies: ["CPaintVmMetalNative", "PaintInstructions", "PixelContainer"],
            path: "Sources/PaintVmMetalNative",
            linkerSettings: [
                .unsafeFlags([
                    "-L", "\(packageDirectory)/Sources/CPaintVmMetalNative",
                    "-l", "paint_vm_metal_c",
                ]),
                .linkedFramework("Metal"),
                .linkedFramework("CoreGraphics"),
                .linkedFramework("CoreText"),
                .linkedFramework("CoreFoundation"),
                .linkedFramework("AppKit"),
                .linkedLibrary("objc"),
            ]
        ),
        .testTarget(
            name: "PaintVmMetalNativeTests",
            dependencies: ["PaintVmMetalNative", "PaintInstructions", "PixelContainer"],
            path: "Tests/PaintVmMetalNativeTests"
        ),
    ]
)
