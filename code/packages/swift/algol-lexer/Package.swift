// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "AlgolLexer",
    products: [
        .library(
            name: "AlgolLexer",
            targets: ["AlgolLexer"]
        ),
    ],
    dependencies: [
        .package(path: "../grammar-tools"),
        .package(path: "../lexer"),
    ],
    targets: [
        .target(
            name: "AlgolLexer",
            dependencies: [
                .product(name: "GrammarTools", package: "grammar-tools"),
                .product(name: "Lexer", package: "lexer"),
            ]
        ),
        .testTarget(
            name: "AlgolLexerTests",
            dependencies: ["AlgolLexer"]
        ),
    ]
)
