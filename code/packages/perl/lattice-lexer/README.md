# CodingAdventures::LatticeLexer (Perl)

A Lattice tokenizer that tokenizes Lattice (CSS superset) source text into a flat list of typed token hashrefs. It reads the shared `lattice.tokens` grammar file and uses `CodingAdventures::GrammarTools` to compile the token definitions into Perl regexes.

## What is Lattice?

Lattice is a CSS superset language that adds:
- **Variables**: `$color: #ff0000;`
- **Mixins**: `@mixin` and `@include`
- **Control flow**: `@if`, `@else`, `@for`, `@each`
- **Functions**: `@function` and `@return`
- **Modules**: `@use`
- **Placeholder selectors**: `%button-base` (used with `@extend`)
- **Single-line comments**: `// to end of line`
- **Comparison operators**: `==`, `!=`, `>=`, `<=` (in `@if` conditions)
- **Variable flags**: `!default`, `!global`

Every valid CSS file is valid Lattice.

## What it does

Given the input `$color: #ff0000;`, the tokenizer produces:

| type      | value     | line | col |
|-----------|-----------|------|-----|
| VARIABLE  | `$color`  | 1    | 1   |
| COLON     | `:`       | 1    | 7   |
| HASH      | `#ff0000` | 1    | 9   |
| SEMICOLON | `;`       | 1    | 16  |
| EOF       |           | 1    | 17  |

Whitespace (including newlines), `//` line comments, and `/* block comments */` are silently consumed. Because `lattice.tokens` uses `escapes: none`, STRING values include surrounding quotes with raw escape sequences (CSS escape decoding is a semantic post-parse concern).

## Token types

Lattice-specific: VARIABLE, PLACEHOLDER, EQUALS_EQUALS, NOT_EQUALS, GREATER_EQUALS, LESS_EQUALS, BANG_DEFAULT, BANG_GLOBAL

Numeric (priority DIMENSION > PERCENTAGE > NUMBER): DIMENSION, PERCENTAGE, NUMBER

CSS shared: STRING, HASH, AT_KEYWORD, URL_TOKEN, FUNCTION, CUSTOM_PROPERTY, IDENT, UNICODE_RANGE, CDO, CDC

CSS attribute operators: COLON_COLON, TILDE_EQUALS, PIPE_EQUALS, CARET_EQUALS, DOLLAR_EQUALS, STAR_EQUALS

Delimiters: LBRACE, RBRACE, LPAREN, RPAREN, LBRACKET, RBRACKET, SEMICOLON, COLON, COMMA, DOT, AMPERSAND, BANG, SLASH, EQUALS, PLUS, GREATER, LESS, TILDE, STAR, PIPE, MINUS

## Usage

```perl
use CodingAdventures::LatticeLexer;

my $tokens = CodingAdventures::LatticeLexer->tokenize('$color: #ff0000;');
for my $tok (@$tokens) {
    printf "%s  %s\n", $tok->{type}, $tok->{value};
}
```

## How it fits in the stack

```
lattice.tokens  (code/grammars/)
    ↓  parsed by GrammarTools
TokenGrammar
    ↓  compiled to Perl regexes by
LatticeLexer  ← you are here
    ↓  feeds
lattice_parser  (future)
```

## Dependencies

- `CodingAdventures::GrammarTools` — parses `lattice.tokens`
- `CodingAdventures::Lexer` — provides shared Lexer infrastructure

## Running tests

```bash
prove -l -v t/
```
