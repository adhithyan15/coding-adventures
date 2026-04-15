// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "AlgolParser",
    products: [
        .library(
            name: "AlgolParser",
            targets: ["AlgolParser"]
        ),
    ],
    dependencies: [
        .package(path: "../grammar-tools"),
        .package(path: "../lexer"),
        .package(path: "../parser"),
        .package(path: "../algol-lexer"),
    ],
    targets: [
        .target(
            name: "AlgolParser",
            dependencies: [
                .product(name: "GrammarTools", package: "grammar-tools"),
                .product(name: "Lexer", package: "lexer"),
                .product(name: "Parser", package: "parser"),
                .product(name: "AlgolLexer", package: "algol-lexer"),
            ]
        ),
        .testTarget(
            name: "AlgolParserTests",
            dependencies: [
                "AlgolParser",
                .product(name: "Parser", package: "parser"),
            ]
        ),
    ]
)
