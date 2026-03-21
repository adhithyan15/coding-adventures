# Starlark Lexer

A Ruby gem that tokenizes Starlark source code using the grammar-driven lexer engine.

## Overview

This gem is a thin wrapper around `coding_adventures_lexer`'s `GrammarLexer`. Instead of hardcoding Starlark-specific tokenization rules, it loads the `starlark.tokens` grammar file and feeds it to the general-purpose lexer engine.

Starlark is a deterministic subset of Python designed for configuration files (Bazel BUILD files, Buck2 TARGETS files, etc.). It removes features that make Python non-deterministic: no while loops, no recursion, no try/except, no classes.

This demonstrates the core idea behind grammar-driven language tooling: the same engine can process any language, as long as you provide the right grammar file.

## How It Fits in the Stack

```
starlark.tokens (grammar file)
       |
       v
grammar_tools (parses .tokens into TokenGrammar)
       |
       v
lexer (GrammarLexer uses TokenGrammar to tokenize)
       |
       v
starlark_lexer (this gem -- thin wrapper providing Starlark API)
```

## Usage

```ruby
require "coding_adventures_starlark_lexer"

tokens = CodingAdventures::StarlarkLexer.tokenize("x = 1 + 2")
tokens.each { |t| puts t }
# Token(NAME, "x", 1:1)
# Token(EQUALS, "=", 1:3)
# Token(NUMBER, "1", 1:5)
# Token(PLUS, "+", 1:7)
# Token(NUMBER, "2", 1:9)
# Token(NEWLINE, "", 1:10)
# Token(EOF, "", 1:10)
```

## Key Differences from Python Lexer

- **Keywords**: Starlark adds `load`, `lambda`, `in`, `not`, `and`, `or` as keywords. It removes `while`, `class`, `import`, `try`, `except`, etc.
- **Reserved words**: Python keywords not in Starlark (e.g., `class`, `import`, `while`) cause a syntax error if used.
- **Indentation mode**: Like Python, Starlark uses significant whitespace with INDENT/DEDENT/NEWLINE tokens.

## Dependencies

- `coding_adventures_grammar_tools` -- reads the `.tokens` grammar file
- `coding_adventures_lexer` -- the grammar-driven lexer engine

## Development

```bash
bundle install
bundle exec rake test
```
