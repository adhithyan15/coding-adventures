// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "XMLLexer",
    products: [
        .library(
            name: "XMLLexer",
            targets: ["XMLLexer"]
        ),
    ],
    dependencies: [
        .package(path: "../grammar-tools"),
        .package(path: "../lexer"),
    ],
    targets: [
        .target(
            name: "XMLLexer",
            dependencies: [
                .product(name: "GrammarTools", package: "grammar-tools"),
                .product(name: "Lexer", package: "lexer"),
            ]
        ),
        .testTarget(
            name: "XMLLexerTests",
            dependencies: ["XMLLexer"]
        ),
    ]
)
