# coding_adventures_lexer

Tokenizer with hand-written and grammar-driven modes. This is the Ruby port of the Python `lexer` package.

## What It Does

Breaks source code into tokens -- the smallest meaningful units of a programming language. Provides two complementary approaches:

1. **Tokenizer** -- hand-written lexer with hardcoded character dispatch
2. **GrammarLexer** -- grammar-driven lexer that reads `.tokens` files via grammar_tools

Both produce identical `Token` objects so downstream tools (parser) don't care which one generated the tokens.

## Usage

```ruby
require "coding_adventures_lexer"

# Hand-written lexer
tokenizer = CodingAdventures::Lexer::Tokenizer.new("x = 1 + 2", keywords: ["if", "else"])
tokens = tokenizer.tokenize

# Grammar-driven lexer
require "coding_adventures_grammar_tools"
grammar = CodingAdventures::GrammarTools.parse_token_grammar(File.read("python.tokens"))
tokens = CodingAdventures::Lexer::GrammarLexer.new("x = 1 + 2", grammar).tokenize
```

## Token Types

NAME, NUMBER, STRING, KEYWORD, PLUS, MINUS, STAR, SLASH, EQUALS, EQUALS_EQUALS, LPAREN, RPAREN, COMMA, COLON, NEWLINE, EOF.

## Dependencies

- `coding_adventures_grammar_tools` (for GrammarLexer)
