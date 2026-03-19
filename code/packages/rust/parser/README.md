# Parser (Rust)

A Rust crate providing two parser implementations: a hand-written recursive descent parser for a Python subset, and a grammar-driven universal parser that can parse any language from a `.grammar` file specification.

## How it fits in the stack

```
Source code
    |
    v
[Lexer] -----> Token stream
    |
    v
[Parser] ----> Abstract Syntax Tree (AST)
    |
    v
[Compiler / Interpreter / Formatter]
```

The parser sits between the lexer (which produces tokens) and downstream tools (which consume the AST). It depends on:

- **lexer** -- provides `Token`, `TokenType`, and the `Lexer` tokenizer
- **grammar-tools** -- provides `ParserGrammar` and `GrammarElement` for grammar-driven parsing

## Two parsers, two approaches

### 1. Hand-written recursive descent parser (`parser::parser`)

Grammar rules are encoded directly as Rust functions. Each grammar rule becomes a method that calls other methods recursively:

```rust
use lexer::tokenizer::Lexer;
use parser::parser::Parser;
use parser::ast::ASTNode;

let mut lexer = Lexer::new("x = 1 + 2 * 3", None);
let tokens = lexer.tokenize().unwrap();
let mut parser = Parser::new(tokens);
let ast = parser.parse().unwrap();
// ast is: Program([Assignment { target: "x", value: BinaryOp(1 + BinaryOp(2 * 3)) }])
```

Produces typed AST nodes with Rust enum variants: `Number`, `String`, `Name`, `BinaryOp`, `Assignment`, `ExpressionStmt`, `Program`.

### 2. Grammar-driven parser (`parser::grammar_parser`)

Reads grammar rules from a `ParserGrammar` (parsed from a `.grammar` file by the `grammar-tools` crate) and uses backtracking to match tokens against the grammar:

```rust
use lexer::tokenizer::Lexer;
use grammar_tools::parser_grammar::{ParserGrammar, GrammarRule, GrammarElement};
use parser::grammar_parser::GrammarParser;

let mut lexer = Lexer::new("1 + 2", None);
let tokens = lexer.tokenize().unwrap();
let grammar = /* parsed from .grammar file */;
let mut parser = GrammarParser::new(tokens, grammar);
let ast = parser.parse().unwrap();
// ast is: GrammarASTNode { rule_name: "expression", children: [...] }
```

Produces generic AST nodes where each node has a rule name and a list of children (nested nodes or raw tokens).

## Supported language subset

The hand-written parser handles:

- **Expressions**: arithmetic with `+`, `-`, `*`, `/` and parentheses
- **Literals**: numbers, strings, variable names
- **Statements**: assignments (`x = expr`) and expression statements
- **Programs**: sequences of statements separated by newlines
- **Operator precedence**: `*`/`/` bind tighter than `+`/`-`

## Running tests

```bash
cargo test -p parser -- --nocapture
```
