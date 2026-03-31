# CodingAdventures::JsonLexer (Perl)

A grammar-driven JSON tokenizer. Reads the shared `json.tokens` grammar file, compiles the token definitions into Perl regexes, and tokenizes JSON source into a flat list of typed tokens.

## What it does

Given `{"key": 42, "ok": true}`, produces:

| type    | value   | line | col |
|---------|---------|------|-----|
| LBRACE  | `{`     | 1    | 1   |
| STRING  | `"key"` | 1    | 2   |
| COLON   | `:`     | 1    | 7   |
| NUMBER  | `42`    | 1    | 9   |
| COMMA   | `,`     | 1    | 11  |
| STRING  | `"ok"`  | 1    | 13  |
| COLON   | `:`     | 1    | 17  |
| TRUE    | `true`  | 1    | 19  |
| RBRACE  | `}`     | 1    | 23  |
| EOF     |         | 1    | 24  |

Whitespace is consumed silently. The last token is always `EOF`.

## Token types

| Token  | Example |
|--------|---------|
| STRING | `"hello"`, `"a\nb"`, `"\u0041"` |
| NUMBER | `42`, `-1`, `3.14`, `2.5E-3` |
| TRUE   | `true` |
| FALSE  | `false` |
| NULL   | `null` |
| LBRACE | `{` |
| RBRACE | `}` |
| LBRACKET | `[` |
| RBRACKET | `]` |
| COLON  | `:` |
| COMMA  | `,` |
| EOF    | (end of input) |

## Usage

```perl
use CodingAdventures::JsonLexer;

my $tokens = CodingAdventures::JsonLexer->tokenize('{"x": 1}');
for my $tok (@$tokens) {
    printf "%s  %s  (line %d, col %d)\n",
        $tok->{type}, $tok->{value}, $tok->{line}, $tok->{col};
}
```

## How it fits in the stack

```
json.tokens  (code/grammars/)
    ↓  parsed by CodingAdventures::GrammarTools
TokenGrammar
    ↓  compiled to Perl qr// rules
CodingAdventures::JsonLexer  ← you are here
    ↓  feeds
json_parser  (future)
```

## Dependencies

- `CodingAdventures::GrammarTools` — parses `json.tokens`
- `CodingAdventures::Lexer` — general-purpose rule-driven lexer (transitive)

## Running tests

```bash
prove -l -v t/
```
