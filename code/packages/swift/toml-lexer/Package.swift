// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "TOMLLexer",
    products: [
        .library(name: "TOMLLexer", targets: ["TOMLLexer"]),
    ],
    dependencies: [
        .package(path: "../lexer"),
        .package(path: "../grammar-tools"),
    ],
    targets: [
        .target(name: "TOMLLexer", dependencies: [
            .product(name: "Lexer", package: "lexer"),
            .product(name: "GrammarTools", package: "grammar-tools"),
        ]),
        .testTarget(name: "TOMLLexerTests", dependencies: ["TOMLLexer"]),
    ]
)
