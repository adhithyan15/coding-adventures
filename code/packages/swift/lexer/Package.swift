// swift-tools-version: 6.0
// The swift-tools-version declares the minimum version of Swift required to build this package.
//
// === Lexer Package ===
//
// A grammar-driven lexer that tokenizes source code into a stream of tokens.
// Instead of hardcoding character-matching logic for each language, this lexer
// reads token definitions from a TokenGrammar (parsed from a .tokens file by
// the grammar-tools package) and uses those definitions to drive tokenization
// at runtime.
//
// This is a Swift port of the TypeScript grammar-driven lexer from the
// coding-adventures project. It supports:
//
// - First-match-wins tokenization from regex/literal patterns
// - Pattern groups with stack-based activation for context-sensitive lexing
// - On-token callbacks with a LexerContext for group transitions
// - Pre/post tokenize hooks for source and token transforms
// - Indentation mode with INDENT/DEDENT emission (Python-like languages)
// - Bracket depth tracking for template literal interpolation
// - Token lookbehind via previousToken()
// - Context-sensitive keyword detection via TOKEN_CONTEXT_KEYWORD flag
// - Newline detection via precededByNewline()
//
// Dependencies:
// - GrammarTools: provides TokenGrammar, TokenDefinition, PatternGroup types.

import PackageDescription

let package = Package(
    name: "Lexer",
    products: [
        .library(
            name: "Lexer",
            targets: ["Lexer"]
        ),
    ],
    dependencies: [
        .package(path: "../grammar-tools"),
    ],
    targets: [
        .target(
            name: "Lexer",
            dependencies: [
                .product(name: "GrammarTools", package: "grammar-tools"),
            ]
        ),
        .testTarget(
            name: "LexerTests",
            dependencies: ["Lexer"]
        ),
    ]
)
