# TOML Lexer (Ruby)

Tokenizes TOML v1.0.0 text using the grammar-driven lexer engine.

## Usage

```ruby
require "coding_adventures_toml_lexer"

tokens = CodingAdventures::TomlLexer.tokenize('name = "TOML"')
tokens.each { |t| puts "#{t.type}: #{t.value}" }
# BARE_KEY: name
# EQUALS: =
# BASIC_STRING: "TOML"
# EOF:
```

## Token Types

TOML produces 20 token types across 7 categories: strings (4), numbers (2),
booleans (2), date/times (4), keys (1), delimiters (7), plus NEWLINE and EOF.

## Dependencies

- `coding_adventures_lexer` — grammar-driven lexer engine
- `coding_adventures_grammar_tools` — parses `.tokens` files
- `coding_adventures_state_machine` — DFA engine
- `coding_adventures_directed_graph` — graph library
