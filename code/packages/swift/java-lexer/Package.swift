// swift-tools-version: 6.0
import PackageDescription

let package = Package(
  name: "JavaLexer",
  products: [
    .library(
      name: "JavaLexer",
      targets: ["JavaLexer"]
    )
  ],
  dependencies: [
    .package(path: "../grammar-tools"),
    .package(path: "../lexer"),
  ],
  targets: [
    .target(
      name: "JavaLexer",
      dependencies: [
        .product(name: "GrammarTools", package: "grammar-tools"),
        .product(name: "Lexer", package: "lexer"),
      ]
    ),
    .testTarget(
      name: "JavaLexerTests",
      dependencies: ["JavaLexer"]
    ),
  ]
)
