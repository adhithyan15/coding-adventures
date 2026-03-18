# coding_adventures_grammar_tools

Reads `.tokens` and `.grammar` files for language tooling. This is the Ruby port of the Python `grammar-tools` package.

## What It Does

This gem parses two kinds of declarative grammar files:

- **`.tokens` files** -- describe the lexical grammar (what tokens look like)
- **`.grammar` files** -- describe the syntactic grammar using EBNF (how tokens combine into statements)

It also cross-validates that grammar references resolve to defined tokens.

## Classes

- `TokenGrammar` -- parses `.tokens` files (regex/literal patterns + keywords section)
- `ParserGrammar` -- parses `.grammar` files (EBNF rules with recursion)
- `CrossValidator` -- checks that grammar references resolve to defined tokens

## Usage

```ruby
require "coding_adventures_grammar_tools"

GT = CodingAdventures::GrammarTools

# Parse a .tokens file
token_grammar = GT.parse_token_grammar(File.read("python.tokens"))
puts token_grammar.definitions.length  # => number of token definitions
puts token_grammar.keywords            # => ["if", "else", ...]

# Parse a .grammar file
parser_grammar = GT.parse_parser_grammar(File.read("python.grammar"))
puts parser_grammar.rules.length       # => number of grammar rules

# Cross-validate
issues = GT.cross_validate(token_grammar, parser_grammar)
puts issues  # => errors/warnings about mismatched references
```

## How It Fits in the Stack

This gem is a foundational piece of the language tools layer. The `lexer` gem uses `TokenGrammar` to drive grammar-based tokenization, and the `parser` gem uses `ParserGrammar` to drive grammar-based parsing.
