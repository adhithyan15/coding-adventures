# Parser (Swift)

A grammar-driven parser that reads grammar rules from a `.grammar` file (via
the GrammarTools package) and interprets them at runtime. The same Swift code
can parse Python, Ruby, or any language -- just swap the `.grammar` file.

## Features

- **Packrat memoization** -- caches parse results for each (rule, position) pair,
  avoiding exponential backtracking
- **Warth left-recursion** -- handles left-recursive grammars using the algorithm
  from Warth et al. (2008)
- **Furthest-failure error reporting** -- when parsing fails, reports what was
  expected at the furthest position reached
- **Pre/post parse hooks** -- transform tokens before parsing and AST after
- **Position tracking** -- every AST node carries start/end line/column
- **AST walking utilities** -- `walkAST`, `findNodes`, `collectTokens`

## How It Fits

```
Source code --> [Lexer] --> Token stream --> [Parser] --> AST
                                                ^
                                                |
                        ParserGrammar from .grammar file
```

## Element Type Support

All standard EBNF elements plus extensions:

| Grammar Syntax | Description |
|---------------|-------------|
| `A B C` | Sequence -- all must match |
| `A \| B` | Alternation -- try each |
| `{ A }` | Zero or more |
| `[ A ]` | Optional |
| `( A )` | Grouping |
| `& A` | Positive lookahead |
| `! A` | Negative lookahead |
| `A +` | One or more |
| `A // B` | Separated repetition |

## Usage

```swift
import Lexer
import GrammarTools
import Parser

// Parse grammar files
let tokenGrammar = try parseTokenGrammar(source: tokensSource)
let parserGrammar = try parseParserGrammar(source: grammarSource)

// Tokenize source code (using lexer)
let tokens: [Token] = ...

// Parse
let parser = GrammarParser(tokens: tokens, grammar: parserGrammar)
let ast = try parser.parse()

// Walk the AST
walkAST(ast) { node in
    print(node.ruleName)
}

// Find specific nodes
let functions = findNodes(in: ast, named: "function_definition")

// Collect all tokens from a subtree
let tokens = collectTokens(from: ast)
```

## Part Of

The [coding-adventures](https://github.com/adhithyan15/coding-adventures) educational computing stack.
