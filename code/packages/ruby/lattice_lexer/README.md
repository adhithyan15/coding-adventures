# lattice-lexer

Tokenizer for the Lattice CSS superset language. A thin wrapper around the
`coding_adventures_lexer` grammar-driven lexer engine that loads
`lattice.tokens` to break Lattice source text into tokens.

## Where It Fits

```
Lattice source text
        │
  lattice_lexer          ← this package
        │ tokens
  lattice_parser
        │ AST
  lattice_ast_to_css
        │ CSS AST
  lattice_transpiler
        │ CSS text
```

Lattice is a CSS superset inspired by Sass/SCSS. It adds variables
(`$primary: #4a90d9`), mixins (`@mixin`/`@include`), control flow
(`@if`/`@for`/`@each`), and user-defined functions (`@function`/`@return`)
on top of standard CSS3 syntax.

## Usage

```ruby
require "coding_adventures_lattice_lexer"

tokens = CodingAdventures::LatticeLexer.tokenize("$color: red; h1 { color: $color; }")
tokens.each { |t| puts "#{t.type}: #{t.value.inspect}" }
# AT_KEYWORD: "@mixin"
# IDENT: "centered"
# LBRACE: "{"
# ...
# EOF: ""
```

## Token Types

All CSS token types are supported (IDENT, AT_KEYWORD, FUNCTION, STRING,
NUMBER, DIMENSION, PERCENTAGE, HASH, CUSTOM_PROPERTY, etc.) plus five
Lattice-specific tokens:

| Token           | Pattern       | Example        |
|-----------------|---------------|----------------|
| `VARIABLE`      | `$identifier` | `$primary`     |
| `EQUALS_EQUALS` | `==`          | `$a == dark`   |
| `NOT_EQUALS`    | `!=`          | `$a != light`  |
| `GREATER_EQUALS`| `>=`          | `$n >= 10`     |
| `LESS_EQUALS`   | `<=`          | `$n <= 5`      |

Whitespace and comments (`// ...`, `/* ... */`) are skipped automatically.

## Dependencies

- `coding_adventures_grammar_tools` — `.tokens` file parser
- `coding_adventures_lexer` — `GrammarLexer` engine

## Development

```bash
# Run tests
bundle exec rake test
```
