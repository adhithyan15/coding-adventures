# CodingAdventures::CssLexer (Perl)

A grammar-driven CSS3 tokenizer. Reads the shared `css.tokens` grammar file and tokenizes CSS source into a flat list of token hashrefs.

## What it does

Given the input `h1 { color: red; }`, the tokenizer produces:

| # | type      | value   | line | col |
|---|-----------|---------|------|-----|
| 1 | IDENT     | `h1`    | 1    | 1   |
| 2 | LBRACE    | `{`     | 1    | 4   |
| 3 | IDENT     | `color` | 1    | 6   |
| 4 | COLON     | `:`     | 1    | 11  |
| 5 | IDENT     | `red`   | 1    | 13  |
| 6 | SEMICOLON | `;`     | 1    | 16  |
| 7 | RBRACE    | `}`     | 1    | 18  |
| 8 | EOF       |         | 1    | 19  |

Whitespace and `/* ... */` comments are silently consumed.

## CSS tokenization challenges

CSS uses compound tokens — single lexical units from multiple character classes:

| Input       | Wrong (two tokens)           | Correct (one token)      |
|-------------|------------------------------|--------------------------|
| `10px`      | NUMBER(`10`) IDENT(`px`)     | DIMENSION(`10px`)        |
| `50%`       | NUMBER(`50`) PERCENT         | PERCENTAGE(`50%`)        |
| `rgba(`     | IDENT(`rgba`) LPAREN(`(`)    | FUNCTION(`rgba(`)        |
| `url(./x)`  | FUNCTION(`url(`) …           | URL_TOKEN(`url(./x)`)    |
| `::`        | COLON(`:`) COLON(`:`)        | COLON_COLON(`::`)        |
| `--main`    | MINUS MINUS IDENT            | CUSTOM_PROPERTY(`--main`)|

The `css.tokens` grammar uses first-match-wins ordering to get this right. This module preserves that ordering.

## Usage

```perl
use CodingAdventures::CssLexer;

my $tokens = CodingAdventures::CssLexer->tokenize('h1 { color: red; }');
for my $tok (@$tokens) {
    printf "%s  %s  (line %d, col %d)\n",
        $tok->{type}, $tok->{value}, $tok->{line}, $tok->{col};
}
```

## Token types

Each token hashref has four keys: `type`, `value`, `line`, `col`.

Compound tokens: `DIMENSION`, `PERCENTAGE`, `AT_KEYWORD`, `HASH`, `FUNCTION`, `URL_TOKEN`, `CUSTOM_PROPERTY`

Value tokens: `NUMBER`, `STRING`, `IDENT`, `UNICODE_RANGE`, `CDO`, `CDC`

Multi-char operators: `COLON_COLON`, `TILDE_EQUALS`, `PIPE_EQUALS`, `CARET_EQUALS`, `DOLLAR_EQUALS`, `STAR_EQUALS`

Delimiters: `LBRACE`, `RBRACE`, `LPAREN`, `RPAREN`, `LBRACKET`, `RBRACKET`, `SEMICOLON`, `COLON`, `COMMA`, `DOT`, `PLUS`, `GREATER`, `TILDE`, `STAR`, `PIPE`, `BANG`, `SLASH`, `EQUALS`, `AMPERSAND`, `MINUS`

Error tokens: `BAD_STRING`, `BAD_URL`

## Running tests

```bash
prove -l -v t/
```
