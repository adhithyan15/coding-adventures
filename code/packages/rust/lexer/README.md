# Lexer

A Rust crate providing two lexer implementations: a hand-written Python tokenizer and a grammar-driven universal lexer.

## What is a lexer?

A lexer (also called a tokenizer or scanner) is the first stage of every compiler and interpreter. It takes raw source code as a string and breaks it into a sequence of **tokens** — the smallest meaningful units of the language, like numbers, names, operators, and keywords.

## Two approaches

### 1. Hand-written lexer (`tokenizer` module)

A character-by-character lexer with rules baked into Rust code. It reads one character at a time and uses match/if logic to decide what token to produce. This is the approach used by most production compilers.

```rust
use lexer::tokenizer::{Lexer, LexerConfig};
use lexer::token::TokenType;

let config = LexerConfig {
    keywords: vec!["if".to_string(), "else".to_string()],
};
let mut lexer = Lexer::new("x = 1 + 2", Some(config));
let tokens = lexer.tokenize().unwrap();

assert_eq!(tokens[0].type_, TokenType::Name);
assert_eq!(tokens[0].value, "x");
```

### 2. Grammar-driven lexer (`grammar_lexer` module)

A lexer that reads its rules from a `TokenGrammar` parsed from a `.tokens` file by the `grammar-tools` crate. Instead of hard-coding rules, it compiles grammar patterns into regexes and tries them in priority order.

```rust
use grammar_tools::token_grammar::parse_token_grammar;
use lexer::grammar_lexer::GrammarLexer;
use lexer::token::TokenType;

let grammar = parse_token_grammar(r#"
NAME   = /[a-zA-Z_][a-zA-Z0-9_]*/
NUMBER = /[0-9]+/
PLUS   = "+"
EQUALS = "="
"#).unwrap();

let tokens = GrammarLexer::new("x = 1 + 2", &grammar).tokenize().unwrap();
```

## Token types

The lexer recognizes these token types:

| Category    | Tokens                                          |
|-------------|--------------------------------------------------|
| Values      | NAME, NUMBER, STRING, KEYWORD                    |
| Arithmetic  | PLUS, MINUS, STAR, SLASH                         |
| Assignment  | EQUALS, EQUALS_EQUALS                            |
| Grouping    | LPAREN, RPAREN, LBRACE, RBRACE, LBRACKET, RBRACKET |
| Punctuation | COMMA, COLON, SEMICOLON, DOT, BANG               |
| Structure   | NEWLINE, EOF                                     |

## Dependencies

- `grammar-tools` — provides the `TokenGrammar` type for the grammar-driven lexer
- `regex` — used by the grammar-driven lexer to compile token patterns

## How it fits in the stack

```
.tokens file  -->  grammar-tools  -->  TokenGrammar  -->  grammar_lexer  -->  tokens
source code   -->  tokenizer (hand-written)           -->  tokens
tokens        -->  parser (next layer)                -->  AST
```

## Running tests

```
cargo test -p lexer -- --nocapture
```

## Exact source ranges (CV04a)

`GrammarLexer::tokenize_source_with_ranges(source, &grammar, LexicalLimits::default())`
returns a snapshot borrowing the inspected UTF8 buffer. `tokens()` preserves the
ordinary token stream; `pieces()` partitions raw bytes into accepted token,
skipped trivia and terminal zero-width EOF ranges. Quotes and escapes are
measured before Token value processing. Positions independently report zero-based
byte/scalar/UTF16 offsets and ECMAScript line/scalar/UTF16 columns; existing
one-based Token locations are preserved.

```rust
use grammar_tools::token_grammar::parse_token_grammar;
use lexer::grammar_lexer::GrammarLexer;
use lexer::lexical::{validate_lexical_partition, LexicalLimits};
let grammar = parse_token_grammar("NAME = /[a-z]+/").unwrap();
let snapshot = GrammarLexer::tokenize_source_with_ranges("x", &grammar, LexicalLimits::default()).unwrap();
assert_eq!(snapshot.pieces()[0].range.end.byte_offset, 1);
validate_lexical_partition(snapshot.source(), snapshot.tokens().len(), snapshot.pieces(), LexicalLimits::default()).unwrap();
```

Detached partition validation checks coordinates, coverage and ordinals against
supplied bytes in one forward walk. It does not authenticate content identity or
recognize grammar categories. Capture requires a fresh standard lexer and rejects
callbacks, pre/post hooks, indentation/layout and reuse before their execution.
Standalone handwritten/DFA APIs retain their existing behavior and make no new
range claim. Defaults cap source at 8 MiB, descriptors at 262144/64 MiB and cursor
work at 32 Mi units. Lower limits can be supplied; excessive configuration rejects.
This API adds no raw-source copies, required Token fields or dependencies.
