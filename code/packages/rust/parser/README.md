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

#### Token-adjacency predicates (F11)

`@space_before`, `@no_space_before`, `@newline_before`, `@no_newline_before`,
`@space_after_next` and `@no_space_after_next` are zero-width elements that
test the gap in front of the next token, using the lexer's token flags:

```text
statement    = command_call | expression ;
command_call = NAME @space_before MINUS @no_space_before NUMBER ;  # `f -1`
expression   = NAME { MINUS NUMBER } ;                             # `f - 1`, `f-1`
```

They never move the cursor or add children, record no expectation in error
messages, and work under `[ ]`, `{ }`, alternation and `&`/`!`. When newlines
are insignificant, `NEWLINE` tokens in front of the next token count as a line
break. Using them does not make a grammar newline-significant. They read only
tokens at the cursor, so packrat memoization stays exact.

#### Recursion-depth guard (opt-in)

The grammar-driven parser is recursive descent: nested rule references recurse
once per nesting level, so pathological input (e.g. `((((…))))` or `[[[…]]]`)
can overflow the native call stack — an *uncatchable* process abort. The parser
offers a depth guard that turns this into a normal, recoverable
`GrammarParseError` ("input nests deeper than the supported limit"), but it is
**opt-in**: `GrammarParser::new` defaults to *unlimited* depth.

```rust
// Untrusted input on the default stack → dial in the DoS backstop:
let mut parser = GrammarParser::new(tokens, grammar)
    .with_max_depth(DEFAULT_MAX_RULE_DEPTH); // 128
```

The guard is opt-in because a single global cap is unsound: **rule-chain depth
≠ source-nesting depth**. A rich grammar (e.g. Wolfram) spends dozens of
rule-frames per source-nesting level, so a cap low enough to stay below the
native-stack overflow point (~200 frames) would reject legitimate *moderate*
nesting — and it would preempt frontends that already guard themselves on an
enlarged stack (e.g. `python-to-semantic-ir`) and rely on their *lowerer's* own
depth check. `DEFAULT_MAX_RULE_DEPTH` (128) is the recommended value for
JS-shaped grammars, where the expression rule-chain is shallow; closurec opts in
at that value and degrades over-deep input to WHITESPACE_ONLY.

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
