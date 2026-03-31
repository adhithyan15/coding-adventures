# CodingAdventures::TomlLexer (Perl)

A grammar-driven TOML tokenizer. Reads the shared `toml.tokens` grammar file, compiles the token definitions into Perl regexes, and tokenizes TOML source into a flat list of typed tokens.

## What it does

Given:

```toml
[server]
host = "localhost"
port = 8080
```

Produces token hashrefs:

| type          | value          | line | col |
|---------------|----------------|------|-----|
| LBRACKET      | `[`            | 1    | 1   |
| BARE_KEY      | `server`       | 1    | 2   |
| RBRACKET      | `]`            | 1    | 8   |
| BARE_KEY      | `host`         | 2    | 1   |
| EQUALS        | `=`            | 2    | 6   |
| BASIC_STRING  | `"localhost"`  | 2    | 8   |
| BARE_KEY      | `port`         | 3    | 1   |
| EQUALS        | `=`            | 3    | 6   |
| INTEGER       | `8080`         | 3    | 8   |
| EOF           |                | 4    | 1   |

Horizontal whitespace and comments are consumed silently. The last token is always `EOF`.

## Token types

| Token                | Description                        |
|----------------------|------------------------------------|
| BARE_KEY             | Unquoted key: `my-key`, `_port`    |
| BASIC_STRING         | `"hello"`, `"a\nb"`                |
| LITERAL_STRING       | `'C:\path'`                        |
| ML_BASIC_STRING      | `"""multi\nline"""`                |
| ML_LITERAL_STRING    | `'''multi\nline'''`                |
| INTEGER              | `42`, `-17`, `0xFF`, `0o755`, `0b1010` |
| FLOAT                | `3.14`, `-0.5`, `5e22`, `inf`, `nan` |
| TRUE                 | `true`                             |
| FALSE                | `false`                            |
| OFFSET_DATETIME      | `1979-05-27T07:32:00Z`             |
| LOCAL_DATETIME       | `1979-05-27T07:32:00`              |
| LOCAL_DATE           | `1979-05-27`                       |
| LOCAL_TIME           | `07:32:00`, `07:32:00.999`         |
| EQUALS               | `=`                                |
| DOT                  | `.`                                |
| COMMA                | `,`                                |
| LBRACKET             | `[`                                |
| RBRACKET             | `]`                                |
| LBRACE               | `{`                                |
| RBRACE               | `}`                                |
| EOF                  | End of input                       |

## Usage

```perl
use CodingAdventures::TomlLexer;

my $tokens = CodingAdventures::TomlLexer->tokenize('key = "value"');
for my $tok (@$tokens) {
    printf "%s  %s  (line %d, col %d)\n",
        $tok->{type}, $tok->{value}, $tok->{line}, $tok->{col};
}
```

## How it fits in the stack

```
toml.tokens  (code/grammars/)
    ↓  parsed by CodingAdventures::GrammarTools
TokenGrammar
    ↓  compiled to Perl qr// rules
CodingAdventures::TomlLexer  ← you are here
    ↓  feeds
toml_parser  (future)
```

## TOML-specific notes

**Newlines are significant** — TOML key-value pairs are terminated by newlines. The `toml.tokens` grammar skips only horizontal whitespace (spaces and tabs) and comments. A newline in the source will be passed to the token-matching phase. If no token rule matches it, the lexer raises a `LexerError`.

**Pattern ordering** — More-specific patterns appear before less-specific ones in the grammar: multi-line strings before single-line, datetimes before bare keys, floats before integers, booleans before bare keys.

**Aliases** — `FLOAT_SPECIAL`, `FLOAT_EXP`, `FLOAT_DEC` all emit as `FLOAT`. `HEX_INTEGER`, `OCT_INTEGER`, `BIN_INTEGER` all emit as `INTEGER`.

## Dependencies

- `CodingAdventures::GrammarTools` — parses `toml.tokens`
- `CodingAdventures::Lexer` — general-purpose rule-driven lexer (transitive)

## Running tests

```bash
prove -l -v t/
```
