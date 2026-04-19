// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "note-frequency",
    products: [
        .library(name: "NoteFrequency", targets: ["NoteFrequency"]),
    ],
    targets: [
        .target(name: "NoteFrequency"),
        .testTarget(name: "NoteFrequencyTests", dependencies: ["NoteFrequency"]),
    ]
)
