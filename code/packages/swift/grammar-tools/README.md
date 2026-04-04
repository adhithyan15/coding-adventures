# GrammarTools (Swift)

Parser and validator for `.tokens` and `.grammar` files -- the declarative
specifications that describe a programming language's lexical and syntactic
structure.

## What It Does

This package provides three capabilities:

1. **Token grammar parsing** (`parseTokenGrammar`) -- reads a `.tokens` file and
   produces a `TokenGrammar` struct containing all token definitions, keywords,
   context-sensitive keywords, skip patterns, and pattern groups.

2. **Parser grammar parsing** (`parseParserGrammar`) -- reads a `.grammar` file
   (EBNF notation) and produces a `ParserGrammar` struct containing grammar rules
   as a tree of `GrammarElement` nodes.

3. **Cross-validation** (`crossValidate`) -- checks that a `.tokens` file and a
   `.grammar` file are consistent with each other.

## How It Fits

```
.tokens file  -->  [GrammarTools]  -->  TokenGrammar  -->  Lexer
.grammar file -->  [GrammarTools]  -->  ParserGrammar -->  Parser
                   [CrossValidator] checks consistency between the two
```

## Grammar Element Types

The `GrammarElement` enum supports standard EBNF plus extensions:

| Syntax | Element | Description |
|--------|---------|-------------|
| `name` | `.ruleReference` | Reference to another rule |
| `NAME` | `.tokenReference` | Reference to a token type |
| `"lit"` | `.literal` | Literal string match |
| `A B` | `.sequence` | Ordered sequence |
| `A \| B` | `.alternation` | Choice between alternatives |
| `{ A }` | `.repetition` | Zero or more |
| `[ A ]` | `.optional` | Zero or one |
| `( A )` | `.group` | Grouping |
| `& A` | `.positiveLookahead` | Match without consuming |
| `! A` | `.negativeLookahead` | Fail if matches |
| `A +` | `.oneOrMore` | One or more |
| `A // B` | `.separatedRepetition` | One+ separated by B |

## Usage

```swift
import GrammarTools

// Parse a .tokens file
let tokenGrammar = try parseTokenGrammar(source: tokensFileContent)

// Parse a .grammar file
let parserGrammar = try parseParserGrammar(source: grammarFileContent)

// Cross-validate
let issues = crossValidate(tokenGrammar: tokenGrammar, parserGrammar: parserGrammar)
```

## Part Of

The [coding-adventures](https://github.com/adhithyan15/coding-adventures) educational computing stack.
