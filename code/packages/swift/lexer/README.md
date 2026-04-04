# Lexer

A grammar-driven lexer for Swift that tokenizes source code into a stream of tokens using pattern definitions from a `TokenGrammar` (parsed from `.tokens` files by the `grammar-tools` package).

This is a Swift port of the TypeScript grammar-driven lexer from the coding-adventures project.

## Overview

Instead of hardcoding character-matching logic for each language, this lexer reads token definitions at runtime and uses first-match-wins semantics to tokenize any language.

## Features

- **First-match-wins tokenization** from regex and literal patterns
- **Pattern groups** with stack-based activation for context-sensitive lexing (e.g., XML tags)
- **On-token callbacks** with `LexerContext` for group transitions, token emission, suppression
- **Pre/post tokenize hooks** for source text and token list transforms
- **Indentation mode** with INDENT/DEDENT emission for Python-like languages
- **Bracket depth tracking** for template literal interpolation
- **Token lookbehind** via `previousToken()` for context-sensitive decisions
- **Context keywords** with `TOKEN_CONTEXT_KEYWORD` flag for words that are sometimes keywords
- **Newline detection** via `precededByNewline()` for automatic semicolon insertion

## Usage

```swift
import GrammarTools
import Lexer

// Parse a token grammar
let grammar = try parseTokenGrammar(source: grammarText)

// Simple tokenization
let tokens = try grammarTokenize(source: "x = 1 + 2", grammar: grammar)

// Advanced: class-based with callbacks
let lexer = GrammarLexer(source: "<div>hello</div>", grammar: xmlGrammar)
lexer.setOnToken { token, ctx in
    if token.type == "OPEN_TAG" { ctx.pushGroup("tag") }
    if token.type == "TAG_CLOSE" { ctx.popGroup() }
}
let tokens = try lexer.tokenize()
```

## Dependencies

- **GrammarTools** (`../grammar-tools`): provides `TokenGrammar`, `TokenDefinition`, `PatternGroup` types.

## How It Fits in the Stack

The lexer is Layer 2 of the computing stack. It takes raw source code text (Layer 1: characters) and produces a token stream that the parser (Layer 3) can work with. The grammar-tools package provides the grammar definitions that drive the lexer.
