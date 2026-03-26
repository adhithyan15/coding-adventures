# grammar-tools (Elixir program)

A standalone escript CLI for validating `.tokens` and `.grammar` files. This
program wraps the `CodingAdventures.GrammarTools` library behind a proper
command-line interface built with `CodingAdventures.CliBuilder`.

## What it does

`grammar-tools` checks grammar files for correctness before they reach the
lexer or parser stages. Think of it as a compiler's `-fsyntax-only` flag: it
parses, validates, and reports issues without producing any output other than
a human-readable report.

### Checks performed

**Token grammar (`.tokens` file)**
- Duplicate token names
- Invalid regex patterns (won't compile)
- Non-`UPPER_CASE` naming conventions
- Invalid aliases

**Parser grammar (`.grammar` file)**
- Undefined rule references
- Duplicate rule names
- Non-lowercase rule names

**Cross-validation (both files)**
- Token referenced in grammar but not defined in tokens file
- Token defined in tokens file but never used in grammar

## Usage

```bash
# Validate a token/grammar pair together (most common)
grammar-tools validate css.tokens css.grammar

# Validate just a .tokens file
grammar-tools validate-tokens css.tokens

# Validate just a .grammar file
grammar-tools validate-grammar css.grammar

# Help
grammar-tools --help
```

## Building

Build the escript binary from this directory:

```bash
mix deps.get
mix escript.build
./grammar-tools validate css.tokens css.grammar
```

Or run directly with Mix during development:

```bash
mix deps.get
mix run -e 'GrammarTools.CLI.main(["validate", "css.tokens", "css.grammar"])'
```

## Where this fits

```
.tokens file  ──► grammar-tools validate ──► OK / errors
.grammar file ─┘
                          │
                     (on success)
                          │
                    GrammarLexer  ──►  tokens
                    GrammarParser ──►  AST
```

This program lives in `code/programs/elixir/grammar_tools/`. The library it
wraps is at `code/packages/elixir/grammar_tools/`.

## Exit codes

| Code | Meaning                                      |
|------|----------------------------------------------|
|  0   | All checks passed                            |
|  1   | One or more validation errors found          |
|  2   | Usage error (wrong number of arguments)      |
