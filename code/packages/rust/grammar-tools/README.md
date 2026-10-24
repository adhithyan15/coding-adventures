# grammar-tools (Rust)

Parser and validator for `.tokens` and `.grammar` files — the bridge between human-readable grammar specifications and machine-usable parse tables.

## What it does

This crate reads two kinds of grammar files:

1. **`.tokens` files** describe the lexical grammar — the "words" of a language (e.g., `NUMBER`, `PLUS`, `IF`).
2. **`.grammar` files** describe the syntactic grammar in EBNF — the "sentences" of a language (e.g., `expression = term { PLUS term }`).

It produces structured Rust types that downstream tools (lexer generators, parser generators, syntax highlighters) can consume.

## Where it fits in the stack

```
                  Layer 7: Grammar Tools (this crate)
                     |
    .tokens files ---+--- .grammar files
         |                      |
    TokenGrammar          ParserGrammar
         |                      |
    lexer generator      parser generator
         |                      |
         +-------> compiler <---+
```

Grammar tools sits at layer 7 of the computing stack — it provides the specification layer that drives code generation for lexers and parsers.

## Usage

```rust
use grammar_tools::token_grammar::parse_token_grammar;
use grammar_tools::parser_grammar::parse_parser_grammar;
use grammar_tools::cross_validator::cross_validate;

// Parse the token grammar
let tokens = parse_token_grammar(r#"
NUMBER = /[0-9]+/
PLUS   = "+"
"#).unwrap();

// Parse the parser grammar (EBNF)
let grammar = parse_parser_grammar(r#"
expression = NUMBER { PLUS NUMBER } ;
"#).unwrap();

// Cross-validate: check that both files are consistent
let issues = cross_validate(&tokens, &grammar);
assert!(issues.is_empty());
```

### Token-adjacency predicates (F11)

A `.grammar` can test the *gap* in front of the next token with zero-width
`@` predicates. They read flags the lexer records on each token, consume
nothing and add nothing to the tree:

```text
return_statement = "return" [ @no_newline_before expression ] SEMICOLON ;  # ECMAScript
command_arg      = @space_before MINUS @no_space_before primary ;          # Ruby `f -1`
```

The six predicates are `@space_before`, `@no_space_before`,
`@newline_before`, `@no_newline_before`, `@space_after_next` and
`@no_space_after_next` (`!@p` equals `@no_p`). They parse to
`GrammarElement::AdjacencyPredicate`, compile to a fully qualified
`AdjacencyPredicate::…` path (so grammars without predicates regenerate
byte-identically), and validation rejects a repetition built only from
zero-width elements. Unknown names and `@asi(...)` (not yet implemented) are
parse errors. See `code/specs/F11-grammar-token-adjacency-predicates.md`.

## Modules

- **`token_grammar`** — Parse `.tokens` files into `TokenGrammar` structs. Includes validation for duplicate names, invalid regex, and naming conventions.
- **`parser_grammar`** — Parse `.grammar` files (EBNF) into `ParserGrammar` structs using a hand-written recursive descent parser. Includes validation for undefined references, duplicates, and unreachable rules.
- **`cross_validator`** — Check that `.tokens` and `.grammar` files reference each other consistently.

## Running tests

```bash
cargo test -p grammar-tools
```
