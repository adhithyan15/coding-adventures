# JSON Lexer

A Ruby gem that tokenizes JSON text using the grammar-driven lexer engine.

## Overview

This gem is a thin wrapper around `coding_adventures_lexer`'s `GrammarLexer`. Instead of hardcoding JSON-specific tokenization rules, it loads the `json.tokens` grammar file and feeds it to the general-purpose lexer engine.

JSON (JavaScript Object Notation, RFC 8259) is the simplest practical grammar for the grammar-driven infrastructure. It has no keywords, no comments, no identifiers, no indentation significance, and no reserved words. The entire token vocabulary is just 9 types plus whitespace.

This demonstrates the core idea behind grammar-driven language tooling: the same engine can process any language, as long as you provide the right grammar file.

## How It Fits in the Stack

```
json.tokens (grammar file)
       |
       v
grammar_tools (parses .tokens into TokenGrammar)
       |
       v
lexer (GrammarLexer uses TokenGrammar to tokenize)
       |
       v
json_lexer (this gem -- thin wrapper providing JSON API)
```

## Usage

```ruby
require "coding_adventures_json_lexer"

tokens = CodingAdventures::JsonLexer.tokenize('{"key": 42}')
tokens.each { |t| puts t }
# Token(LBRACE, "{", 1:1)
# Token(STRING, "key", 1:2)
# Token(COLON, ":", 1:7)
# Token(NUMBER, "42", 1:9)
# Token(RBRACE, "}", 1:11)
# Token(EOF, "", 1:12)
```

## Key Differences from Starlark/Python Lexers

- **No keywords**: true/false/null are their own token types (TRUE, FALSE, NULL), not reclassified NAME tokens.
- **No indentation mode**: Whitespace is silently skipped. No INDENT, DEDENT, or NEWLINE tokens.
- **No reserved words**: JSON has no concept of reserved identifiers.
- **No comments**: JSON does not support comments of any kind.

## Dependencies

- `coding_adventures_grammar_tools` -- reads the `.tokens` grammar file
- `coding_adventures_lexer` -- the grammar-driven lexer engine

## Development

```bash
bundle install
bundle exec rake test
```
