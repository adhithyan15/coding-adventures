// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "TOMLLexer",
    products: [
        .library(
            name: "TOMLLexer",
            targets: ["TOMLLexer"]
        ),
    ],
    dependencies: [
        .package(path: "../grammar-tools"),
        .package(path: "../lexer"),
    ],
    targets: [
        .target(
            name: "TOMLLexer",
            dependencies: [
                .product(name: "GrammarTools", package: "grammar-tools"),
                .product(name: "Lexer", package: "lexer"),
            ]
        ),
        .testTarget(
            name: "TOMLLexerTests",
            dependencies: ["TOMLLexer"]
        ),
    ]
)
