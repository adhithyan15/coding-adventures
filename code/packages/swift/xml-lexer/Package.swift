// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "XMLLexer",
    products: [
        .library(name: "XMLLexer", targets: ["XMLLexer"]),
    ],
    dependencies: [
        .package(path: "../lexer"),
        .package(path: "../grammar-tools"),
    ],
    targets: [
        .target(name: "XMLLexer", dependencies: [
            .product(name: "Lexer", package: "lexer"),
            .product(name: "GrammarTools", package: "grammar-tools"),
        ]),
        .testTarget(name: "XMLLexerTests", dependencies: ["XMLLexer"]),
    ]
)
