// swift-tools-version: 5.9
import Foundation
import PackageDescription

let packageDirectory = URL(fileURLWithPath: #filePath).deletingLastPathComponent().path

let package = Package(
    name: "PaintVmDirect2DNative",
    products: [
        .library(name: "PaintVmDirect2DNative", targets: ["PaintVmDirect2DNative"]),
    ],
    dependencies: [
        .package(path: "../PaintInstructions"),
        .package(path: "../PixelContainer"),
    ],
    targets: [
        .systemLibrary(
            name: "CPaintVmDirect2DNative",
            path: "Sources/CPaintVmDirect2DNative"
        ),
        .target(
            name: "PaintVmDirect2DNative",
            dependencies: [
                "PaintInstructions",
                "PixelContainer",
                .target(
                    name: "CPaintVmDirect2DNative",
                    condition: .when(platforms: [.windows])
                ),
            ],
            path: "Sources/PaintVmDirect2DNative",
            linkerSettings: [
                .unsafeFlags([
                    "-L", "\(packageDirectory)/Sources/CPaintVmDirect2DNative",
                    "-l", "paint_vm_direct2d_c",
                ], .when(platforms: [.windows])),
            ]
        ),
        .testTarget(
            name: "PaintVmDirect2DNativeTests",
            dependencies: ["PaintVmDirect2DNative", "PaintInstructions", "PixelContainer"],
            path: "Tests/PaintVmDirect2DNativeTests"
        ),
    ]
)
